# FactoryClass Top-Level Save / Load / Restore Order - Reswarm 2026-05-28

**Address(es):** `FUN_0067D300`, `FUN_0067FDF0`, `FUN_0067E730`, `FactoryClass::Constructor @ 0x004C98B0`, Factory class-factory thunk `0x006C523C..0x006C526A`, `FactoryClass::Save @ 0x004CA3C0`, `FactoryClass::Load @ 0x004CA270`, `LogicClass::PerTickUpdate @ 0x0055AFB0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** top-level save stream order for `FactoryClass` instances, load-side object creation order, restore append order into `g_FactoryClass_Array`, and relationship to runtime FactoryClass global-array iteration.
**Non-Scope:** constructor/destructor/per-tick behavior except as comparison evidence; inner per-factory queue order except as already-settled dependency; House production/sidebar mechanics; replay restore.
**Confidence:** High
**Active in YR:** Conditional - active for standard savegame save/load; runtime array iteration is active in normal gameplay.
**Status:** COMPLETE

## 0. Working Notes

Target question: Prove the top-level save stream order and load/restore append order for `FactoryClass` instances, and relate it to runtime `g_FactoryClass_Array` iteration.

Non-goals: Do not redo constructor/destructor/per-tick behavior except as comparison; do not investigate House production/sidebar broadly.

Evidence needed to mark COMPLETE: decompile plus assembly proving the real FactoryClass top-level save emitter, load object-record order, constructor append during restore, count bounds, and current Rust persistence/tick surfaces.

Stop conditions: stop once the FactoryClass OLE record emitter and load append mechanism are proven or explicitly falsified; write only this report and optional `.swarm-claims.md`.

## 1. Overview

The previously deferred top-level FactoryClass save order is now resolved. `FUN_0067D300` reaches a generic OLE-array save helper with `EDX=0x00A83E30`, the `FactoryClass` dynamic-vector header. That helper writes `*(vector+0x10)` as the count and then saves `*(vector+4)[i]` through `OleSaveToStream` in ascending `i`.

Load restores the same stream slot by reading a count and calling `OleLoadFromStream` once per stream record in ascending record order. Each FactoryClass record is created by the FactoryClass COM class factory, which allocates `0x74` bytes and calls the normal `FactoryClass::Constructor`; the constructor appends at `g_FactoryClass_Array[old_count]`. Therefore save/load preserves the runtime global FactoryClass array order exactly: save stream order is ascending `g_FactoryClass_Array` index, and restored array order is the same stream order.

## 2. Key Offsets / Globals

| Offset / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `0x00A83E30` | `FactoryClass` dynamic-vector header / vtable-ish owner passed to generic save helper | save call `0x0067DF08..0x0067DF0F`; constructor grow path `0x004C995E..0x004C9968` | Conditional for save; Yes for runtime construction |
| `0x00A83E34` | `g_FactoryClass_Array` data pointer, used as `*(0xA83E30+4)` by generic save helper | helper `0x0067FDF0` reads `param2+4`; constructor writes `0x004C9983..0x004C9989` | Yes |
| `0x00A83E40` | `g_FactoryClass_Array_Count`, used as `*(0xA83E30+0x10)` by generic save helper | helper `0x0067FDF0` reads `param2+0x10`; PerTick reads `0x0055B66A/0x0055B683` | Yes |
| `FactoryClass+0x44/+0x50` | inner queued-object vector data/count | `FactoryClass::Save/Load @ 0x004CA3C0/0x004CA270` | Conditional save/load |
| `FactoryClass+0x58/+0x6C` | produced object pointer and owner pointer swizzle slots | `0x004CA39B`, `0x004CA3A9` | Conditional save/load |

## 3. Core Logic

### 3.1 Top-level save stream order is the runtime FactoryClass array order

`FUN_0067D300` is the standard `CONTENTS` stream save owner. After earlier object-array sections and before the next array at `0x00A8EB28`, it executes:

```text
0067df08: MOV EDX,0xa83e30
0067df0d: MOV ECX,ESI
0067df0f: CALL 0x0067fdf0
0067df22: MOV EDX,0xa8eb28
0067df27: MOV ECX,ESI
0067df29: CALL 0x0067fe70
```

`0x00A83E30` is the FactoryClass vector header. `get_bulk_xrefs` shows this exact literal is referenced by constructor vector growth (`0x004C995E/0x004C9968`), destructor vector removal (`0x004CA796/0x004CA7A1`), vector init/reset (`0x004E6E7D/0x004E6EA8`), and this save call (`0x0067DF08`). Active in YR: Conditional for savegame save; the vector itself is active in normal production.

`FUN_0067FDF0` is the generic OLE-array saver:

```text
count = *(param2 + 0x10)
stream.Write(&count, 4)
for i = 0; i < count; i++:
    obj = *( *(param2 + 4) + i*4 )
    obj->QueryInterface(IPersistStream, &persist)
    OleSaveToStream(persist, stream)
    persist->Release()
