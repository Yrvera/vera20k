# FactoryClass Global Array Insertion / Removal / Rebuild Order - Reswarm 2026-05-28

**Address(es):** `FactoryClass::Constructor @ 0x004C98B0`, `FactoryClass::Destructor/Release @ 0x004CA770`, `LogicClass::PerTickUpdate @ 0x0055AFB0`, `HouseClass::Begin_Production @ 0x004FA350`, `BuildingClass::Update helper @ 0x004500F0`, `FactoryClass::Load @ 0x004CA270`, `FactoryClass::Save @ 0x004CA3C0`, persistence class-factory thunk around `0x006C523C..0x006C526A`
**Investigation Mode:** coverage-map
**Claimed Scope:** global `FactoryClass` dynamic-vector append/remove semantics, the PerTick tail loop order/count behavior, direct runtime factory creation/destruction owners found from constructor xrefs, and bounded save/load evidence for FactoryClass object creation and inner queue serialization.
**Non-Scope:** production step math, sidebar cameo ordering, final unit exit placement, AI build-choice formulas, the full OLE save stream object enumerator, replay restore, and broad `HouseClass::Update` internals.
**Confidence:** High for runtime insertion/removal and PerTick ordering; Medium for persistence construction mechanics; Low for top-level save stream global factory order because the save/load object enumerator was not drained.
**Active in YR:** Yes for runtime production and PerTick factory array behavior. Conditional for save/load paths, active only during savegame persistence restore.
**Status:** PARTIAL. Runtime global array ordering is complete for this slice; top-level save/load stream ordering remains a bounded uncertainty.

## 0. Working Notes

Target question: Determine `FactoryClass` global array insertion/removal/rebuild ordering for same-frame factory completion and save/load/session reconstruction.

Non-goals: Do not re-investigate production formulas, sidebar/cameo animation, object spawn exit details, AI queue choice, or the already-verified broad PerTick ladder.

Evidence needed to mark COMPLETE: decompile plus disassembly/xrefs proving the global array loop, constructor insertion, destructor/removal, and any save/load or rebuild owner path that reconstructs `FactoryClass` order, plus current Rust surface scan.

Stop conditions: all material open questions resolved or explicitly deferred, zero-add pass over primary paths, and report written only to the allowed path.

## 1. Overview

`FactoryClass` instances are not ticked per house or per category. They are appended into `g_FactoryClass_Array` (`0x00A83E34` pointer, `0x00A83E40` count) when constructed, compacted left when released, and ticked by `LogicClass::PerTickUpdate` in ascending array index after Tactical and before the global House loop.

For simultaneous completions, native completion order is therefore global factory-array order. In normal runtime production this order is first-creation order of the `FactoryClass` objects, not owner-name order, not `HouseClass` array order, and not Rust's nested owner/category map order.

## 2. Class Layout / Key Offsets

| Field / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `0x00A83E34` | `g_FactoryClass_Array` pointer storage | constructor writes `*(array + old_count*4)=this`; PerTick reads it | Yes |
| `0x00A83E38` | capacity | constructor capacity compare/grow gate | Yes |
| `0x00A83E3D` | vector initialized/allocated flag used by grow gate | constructor grow gate | Yes |
| `0x00A83E40` | `g_FactoryClass_Array_Count` | constructor increments, destructor decrements, PerTick reloads | Yes |
| `0x00A83E44` | growth increment | constructor grow gate adds growth to capacity | Yes |
| `Factory+0x40..0x54` | inner queued-object dynamic vector | Factory save/load preserves count and per-slot order | Yes |
| `Factory+0x6C` | owner `HouseClass*` | prior layout and `StartProduction` writes; saved via stream body/Factory state | Yes |

## 3. Core Logic

### 3.1 Runtime append is tail insertion at current count

`FactoryClass::Constructor @ 0x004C98B0` initializes fields and vtables, assigns an `AbstractClass` unique ID, then appends to the global factory vector. The decompile shows:

```text
if capacity permits or grow succeeds:
    old = g_FactoryClass_Array_Count
    g_FactoryClass_Array_Count = old + 1
    g_FactoryClass_Array[old] = this
```