```

Because `param2 = 0x00A83E30`, the saved count is `0x00A83E40` and each saved object pointer comes from `0x00A83E34 + i*4`. There is no sort, owner/category grouping, House pointer-field walk, or reverse traversal in the FactoryClass save block. Active in YR: Conditional - standard savegame save.

Count/bounds detail: the helper writes the current signed `int` count and loops only while `i < count`; if count is `<= 0`, no FactoryClass records are emitted. It does not snapshot or reload the count inside the loop. Active in YR: Conditional save path.

### 3.2 Top-level load consumes FactoryClass records in stream order

`FUN_0067E730` is the matching `CONTENTS` stream load owner. Its object-array load pattern is count first, then ascending record index:

```text
0067e87c: PUSH 0x4
0067e87f: PUSH ESI
0067e880: CALL dword ptr [EAX + 0xc]       ; IStream::Read count
...
0067e895: LEA EDX,[ESP + 0x14]
0067e89a: PUSH 0x7f7c90
0067e89f: PUSH ESI
0067e8a0: CALL EDI                         ; OleLoadFromStream
0067e8aa: MOV EAX,dword ptr [ESP + 0x10]
0067e8ae: INC EBP
0067e8af: CMP EBP,EAX
0067e8b1: JL 0x0067e895
```

The same pattern repeats for later object-array slots, e.g. `0x0067F225..0x0067F241`. Load does not need a `0x00A83E30` literal for the FactoryClass slot because the class identity is in each OLE record; `OleLoadFromStream` instantiates from the CLSID in the stream. The FactoryClass stream slot is identified by the save-side slot at `0x0067DF08..0x0067DF0F`, and load consumes the `CONTENTS` stream sequentially in the same order. Active in YR: Conditional - standard savegame load.

### 3.3 Restore append order comes from FactoryClass class-factory construction

The FactoryClass COM class-factory thunk allocates and constructs each loaded record:

```text
006c523c: PUSH 0x74
006c523e: CALL 0x007c8e17
006c524a: MOV ECX,EAX
006c524c: CALL 0x004c98b0                  ; FactoryClass::Constructor
006c5261: MOV ECX,dword ptr [ESP + 0x14]
006c5267: PUSH EDI
006c5268: PUSH ECX
006c5269: PUSH ESI
006c526a: CALL dword ptr [EAX]
```

The normal constructor appends at the old global-array count:

```text
004c9974: MOV ECX,dword ptr [0x00a83e40]
004c997a: MOV EAX,ECX
004c997c: INC ECX
004c997d: MOV dword ptr [0x00a83e40],ECX
004c9983: MOV ECX,dword ptr [0x00a83e34]
004c9989: MOV dword ptr [ECX + EAX*0x4],ESI
```

Therefore the first FactoryClass record loaded becomes array index 0 after a cleared load-state, the second becomes index 1, and so on. No post-load sort or House/category rebuild was found or needed for the array order. Active in YR: Conditional - restore path; constructor append itself is active in normal production.

### 3.4 Runtime iteration uses the same array order after load

`LogicClass::PerTickUpdate` ticks factories in ascending `g_FactoryClass_Array` index and reloads count after each `FactoryClass::AI`:

```text
0055b66a: MOV EAX,[0x00a83e40]
0055b675: MOV ECX,dword ptr [0x00a83e34]
0055b67b: MOV ECX,dword ptr [ECX + ESI*0x4]
0055b680: CALL dword ptr [EDX + 0x5c]
0055b683: MOV EAX,[0x00a83e40]
0055b688: INC ESI
0055b689: CMP ESI,EAX
0055b68b: JL 0x0055b675
```

After a load, same-frame multi-factory completions therefore occur in the saved/restored FactoryClass stream order, which is the pre-save runtime global-array order. Active in YR: Yes for normal gameplay after savegame load.

## 4. Relationship To Prior Partial Report

The prior negative finding remains true but incomplete: the direct loop at `0x0067CACF..0x0067CAF5` over `g_FactoryClass_Array` calls virtual `+0x34` (`0x004CA430` debug/CRC-style), not `IPersistStream::Save`. The real FactoryClass top-level save emitter is later, at `0x0067DF08..0x0067DF0F`, through generic helper `0x0067FDF0` with `EDX=0x00A83E30`.

Active in YR: Conditional. The `0x0067CACF` loop exists but is not the content save stream; `0x0067DF08` is the savegame content stream FactoryClass section.

## 5. Current Rust Implementation Status

| Rust surface | Current shape | Delta for this target |
|---|---|---|
| `src/sim/production/production_types.rs:197` | `ProductionState` stores `queues_by_owner: BTreeMap<owner, BTreeMap<ProductionCategory, VecDeque<BuildQueueItem>>>`. | Missing explicit native-order `FactoryClass` list. |
| `src/sim/production/production_queue.rs:442..446` | Production tick collects `(owner, category)` from nested maps before ticking. | Iteration order is owner/category key order, not restored FactoryClass array order. |
| `src/sim/world/mod.rs:269` | `Simulation` serializes `production: ProductionState`. | Snapshot restore preserves Rust structures, not native FactoryClass stream order unless a list is added. |
| `src/sim/snapshot.rs:70..97` | `GameSnapshot::save/load` serializes/deserializes the whole `Simulation` through bincode. | No native FactoryClass OLE stream model or restored append order. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| FactoryClass top-level save emitter | verified | `0x0067DF08..0x0067DF0F`; `FUN_0067FDF0` decompile | none |
| FactoryClass save stream order | verified | `FUN_0067FDF0` reads count at vector+0x10 and data at vector+4, loops ascending index | none |
| Factory vector identity | verified | `0x00A83E30` xrefs: constructor/destructor/init/save; `0x00A83E34/40` runtime xrefs | none |
| Load object-record order | verified | `FUN_0067E730` count + ascending `OleLoadFromStream` loop, e.g. `0x0067E895..0x0067E8B1` and repeated slots | none |
| Restore append order | verified | class factory `0x006C523C..0x006C526A`; constructor `0x004C9974..0x004C9989` | none |
| Relationship to runtime iteration | verified | PerTick `0x0055B66A..0x0055B68B` | none |
| Inner queue save/load order | verified by prior work, spot-checked | `0x004CA3C0`, `0x004CA270` | none for this target |
| House production/sidebar behavior | deferred | user non-goal | separate investigation only if requested |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-FS-001 - What is the target question? -> Top-level FactoryClass save stream order and load/restore append order.` (evidence: user prompt; working notes)
- `[RESOLVED] OQ-FS-002 - Is the direct `0x0067CACF` array loop the save stream? -> No; prior report proved it calls virtual `+0x34`, not IPersist Save.` (evidence: `FACTORYCLASS_SAVE_STREAM_GLOBAL_ORDER_RESWARM_20260528.md`; `0x0067CAEA`)
- `[RESOLVED] OQ-FS-003 - Where is the real FactoryClass top-level save emitter? -> `FUN_0067D300` passes `EDX=0x00A83E30` to generic OLE-array saver `FUN_0067FDF0`.` (evidence: `0x0067DF08..0x0067DF0F`)
- `[RESOLVED] OQ-FS-004 - What order does that helper save? -> Count from `+0x10`, then object pointers from `+4 + i*4` in ascending index.` (evidence: `FUN_0067FDF0` decompile)
- `[RESOLVED] OQ-FS-005 - Does the helper reload FactoryClass count during save? -> No; it snapshots count once into a local and loops against that local.` (evidence: `FUN_0067FDF0`)
- `[RESOLVED] OQ-FS-006 - Is there a save-side owner/category or House pointer-field sort? -> No in this save block; it walks `g_FactoryClass_Array` directly by index.` (evidence: `0x0067DF08`, `FUN_0067FDF0`)
- `[RESOLVED] OQ-FS-007 - How does load create FactoryClass objects? -> `OleLoadFromStream` instantiates the FactoryClass COM class factory, which calls `FactoryClass::Constructor`.` (evidence: load loop `0x0067E895..0x0067E8B1`; class factory `0x006C523C..0x006C526A`)
- `[RESOLVED] OQ-FS-008 - What is restore append order? -> The normal constructor appends each loaded FactoryClass at old count, so append order equals stream record order.` (evidence: `0x004C9974..0x004C9989`)
- `[RESOLVED] OQ-FS-009 - How does post-load runtime iteration relate? -> PerTick uses ascending restored array index, so same-frame completions after load follow saved global array order.` (evidence: `0x0055B66A..0x0055B68B`)
- `[RESOLVED] OQ-FS-010 - Are INI keys involved? -> No INI key controls persistence order; factory existence is content/state-driven.` (evidence: no relevant INI references in save/load path; prior INI scans)
- `[RESOLVED] OQ-FS-011 - Does Rust currently preserve this order? -> No explicit native FactoryClass list was found; production is stored/ticked by nested owner/category maps.` (evidence: `production_types.rs:197`, `production_queue.rs:442..446`, `snapshot.rs:70..97`)
- `[DEFERRED] OQ-FS-012 - Full House production/sidebar consequence matrix.` (category: `out-of-scope`; reason: user explicitly excluded broad House production/sidebar; next-step-if-pursued: trace House production management after factory completion.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Top-level save writes FactoryClass objects by ascending `g_FactoryClass_Array` index. | `0x0067DF08..0x0067DF0F`; `FUN_0067FDF0`; Active in YR: Conditional | Missing explicit factory list in snapshots/native-save model | `src/sim/production/production_types.rs`, `src/sim/snapshot.rs` | Persist native-order factory list, separate from owner/category queues. | Save with factories created in order B then A where owner/category sort is A then B; snapshot bytes/load preserve B then A factory list. | Do not serialize only nested `BTreeMap` order and call it native. |
| Load creates FactoryClass records in stream order; each constructor appends at old global count. | `FUN_0067E730` `OleLoadFromStream` loop; `0x006C523C..0x006C526A`; `0x004C9974..0x004C9989`; Active in YR: Conditional | Rust restore has no append-order reconstruction step | snapshot load / production state rebuild | Rebuild runtime factory iteration list from persisted native list order, not from House or queue maps. | `factory_snapshot_roundtrip_restores_factory_array_order`: after load, first production tick visits factories in saved list order. | Do not sort by owner, category, stable id, or House pointer offset after load. |
| Post-load same-frame factory completions use restored array order. | PerTick `0x0055B66A..0x0055B68B`; Active in YR: Yes | Rust production tick collects owner/category pairs from maps | `src/sim/production/production_queue.rs` | Drive production tick order from the native factory list when parity mode is targeted. | `factory_snapshot_roundtrip_same_frame_completion_uses_saved_factory_order`: two factories complete on the first tick after load; delivery/ready side effects occur in saved array order. | Do not let correct per-factory inner queue order mask wrong all-factory ordering. |

Concrete proposed Rust test names:

- `factory_top_level_save_stream_uses_global_array_order`
- `factory_load_restores_global_array_order_from_stream`
- `factory_snapshot_roundtrip_restores_factory_array_order`
- `factory_snapshot_roundtrip_same_frame_completion_uses_saved_factory_order`
- `factory_restore_order_does_not_follow_owner_category_btreemap_order`

## 9. Negative Facts / Do Not Do

- Do not cite the `0x0067CACF..0x0067CAF5` loop as FactoryClass content save order. Active in YR: Conditional; it is a debug/CRC-style virtual `+0x34` walk, not `OleSaveToStream`.
- Do not leave the top-level FactoryClass stream order marked unresolved. Active in YR: Conditional; `0x0067DF08..0x0067DF0F` proves the real save block.
- Do not infer restore order from `HouseClass+0x53AC..0x53CC` factory pointer field order. Active in YR: Conditional; House fields are swizzle slots, while FactoryClass objects are top-level OLE records saved from `0x00A83E30`.
- Do not rebuild post-load factory order from Rust `BTreeMap<owner, category>` order. Active in YR: Yes after load; native PerTick uses restored `g_FactoryClass_Array`.
- Do not merge per-factory queue slot order with all-factory order. Active in YR: Conditional; inner queue order is `FactoryClass::Save/Load`, while all-factory stream order is `FUN_0067D300 -> FUN_0067FDF0(0x00A83E30)`.

## 10. Remaining Uncertainty

None for the claimed slice.

Out of scope: broad House production/sidebar consequences and replay restore.

## 11. Stale Docs / Replacement Wording

- `docs/research/FACTORYCLASS_SAVE_STREAM_GLOBAL_ORDER_RESWARM_20260528.md`: replace the status sentence with:

> COMPLETE SUPERSEDED by `FACTORYCLASS_TOP_LEVEL_SAVE_LOAD_RESTORE_ORDER_RESWARM_20260528.md`: the direct `0x0067CACF` global-array loop is still not IPersist Save, but the real top-level FactoryClass save block is `FUN_0067D300 @ 0x0067DF08..0x0067DF0F`, which passes `EDX=0x00A83E30` to generic OLE-array saver `FUN_0067FDF0`. That helper saves `g_FactoryClass_Array_Count` and then `g_FactoryClass_Array[i]` in ascending index order. Load consumes the same stream records in order through `OleLoadFromStream`; FactoryClass class-factory construction calls the normal constructor, appending each restored factory to `g_FactoryClass_Array` in stream order.

- `docs/research/FACTORYCLASS_GLOBAL_ARRAY_INSERTION_REBUILD_ORDER_RESWARM_20260528.md`: replace the remaining uncertainty bullet "Top-level save stream order for all `FactoryClass` objects remains unresolved" with:

> Resolved by `FACTORYCLASS_TOP_LEVEL_SAVE_LOAD_RESTORE_ORDER_RESWARM_20260528.md`: standard save writes FactoryClass OLE records from `g_FactoryClass_Array` ascending index via `FUN_0067FDF0(0x00A83E30)`, and standard load restores array order by constructing each FactoryClass record in stream order.

- `docs/research/.swarm-claims.md`: replace the slot-1 partial line for `FACTORYCLASS_SAVE_STREAM_GLOBAL_ORDER` with wording that points to this complete follow-up, or add a superseded note:

> superseded by slot-5 `FACTORYCLASS_TOP_LEVEL_SAVE_LOAD_RESTORE_ORDER` COMPLETE; real save emitter is `0x0067DF08..0x0067DF0F`, not the direct checksum/debug loop.

## Sources

- Ghidra read-only decompile/assembly: `FUN_0067D300`, `FUN_0067FDF0`, `FUN_0067E730`, `FactoryClass::Constructor @ 0x004C98B0`, Factory class-factory thunk `0x006C523C..0x006C526A`, `FactoryClass::Save @ 0x004CA3C0`, `FactoryClass::Load @ 0x004CA270`, `LogicClass::PerTickUpdate @ 0x0055AFB0`, `AbstractClass::Save @ 0x00410320`, `AbstractClass::Load @ 0x00410380`, fixup helpers `0x006CF240/0x006CF2C0/0x006CF350`.
- Prior research: `FACTORYCLASS_SAVE_STREAM_GLOBAL_ORDER_RESWARM_20260528.md`, `FACTORYCLASS_GLOBAL_ARRAY_INSERTION_REBUILD_ORDER_RESWARM_20260528.md`, `FACTORY_HOUSE_AI_ORDER_VS_RUST_PRODUCTION_AI_GHIDRA_REPORT.md`.
- Rust static scan: `src/sim/production/production_types.rs`, `src/sim/production/production_queue.rs`, `src/sim/world/mod.rs`, `src/sim/snapshot.rs`.