Assembly context pins the exact index operation:

```text
004c9974: MOV ECX,dword ptr [0x00a83e40]
004c997a: MOV EAX,ECX
004c997c: INC ECX
004c997d: MOV dword ptr [0x00a83e40],ECX
004c9983: MOV ECX,dword ptr [0x00a83e34]
004c9989: MOV dword ptr [ECX + EAX*0x4],ESI
```

Active in YR: Yes. `HouseClass::Begin_Production @ 0x004FA350` calls this constructor when a house has no existing factory pointer for the requested production category; `BuildingClass::Update` helper `0x004500F0` also calls it for a conditional building-owned production path.

Tiny detail: the constructor also appends the same pointer to a second vector at `0x00B0F724/0x00B0F730` after resetting timer fields. That second vector is not the PerTick factory loop; do not use it for factory AI order.

### 3.2 Runtime removal is compacting erase, not tombstone/null

`FactoryClass::Destructor/Release @ 0x004CA770` first detaches notices, then finds this factory in the global vector by calling the vector vtable `+0x10`. If found, it decrements count and shifts successors one slot left:

```text
idx = find(this)
if idx != -1 and idx < count:
    count -= 1
    while idx < count:
        array[idx] = array[idx + 1]
        idx += 1
```

Assembly context:

```text
004ca7b2: MOV ECX,dword ptr [0x00a83e40]
004ca7b8: CMP EAX,ECX
004ca7ba: JGE 0x004ca7df
004ca7bc: DEC ECX
004ca7bf: MOV dword ptr [0x00a83e40],ECX
004ca7c7: MOV ECX,dword ptr [0x00a83e34]
004ca7cd: INC EAX
004ca7ce: MOV EDX,dword ptr [ECX + EAX*0x4]
004ca7d1: MOV dword ptr [ECX + EAX*0x4 + -0x4],EDX
004ca7d5: MOV ECX,dword ptr [0x00a83e40]
004ca7db: CMP EAX,ECX
004ca7dd: JL 0x004ca7c7
```

Active in YR: Yes for factory release/destruction. If `g_GameActive` is nonzero, the destructor calls `FactoryClass::AbandonProduction` after removing the factory from the global vector.

Tiny detail: the PerTick factory loop has no null guard. Because removal compacts rather than tombstones, future Rust must not leave empty slots in a native-mode factory list and then skip them.

### 3.3 PerTick loops factories in ascending global index with live count reload

`LogicClass::PerTickUpdate @ 0x0055AFB0` runs Tactical first, then the factory loop, then the house loop. For factories:

```text
0055b66a: MOV EAX,[0x00a83e40]
0055b66f: XOR ESI,ESI
0055b671: TEST EAX,EAX
0055b673: JLE 0x0055b68d
0055b675: MOV ECX,dword ptr [0x00a83e34]
0055b67b: MOV ECX,dword ptr [ECX + ESI*0x4]
0055b67e: MOV EDX,dword ptr [ECX]
0055b680: CALL dword ptr [EDX + 0x5c]
0055b683: MOV EAX,[0x00a83e40]
0055b688: INC ESI
0055b689: CMP ESI,EAX
0055b68b: JL 0x0055b675
```

Active in YR: Yes. This is the standard active game PerTick tail documented by sibling reports. The factory loop does not snapshot count once at loop start; it reloads `g_FactoryClass_Array_Count` after each factory AI call.

Consequences:

- Two factories that complete on the same frame complete in ascending `g_FactoryClass_Array` index order.
- A factory appended before the loop reaches the new tail can be visited in the same PerTick pass because count is reloaded.
- A compacting removal at or before the current index can make the shifted successor sit at an index the loop has already passed. `FactoryClass::AI` itself was not found releasing itself, so this is a container rule, not a common production-completion path.

### 3.4 Runtime creation owners found from constructor xrefs

Direct code xrefs to `FactoryClass::Constructor`:

| Caller | Role | Ordering effect | Evidence | Active in YR |
|---|---|---|---|---|
| `HouseClass::Begin_Production @ 0x004FA350` | creates a per-house production-category factory when the relevant `HouseClass` factory pointer is null | append occurs before the house pointer field is assigned to the new factory | decompile plus assembly `0x004FA4E5..0x004FA4FA` | Yes |
| `BuildingClass::Update` helper `0x004500F0` | creates a building-owned factory pointer at building field `+0x524` for a conditional building/type path | append occurs when that building update path first needs its factory | decompile plus xref from `BuildingClass::Update` | Conditional |
| COM/persistence class-factory thunk around `0x006C523C..0x006C526A` | allocates `0x74`, calls `FactoryClass::Constructor`, then performs interface/query setup | restore-time append follows object creation stream order | assembly context at `0x006C523C..0x006C526A` | Conditional, save/load |

Tiny detail: in `HouseClass::Begin_Production`, the constructor append occurs before the new pointer is stored into `HouseClass` category fields. Therefore the array's first-creation order is controlled by successful factory object allocation timing, not by the later house field assignment.

### 3.5 Save/load evidence is bounded

Factory IPersist vtable entries from `vtable_FactoryClass @ 0x007E88D0` point to:

- `Load @ 0x004CA270`
- `Save @ 0x004CA3C0`

`FactoryClass::Save @ 0x004CA3C0` calls `AbstractClass::Save`, writes `Factory+0x50` queue count, then writes each queued-object pointer slot from `Factory+0x44 + index*4` in ascending index order.

`FactoryClass::Load @ 0x004CA270` calls `AbstractClass::Load`, reinstalls FactoryClass vtables and the inner queued-object dynamic vector vtable, reads a saved queue count, appends zero placeholders in ascending index order, then registers each queue slot plus `Factory+0x6C` and `Factory+0x58` pointer fields with `FUN_006CF240`.

Assembly context for load queue order:

```text
004ca2f2: CALL dword ptr [EDX + 0xc]      ; read saved queue count
004ca305: CMP EAX,EBX
004ca307: JLE 0x004ca374
...
004ca360: MOV dword ptr [EDX + EAX*0x4],ECX
...
004ca37a: MOV EDX,dword ptr [EDI + 0x44]
004ca37d: LEA EAX,[EDX + ESI*0x4]
004ca386: CALL 0x006cf240                 ; swizzle-register queue slot
004ca38b: INC ESI
004ca38c: CMP ESI,dword ptr [ESP + 0x18]
004ca390: JL 0x004ca37a
```

Assembly context for save queue order:

```text
004ca3db: MOV ECX,dword ptr [EDI + 0x50]  ; queue count
004ca3ee: CALL dword ptr [EDX + 0x10]     ; write count
004ca3ff: MOV EDX,dword ptr [EDI + 0x44]
004ca408: LEA EAX,[EDX + EBX*0x4]
004ca40d: CALL dword ptr [ECX + 0x10]     ; write slot
004ca418: INC EBX
004ca419: CMP EBX,EAX
004ca41b: JL 0x004ca3ff
```

Active in YR: Conditional through savegame persistence. This proves inner factory queue reconstruction order, not the top-level order in which all factories are saved/loaded.

The persistence class-factory thunk allocates `0x74` and calls `FactoryClass::Constructor`:

```text
006c523c: PUSH 0x74
006c523e: CALL 0x007c8e17
006c5246: TEST EAX,EAX
006c524a: MOV ECX,EAX
006c524c: CALL 0x004c98b0
006c5251: MOV ESI,EAX
006c5261: MOV ECX,dword ptr [ESP + 0x14]
006c5265: MOV EAX,dword ptr [ESI]
006c5267: PUSH EDI
006c5268: PUSH ECX
006c5269: PUSH ESI
006c526a: CALL dword ptr [EAX]
```

This means loaded FactoryClass objects append to `g_FactoryClass_Array` during persistence object creation. What remains unproven in this slot is the top-level save stream enumerator's factory object order. Until that owner is drained, the safest statement is: restored global factory order follows the persistence object creation stream order, and the stream order is not yet proven to be `g_FactoryClass_Array` order.

### 3.6 Session teardown destroys factories from slot 0 repeatedly

`FUN_00534450` is a global clear/teardown owner. Its factory section loops while `g_FactoryClass_Array_Count != 0`, loads `*g_FactoryClass_Array`, calls vtable `+0x20`, then tests the count again. Because factory release compacts left, this destroys factories in current array order from index 0 repeatedly.

Assembly context:

```text
00534627: CMP dword ptr [0x00a83e40],EBX
0053462d: JZ 0x00534649
0053462f: MOV EAX,[0x00a83e34]
00534634: MOV ECX,dword ptr [EAX]
00534636: CMP ECX,EBX
0053463c: PUSH 0x1
0053463e: CALL dword ptr [EDX + 0x20]
00534641: CMP dword ptr [0x00a83e40],EBX
00534647: JNZ 0x0053462f
```

Active in YR: Conditional for full game/session clear/load reset paths.

## 4. INI Keys

No INI key directly controls the global `FactoryClass` array insertion/removal order. Factory existence is indirectly caused by production commands and buildability, which are INI-driven, but the global vector append/remove mechanics are code-level container behavior.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `LogicClass::PerTickUpdate` | ascending global factory array loop after Tactical and before House array; reloads count after each call | `0x0055B66A..0x0055B68B` | Yes |
| `FactoryClass::Constructor` | appends to tail of `g_FactoryClass_Array` | `0x004C9974..0x004C9989` | Yes |
| `FactoryClass::Destructor/Release` | removes by compacting left; no tombstone | `0x004CA7B2..0x004CA7DD` | Yes |
| `HouseClass::Begin_Production` | normal house/category factory creator | xref/call `0x004FA4F5` | Yes |
| `BuildingClass::Update` helper | conditional building-owned factory creator/destructor path | decompile `0x004500F0`; caller `BuildingClass::Update` | Conditional |
| persistence class factory | constructor append during savegame object creation | `0x006C523C..0x006C526A` | Conditional |
| Factory IPersist Save/Load | preserves inner queued-object vector order and swizzles Owner/Object/queue pointers | `0x004CA270`, `0x004CA3C0` | Conditional |
| session clear | releases current array slot 0 until count is zero | `FUN_00534450 @ 0x00534627..0x00534647` | Conditional |

## 6. Current Rust Implementation Status

Static scan only; no Rust files were modified.

| Rust surface | Current shape | Delta |
|---|---|---|
| `src/sim/production/production_types.rs:198` | `queues_by_owner: BTreeMap<InternedId, BTreeMap<ProductionCategory, VecDeque<BuildQueueItem>>>` | Not a native global `FactoryClass` array. Iteration order is owner key then category key, not first-created factory object order. |
| `src/sim/production/production_queue.rs:444..455` | `tick_production_with_overlay_registry` collects `(owner, category)` pairs from maps before iterating | Snapshot owner/category pass; not live-count factory-array semantics. |
| `src/sim/production/production_queue.rs:468..638` | advances and may immediately deliver/popup completed queue fronts | Collapses factory progress and delivery effects more than native `FactoryClass::AI` alone. |
| `src/sim/production/production_spawn.rs:73..90` | `active_producer_by_owner` selects a producer building per owner/category | Producer building selection is separate from native `FactoryClass` object order. |
| `src/sim/world/mod.rs:1690..1700` | Rust production runs as a phase before later `ai::tick_ai` | Broad production-before-AI shape matches prior reports, but global factory-array ordering is missing. |
| `src/sim/ai.rs:64..115` | Rust AI queues commands from high-level owner state | Not native `HouseClass::Update` tail and not a source of factory array order. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FactoryClass::Constructor` global append | verified | decompile `0x004C98B0`; assembly `0x004C9974..0x004C9989` | none |
| `FactoryClass::Destructor/Release` global removal | verified | decompile `0x004CA770`; assembly `0x004CA7B2..0x004CA7DD` | none |
| PerTick factory loop order/count reload | verified | assembly `0x0055B66A..0x0055B68B`; prior tail-order reports | none |
| normal production factory creator | verified | `HouseClass::Begin_Production @ 0x004FA350`; xref `0x004FA4F5` | exact argument names remain decompiler-noisy but order effect is clear |
| building-owned factory creator path | touched-not-exhausted | `0x004500F0`; caller `BuildingClass::Update` | exact stock object types/field `+0xEB8` mapping out-of-scope |
| Factory IPersist inner queue save/load order | verified | `0x004CA270`, `0x004CA3C0`; assembly contexts above | none for inner queue order |
| persistence FactoryClass object construction append | verified | class factory thunk `0x006C523C..0x006C526A` | top-level stream enumeration order |
| top-level save stream order for all FactoryClass objects | deferred | vtable-only xrefs; no direct save enumerator drained in this slot | trace OLE save/load object enumerator for FactoryClass |
| full replay/session reconstruction order | deferred | not inspected | separate replay/savegame investigation |
| Rust production order delta | verified static | `production_types.rs`, `production_queue.rs`, `production_spawn.rs`, `world/mod.rs`, `ai.rs` | runtime fixture tests after implementation |

## 8. Open Questions - Final State

- `[RESOLVED] FGA-001 - Which global vector does PerTick use for factories? -> `0x00A83E34/0x00A83E40`.` (evidence: `0x0055B675..0x0055B68B`)
- `[RESOLVED] FGA-002 - Is factory PerTick order per-house? -> No; it is a standalone global factory array before the global house array.` (evidence: `0x0055B66A..0x0055B68D`)
- `[RESOLVED] FGA-003 - Does constructor append or sorted-insert? -> Append at current count; no comparator or owner/category sort in the append path.` (evidence: `0x004C9974..0x004C9989`)
- `[RESOLVED] FGA-004 - Does destructor tombstone or compact? -> Compacting erase shifts successors left and decrements count.` (evidence: `0x004CA7B2..0x004CA7DD`)
- `[RESOLVED] FGA-005 - Does factory PerTick snapshot count once? -> No; count is reloaded after each factory AI call.` (evidence: `0x0055B683..0x0055B68B`)
- `[RESOLVED] FGA-006 - Does factory PerTick null-check slots? -> No null test appears before dereferencing factory slot/vtable.` (evidence: `0x0055B675..0x0055B680`)
- `[RESOLVED] FGA-007 - What is the normal production creation owner? -> `HouseClass::Begin_Production` creates a new factory when the house category pointer is null.` (evidence: xref `0x004FA4F5`; decompile `0x004FA350`)
- `[RESOLVED] FGA-008 - Are there other direct constructor owners? -> Yes, `BuildingClass::Update` helper `0x004500F0` and persistence class-factory thunk `0x006C524C`.` (evidence: constructor xrefs)
- `[RESOLVED] FGA-009 - Does session clear preserve array removal semantics? -> It repeatedly releases slot 0 while count is nonzero; compacting release makes this current-array-order destruction.` (evidence: `FUN_00534450 @ 0x00534627..0x00534647`)
- `[RESOLVED] FGA-010 - Does FactoryClass Save/Load preserve inner queue order? -> Yes, count then ascending queue slots; load appends placeholders then swizzle-registers each slot in order.` (evidence: `0x004CA270`, `0x004CA3C0`)
- `[RESOLVED] FGA-011 - Does persistence object construction append to the global factory array? -> Yes, the class-factory thunk allocates `0x74` and calls the full constructor.` (evidence: `0x006C523C..0x006C526A`)
- `[RESOLVED] FGA-012 - Is current Rust owner/category production order proven equivalent? -> No; it is nested `BTreeMap` owner/category iteration and snapshot collection.` (evidence: `production_types.rs:198`, `production_queue.rs:444..455`)
- `[DEFERRED] FGA-013 - What exact top-level save stream order enumerates all FactoryClass objects?` (category: `requires-different-system-context`; reason: this slot verified FactoryClass Save/Load and persistence construction but did not drain the OLE/object stream owner; next-step-if-pursued: trace from `FUN_0067d300`/`FUN_0067e730` object enumeration to the FactoryClass class factory and vtable entries.)
- `[DEFERRED] FGA-014 - Which stock building types exercise `0x004500F0` building-owned factory path?` (category: `out-of-scope`; reason: same-frame multi-house/category factory order does not require the type matrix; next-step-if-pursued: trace BuildingType field `+0xEB8` reader/writer and stock INI/type defaults.)
- `[DEFERRED] FGA-015 - Replay restore factory order.` (category: `out-of-scope`; reason: user asked save/load/session reconstruction but replay restore is a separate owner; next-step-if-pursued: start from replay load wrapper after savegame stream owner is finished.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native ticks factories in ascending `g_FactoryClass_Array` order after Tactical and before House, reloading count after every factory AI call. | `0x0055B66A..0x0055B68B`; Active in YR: Yes | Rust ticks production by collected `(owner, ProductionCategory)` pairs from nested maps. | `src/sim/production/production_queue.rs`, `src/sim/world/mod.rs` | Introduce or emulate a native-order factory runtime list whose order is factory object creation order, not owner/category key order. | `factory_global_array_same_frame_completion_uses_creation_order`: create factories A/B in reverse owner sort order, make both complete same native frame, assert completion/delivery side effects follow factory creation order. | Do not sort by owner, category, type, stable ID, or sidebar order when claiming native factory AI order. |
| Native factory construction appends to tail at current count; removal compacts left. | constructor `0x004C9974..0x004C9989`; destructor `0x004CA7B2..0x004CA7DD`; Active in YR: Yes | Rust has no `FactoryClass` object list, only queues and active producer maps. | future `FactoryState`/production queue surfaces | Preserve append and compacting erase semantics, including live-count implications. | `factory_array_remove_compacts_and_shifts_successor`: remove earlier factory, then same-frame iteration observes shifted order exactly as native. | Do not leave tombstones/null slots in the factory AI list; native factory loop has no null guard. |
| Normal `HouseClass::Begin_Production` creates a factory only when the relevant house category pointer is null, and the constructor append occurs before the house pointer assignment. | `0x004FA4E5..0x004FA520`; Active in YR: Yes | Rust creates owner/category queues directly on enqueue. | enqueue/start-production path | First time a house/category begins production should create a native-order factory entry; subsequent queue operations should reuse that factory until release. | `begin_production_reuses_existing_factory_without_reordering`: queue second item in same category and assert factory list index remains unchanged. | Do not create a fresh factory entry for every queued item or reorder on queue append. |
| Factory IPersist Save/Load preserves inner queued-object order and swizzles Owner/Object/queue slots; restored global array append follows persistence object creation order, but top-level factory stream order is not proven here. | `0x004CA270`, `0x004CA3C0`, `0x006C523C..0x006C526A`; Active in YR: Conditional | Rust snapshots structured `ProductionState`; no native factory object stream/order model. | snapshot/native-save importer surfaces | Persist native factory-list order explicitly for Rust snapshots; for native `.SAV` import, do not claim stream-order parity until the OLE object enumerator is traced. | `factory_snapshot_roundtrip_preserves_factory_list_order`: save two factories with non-owner-sorted order, load, next factory tick order unchanged. | Do not rebuild factory order from `queues_by_owner`, `active_producer_by_owner`, houses, or sorted entity IDs after load. |

Proposed test names:

- `factory_global_array_same_frame_completion_uses_creation_order`
- `factory_array_live_count_append_can_tick_same_pass`
- `factory_array_remove_compacts_and_shifts_successor`
- `begin_production_reuses_existing_factory_without_reordering`
- `factory_snapshot_roundtrip_preserves_factory_list_order`

## 10. Negative Facts / Do Not Do

- Do not use Rust `BTreeMap<owner, BTreeMap<category, queue>>` iteration as a stand-in for native factory order. Active in YR: Yes; native uses `g_FactoryClass_Array`.
- Do not describe native order as merely "production before AI" when same-frame multi-factory side effects matter. Active in YR: Yes; order is Tactical -> global factories by array index -> global houses.
- Do not leave null/tombstone entries in a native-mode factory array. Active in YR: Yes; destructor compacts and PerTick does not null-check factory slots.
- Do not assume house array order determines factory order. Active in YR: Yes; `HouseClass::Begin_Production` appends factory objects at first category production time, and later house pointer assignment does not sort the global vector.
- Do not claim save/load factory global array order is fully proven by FactoryClass Load alone. Active in YR: Conditional; this report proves FactoryClass object construction appends during persistence and inner queue order, but not the top-level stream enumeration order for all factories.

## 11. Remaining Uncertainty

- Top-level save stream order for all `FactoryClass` objects remains unresolved. The likely next start point is the save/load stream owner around `FUN_0067d300`/`FUN_0067e730`, following the object enumeration that dispatches FactoryClass IPersist `Save`/`Load`.
- The conditional `BuildingClass::Update` helper `0x004500F0` creates a building-owned factory, but the exact stock building/type matrix for that path was not drained.
- Replay restore/session reconstruction outside the full clear path was not investigated.

## 12. Stale Docs / Replacement Wording

- `FACTORY_HOUSE_BULLET_ANIM_SAME_TICK_SYSTEM_MODEL_SYNTHESIS.md`: replace "Exact global FactoryClass array insertion/reconstruction order is complete: unknown" with "Runtime global `FactoryClass` array order is constructor append order with compacting removal; PerTick uses ascending global array index and reloads count after every factory AI call. Save/load top-level factory stream order remains a bounded follow-up."
- `FACTORY_HOUSE_AI_ORDER_VS_RUST_PRODUCTION_AI_GHIDRA_REPORT.md`: replace "Do not assume owner/category map iteration is equivalent to the native global `FactoryClass` array order" with "Native factory order is first-created `FactoryClass` global-array order: constructor appends at `g_FactoryClass_Array[count]`, destructor compacts left, and PerTick iterates ascending index with live count reload. Rust owner/category map order is a mismatch for same-frame multi-factory completion."
- `FACTORYCLASS_AND_CAMEOENTRY_STRUCT_LAYOUT.md`: replace "Constructor ... registers in `g_FactoryClass_Array`" with "Constructor appends to `g_FactoryClass_Array` at the old count (`0x004C9974..0x004C9989`); release removes by compacting erase (`0x004CA7B2..0x004CA7DD`)."

## Sources

- Ghidra read-only decompile/assembly: `FactoryClass::Constructor @ 0x004C98B0`, `FactoryClass::Destructor/Release @ 0x004CA770`, `LogicClass::PerTickUpdate @ 0x0055AFB0`, `HouseClass::Begin_Production @ 0x004FA350`, `BuildingClass::Update helper @ 0x004500F0`, `FactoryClass::Load @ 0x004CA270`, `FactoryClass::Save @ 0x004CA3C0`, persistence class-factory thunk around `0x006C523C..0x006C526A`, teardown owner `FUN_00534450`.
- Prior docs: `FACTORY_HOUSE_BULLET_ANIM_SAME_TICK_SYSTEM_MODEL_SYNTHESIS.md`, `FACTORY_HOUSE_AI_ORDER_VS_RUST_PRODUCTION_AI_GHIDRA_REPORT.md`, `FACTORYCLASS_AND_CAMEOENTRY_STRUCT_LAYOUT.md`, `FACTORYCLASS_PRODUCTION_DEEP_DIVE.md`, `GRIZZLY_FACTORY_STEP_CADENCE_GHIDRA_REPORT.md`, `PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS_GHIDRA_REPORT.md`, `SAVE_LOAD_ACTIVE_VECTOR_RECONSTRUCTION_OWNER_RESWARM_20260528.md`, `POST_LOAD_OBJECT_98_OWNER_RECONCILIATION_RESWARM_20260528.md`.
- Rust static scan: `src/sim/production/production_types.rs`, `src/sim/production/production_queue.rs`, `src/sim/production/production_spawn.rs`, `src/sim/world/mod.rs`, `src/sim/ai.rs`.
