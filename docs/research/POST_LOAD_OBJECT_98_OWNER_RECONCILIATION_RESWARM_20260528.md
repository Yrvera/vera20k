# Post-Load Object+0x98 Owner Reconciliation - Reswarm 2026-05-28

**Address(es):** `FUN_0067e440` savegame load wrapper, `FUN_0067e730` content load owner, `FUN_006cf230 -> FUN_006cf350` swizzle fixup pass, `FUN_00551B90` LogicClass vector load helper, `AbstractClass::Save @ 0x00410320`, `AbstractClass::Load @ 0x00410380`, stream `ObjectClass::Save @ 0x0065AC40`, `ObjectClass::Load @ 0x005F5E80`, CRC-style `ObjectClass__Save @ 0x005F6250`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** native savegame post-load path that could restore, swizzle, reconcile, or otherwise repopulate `ObjectClass+0x98` for objects referenced by the saved `LogicClass` active vector.
**Non-Scope:** ordinary Reveal registration ordering, non-Reveal registration caller inventory, pending-delete/destructor drain timing, subclass AI bodies, and runtime debugger validation of an actual retail save after load.
**Confidence:** High for the wrapper/fixup order and absence of a post-load `FUN_0055BAA0` re-registration owner; Medium for per-class final `+0x98` byte value because proving every Object-derived load constructor is outside this slice.
**Active in YR:** Yes. `FUN_0067e440` is the standard load-game wrapper for the `"LOADING GAME: %s"` path, calls `FUN_0067e730`, then calls `FUN_006cf230(&DAT_00B0C110)` and post-load refresh helpers.

## 1. Overview

The missing post-load owner is not a hidden re-registration pass. The standard savegame load wrapper resolves saved pointers with the generic swizzle manager after `FUN_0067e730`, but that fixup only rewrites queued pointer slots from old saved addresses to new object addresses; it does not call `FUN_0055BAA0`, does not inspect `ObjectClass+0x98`, and does not set a membership byte.

The important correction is that there are two "save" surfaces. `ObjectClass__Save @ 0x005F6250` is the CRC/checksum-style object save routine already known to omit `+0x98`; savegame persistence for Object-derived classes goes through the IPersist stream chain, where `FUN_0065AC40` calls `AbstractClass::Save @ 0x00410320` and that writes a raw class-sized memory body. Load-specific constructors then reset volatile fields for many active object subclasses, including the normal constructor path that initializes `Object+0x98` to `0`.

## 2. Class Layout / Key Offsets

| Field | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `LogicClass+0x04` | active object pointer array | `FUN_00551B90` appends saved pointer tokens and later queues each slot with `FUN_006CF240` | Yes |
| `LogicClass+0x10` | active vector count | saved/loaded by `FUN_00551B20` / `FUN_00551B90` | Yes |
| `ObjectClass+0x98` | membership byte used by `FUN_0055BAA0` / `FUN_0055BAE0` | setter `0x0055BAC6`; remover branch `0x0055BAE0`; constructor decompile shows `*(undefined1 *)(this+0x98)=0` as `param_1+0x26` | Yes |
| `DAT_00B0C110+0x08/+0x14` | queued pointer-slot list data/count | `FUN_006CF240`; `FUN_006CF350` sorts and consumes it | Yes |
| `DAT_00B0C110+0x20/+0x2C` | old-object to new-object swap-map data/count | `FUN_006CF2C0`; `FUN_006CF350` sorts and consumes it | Yes |

## 3. Core Logic

### 3.1 Load wrapper order

`FUN_0067e440` is the active savegame load wrapper. Its decompile and callee list show this order:

1. open the save storage and `CONTENTS` stream;
2. call `FUN_0067e730`;
3. call a vtable method on an object held in the wrapper frame;
4. call `FUN_006cf230(&DAT_00B0C110)`;
5. run post-load refresh helpers: `FUN_00685120`, `FUN_006D03A0`, `FUN_006D04F0(1)`, `SidebarClass::InitSurface`, `TiberiumClass::InitGrowthQueues_All`, `TiberiumClass::InitSpreadQueues_All`, `RadarClass::RefreshRadar`, `FUN_006842F0`, and `FUN_0072DEF0`.

Assembly context confirms the load/fixup order around the wrapper:

- `0x0067e440..0x0067e452`: wrapper entry logs `"LOADING GAME: %s"`.
- decompile of `FUN_0067e440`: `FUN_0067e730(); ... FUN_006cf230(&DAT_00b0c110); FUN_00685120(); ...`.
- `get_function_callees(FUN_0067e440)`: includes `FUN_0067e730`, `FUN_006cf230`, and the post-load refresh helpers; it does not include `FUN_0055BAA0` or `FUN_0055BAE0`.

Active in YR: Yes, this is the standard load-game path. It is not an INI-gated or TS-only path.

### 3.2 LogicClass vector load remains order-preserving

This report inherits the prior verified vector-owner result and spot-checked the post-load integration point. `FUN_0067e730` calls `FUN_00551B90` with `ECX=0x87F778`; the helper reads the saved count, appends saved pointer tokens in stream order, then queues every vector slot with `FUN_006CF240`.

The vector slot is not immediately a valid object pointer after `FUN_00551B90`: `FUN_006CF240` records the old pointer token plus the pointer-slot address and clears the slot to zero. The later wrapper-level `FUN_006CF230 -> FUN_006CF350` pass resolves it.

Active in YR: Yes.

### 3.3 Swizzle fixup pass does not restore `Object+0x98`

`FUN_006CF230` is a thin call to `FUN_006CF350`.

`FUN_006CF350`:

1. checks whether the queued pointer-slot list count at `+0x14` is nonzero;
2. sorts the old->new swap map at `+0x20/+0x2C` when it has entries;
3. sorts the pointer-slot queue at `+0x08/+0x14` when it has entries;
4. walks both sorted lists;
5. when a slot's saved old pointer equals a swap-map old pointer, writes the swap-map new pointer to the queued slot address;
6. clears both vector-like queues through their vtable `+0x0C` clear calls.

The write is only `*(int *)slot_address = new_object_pointer`. There is no object-byte write, no branch on `slot_address` belonging to the `LogicClass` vector, and no call to the normal registration/removal helpers.

Evidence:

- `FUN_006CF350` decompile: the matching branch writes `*(int *)*piVar1 = piVar3[1]`.
- `FUN_006CF350` assembly range `0x006CF350..0x006CF400` disassembled successfully.
- `get_function_callers(0x006CF350)`: only `FUN_006CF230` and `SwizzleManagerClass__Constructor`.
- `get_function_callers(0x006CF230)`: `FUN_0067E440` and `ScenarioClass::Full_Init`.

Active in YR: Yes for savegame load through `FUN_0067E440`; conditional for scenario full init, which uses the same generic fixup manager.

### 3.4 Normal membership setter is not on the load wrapper path

The only verified callers of `FUN_0055BAA0` in this Ghidra session are:

- `BuildingLightClass__Constructor @ 0x00435820`;
- `FUN_00437050` BuildingLight reveal wrapper;
- `FUN_0075F8B0` WaveClass reveal wrapper;
- `ObjectClass::Reveal @ 0x005F4EC0`;
- `TechnoClass::SetInOpenTransport @ 0x00710470`.

None are `FUN_0067E440`, `FUN_0067E730`, `FUN_006CF230`, `FUN_006CF350`, `FUN_00685120`, `FUN_006D03A0`, `FUN_006D04F0`, `FUN_006842F0`, or `FUN_0072DEF0`.

`FUN_0055BAA0` itself is still the ordinary setter: it checks `Object+0x98`, inserts into the vector only when the byte is zero, and writes `Object+0x98 = 1` only after insert succeeds. That is not what the savegame load wrapper uses.

Active in YR: Yes for the listed caller paths; No for savegame post-load restoration.

### 3.5 Savegame persistence surface is not `ObjectClass__Save @ 0x005F6250`

`ObjectClass__Save @ 0x005F6250` omits `+0x98`, but this function is not the IPersist stream save body for ObjectClass savegames. It calls `AbstractClass__ComputeCRC`, then writes object fields to CRC/checksum helpers such as `FUN_004A1D50` and `FUN_004A1CA0`. Its callers include object-specific save/checksum extras such as `AnimClass__SaveExtras`, `BuildingLightClass__Save`, `MissionClass__Save`, and `TerrainClass__Save`.

The stream save path for ObjectClass is `FUN_0065AC40`, called by `FUN_0070C250`. `FUN_0065AC40` calls `AbstractClass::Save @ 0x00410320`, then saves the ObjectClass dynamic vector at `+0xE0`. `AbstractClass::Save` writes:

1. the saved `this` pointer, 4 bytes;
2. a raw memory body of size returned by virtual slot `+0x30` (`GetClassSize`).

That raw body includes bytes inside the object layout, including `Object+0x98`, unless a derived stream load constructor later overwrites them.

Evidence:

- `AbstractClass::Save @ 0x00410320` decompile writes `param_1` with size returned by `(**(code **)(*param_1 + 0x30))(0)`.
- `FUN_0065AC40` decompile calls `AbstractClass__Save` first.
- `get_function_callers(0x0065AC40)`: `FUN_0070C250`.
- `get_function_callers(0x005F6250)`: object save/checksum extras, not the `FUN_0070C250` stream save wrapper.

Active in YR: Yes. This distinction matters because the previous unresolved report treated `0x005F6250` as decisive for savegame byte persistence; it is decisive only for that CRC/checksum-style surface.

### 3.6 Load-specific constructors can wipe raw-loaded `+0x98`

Several active Object-derived load paths call a load-specific constructor after the raw stream load. Those constructors reach `ObjectClass::Constructor`, which initializes `Object+0x98` to zero.

Verified examples:

- `AircraftClass::Load @ 0x0041B430` calls `FootClass::Load @ 0x004DB3C0`, then calls `FootClass::Constructor @ 0x004D3540`; assembly context `0x0041B4B8..0x0041B4DC`.
- `FootClass::Constructor @ 0x004D3540` calls `TechnoClass::Constructor`.
- `TechnoClass::Constructor @ 0x006F2B40` calls `RadioClass::Constructor -> MissionClass::Constructor -> ObjectClass::Constructor`.
- `ObjectClass::Constructor @ 0x005F3900` initializes `Object+0x98` to zero (`*(undefined1 *)(param_1 + 0x26) = 0`, where the decompiler has typed `param_1` as a dword pointer).
- `BuildingClass::Load @ 0x00453E20` calls `FUN_0070BF50` stream load, then calls load-specific `BuildingClass::Constructor @ 0x0043B680`; assembly context `0x00453EB1..0x00453ECF`.
- `BuildingClass::Constructor @ 0x0043B680` calls `TechnoClass::Constructor`, so it also reaches the ObjectClass constructor default.

This means raw persistence alone is not a sufficient answer for active Techno/Foot/Building/Aircraft objects: the post-load constructor sequence can reset the byte after raw load. The savegame wrapper still does not re-set it afterward through `FUN_0055BAA0` or the swizzle pass.

Active in YR: Yes for these class load paths when those object classes are present in a savegame.

## 4. INI Keys

No INI key gates this savegame restore mechanism. The relevant paths are COM/IPersist stream save/load, pointer swizzle fixup, and post-load engine refresh.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `FUN_0067E440` | standard savegame load wrapper; runs content load then swizzle fixup then refresh helpers | decompile/callees; `"LOADING GAME: %s"` string | Yes |
| `FUN_0067E730` | content load owner; includes `LogicClass` vector load through `FUN_00551B90` | prior report plus wrapper call | Yes |
| `FUN_00551B90` | loads saved active vector entries and queues pointer-slot fixups | prior report; `FUN_006CF240` xrefs include `FUN_00551B90` | Yes |
| `FUN_006CF230 -> FUN_006CF350` | generic old-pointer to new-pointer fixup; no membership-byte side effect | decompile; callers `FUN_0067E440`, `ScenarioClass::Full_Init` | Yes |
| `FUN_0055BAA0` | normal add/register helper and only direct `Object+0x98=1` setter found in this slice | decompile and caller list | Yes, but not on savegame post-load |
| `ObjectClass__Save @ 0x005F6250` | CRC/checksum-style routine; omits `+0x98`; not stream save body | decompile and caller list | Yes, but not the post-load persistence owner |
| `FUN_0065AC40 -> AbstractClass::Save` | stream ObjectClass save path; writes raw class-sized memory body | decompile and caller list | Yes |

## 6. Current Rust Implementation Status

Static scan only; no Rust files were modified.

| Rust surface | Current shape | Delta |
|---|---|---|
| `src/sim/world/mod.rs:288` | `live_object_order` is serialized by default (`#[serde(default)]`, not skipped). | This matches native vector persistence directionally, but Rust lacks a separate object-local membership byte and savegame pointer fixup distinction. |
| `src/sim/world/mod.rs:612` | `register_live_object` deduplicates by scanning `Vec<u64>`. | Native ordinary registration deduplicates by `Object+0x98`, not by vector scan. |
| `src/sim/world/mod.rs:618` | `unregister_live_object` always retains/removes by ID. | Native remover first gates on `Object+0x98`; if the byte is zero, it performs no vector search/removal. |
| `src/sim/world/mod.rs:622` | `live_object_order_snapshot` appends sorted missing `EntityStore` IDs after live order. | Native savegame load does not have a sorted repair pass in the verified wrapper/fixup path. |
| `src/sim/snapshot.rs:84` / `:107` | full `Simulation` bincode save/load; caller rebuilds skipped caches. | No native-equivalent two-phase raw-body plus swizzle fixup plus volatile constructor reset model exists yet. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0067E440` wrapper order | verified | decompile, callee list, assembly context `0x0067E440..0x0067E730` | none |
| `FUN_0067E730` vector load call | verified | prior report; wrapper call chain | none for this slice |
| `FUN_00551B90` vector load helper | verified | prior report; `FUN_006CF240` xrefs include helper | none for this slice |
| `FUN_006CF230` | verified | decompile; callers are wrapper/full-init | none |
| `FUN_006CF350` fixup pass | verified | decompile, disassembly `0x006CF350..0x006CF400`, callers | none |
| savegame post-load `FUN_0055BAA0` registration | verified negative | `get_function_callers(0x0055BAA0)` excludes save/load wrapper and refresh helpers | none |
| `ObjectClass__Save @ 0x005F6250` role | verified | decompile and caller list | rename/stale-doc cleanup only |
| stream `ObjectClass` save path `FUN_0065AC40` | verified | decompile; callers; `AbstractClass::Save` raw-body write | none |
| load-specific constructor reset examples | verified for Aircraft/Foot/Building chain | decompile and assembly contexts `0x0041B4B8..0x0041B4DC`, `0x00453EB1..0x00453ECF` | full inventory of every Object-derived load constructor is outside this slot |
| exact runtime byte after loading every possible object class | touched-not-exhausted | representative active classes verified; no runtime debugger observation | runtime save/load watchpoint would close final byte-state sampling |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-PLO98-001 - Is this exhaustive-slice or coverage-map? -> exhaustive-slice for the standard savegame wrapper's post-load reconciliation owner.` (evidence: user scope; `FUN_0067E440`)
- `[RESOLVED] OQ-PLO98-002 - What function owns standard savegame load after `FUN_0067E730`? -> `FUN_0067E440` wraps `FUN_0067E730`, then swizzle fixup, then post-load refresh.` (evidence: `FUN_0067E440` decompile)
- `[RESOLVED] OQ-PLO98-003 - Does the wrapper call `FUN_0055BAA0` after load? -> No; callee list excludes it, and `FUN_0055BAA0` callers are only BuildingLight, WaveClass, Reveal, and OpenTransport paths.` (evidence: `get_function_callees(0x0067E440)`, `get_function_callers(0x0055BAA0)`)
- `[RESOLVED] OQ-PLO98-004 - Does the wrapper call the swizzle fixup pass after `FUN_0067E730`? -> Yes, `FUN_006CF230(&DAT_00B0C110)` runs after `FUN_0067E730`.` (evidence: `FUN_0067E440` decompile)
- `[RESOLVED] OQ-PLO98-005 - Does `FUN_006CF230` do anything besides call fixup? -> It is a thin wrapper around `FUN_006CF350` and returns 0.` (evidence: `FUN_006CF230` decompile)
- `[RESOLVED] OQ-PLO98-006 - Does `FUN_006CF350` set `Object+0x98`? -> No; it sorts fixup queues and writes resolved new object pointers into queued pointer slots only.` (evidence: `FUN_006CF350` decompile; disassembly `0x006CF350..0x006CF400`)
- `[RESOLVED] OQ-PLO98-007 - Is `ObjectClass__Save @ 0x005F6250` the savegame stream body? -> No; the stream ObjectClass save body is `FUN_0065AC40`, while `0x005F6250` is CRC/checksum-style and omits `+0x98` only for that surface.` (evidence: callers of `0x005F6250`, callers of `0x0065AC40`, decompile of both)
- `[RESOLVED] OQ-PLO98-008 - Does IPersist stream save raw object bytes? -> Yes; `AbstractClass::Save` writes the saved `this` pointer and then writes a raw class-sized body from `this`.` (evidence: `0x00410320` decompile)
- `[RESOLVED] OQ-PLO98-009 - Does IPersist stream load register old-this to new-this for swizzling? -> Yes; `AbstractClass::Load` calls `FUN_006CF2C0(&DAT_00B0C110, old_this, new_this)` before raw body read.` (evidence: `0x00410380` decompile; `get_function_callers(0x006CF2C0)`)
- `[RESOLVED] OQ-PLO98-010 - Can load-specific constructors overwrite raw-loaded `+0x98`? -> Yes for verified Techno-derived examples; Aircraft and Building load paths call constructors after stream load, and the constructor chain reaches ObjectClass constructor defaulting `+0x98` to 0.` (evidence: `0x0041B4B8..0x0041B4DC`, `0x00453EB1..0x00453ECF`, `0x005F3900`, `0x006F2B40`)
- `[RESOLVED] OQ-PLO98-011 - Does `FUN_00685120` restore active membership? -> No; it initializes display/radar/visual surfaces and calls no registration helper.` (evidence: `FUN_00685120` decompile/callees)
- `[RESOLVED] OQ-PLO98-012 - Do `FUN_006D03A0` or `FUN_006D04F0` restore active membership? -> No; they are sidebar/surface paths and call no registration helper.` (evidence: decompile of both)
- `[RESOLVED] OQ-PLO98-013 - Do `FUN_006842F0` or `FUN_0072DEF0` restore active membership? -> No; they toggle display mode / free transition assets and call no registration helper.` (evidence: decompile/callees)
- `[RESOLVED] OQ-PLO98-014 - Is there an INI key gate? -> No; this is savegame stream/swizzle mechanics, not INI-driven behavior.` (evidence: function bodies; no INI reads)
- `[DEFERRED] OQ-PLO98-015 - What is the observed byte value for every Object-derived class after a retail save/load?` (category: `needs-runtime-debugger`; reason: static evidence proves wrapper has no byte-reconcile pass, but exact final byte sampling for every class variant needs runtime watchpoints; next-step-if-pursued: set watchpoints on `Object+0x98` for a saved active unit/building/light/wave through load completion.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Savegame load resolves the saved `LogicClass` vector through the generic swizzle pass; the pass only writes queued pointer slots. | `FUN_0067E440`, `FUN_006CF230`, `FUN_006CF350`; Active in YR: Yes | Rust uses stable IDs and bincode, no old-pointer/new-pointer swizzle phase. | `src/sim/snapshot.rs:84`, `src/sim/world/mod.rs:288` | Preserve active order as saved state, but model missing/deleted references explicitly instead of sorted repair. | Save with live order different from stable ID order, load, and assert next live-order consumer sees the exact saved order. | Do not rebuild post-load active order from `EntityStore` sorted IDs. |
| The savegame wrapper does not re-register vector members through `FUN_0055BAA0` after load. | `get_function_callers(0x0055BAA0)` and `FUN_0067E440` callee list; Active in YR: Yes negative | Rust currently has no separate membership byte; `register_live_object` scans the vector and `unregister_live_object` removes unconditionally. | `src/sim/world/mod.rs:612`, `src/sim/world/mod.rs:618` | Future membership model must separate "in active vector" from "membership byte says remover should search." | After load, exercise a native-matched conceal/despawn scenario once runtime byte sampling exists; assert removal behavior follows the byte gate, not just vector containment. | Do not assume vector membership and duplicate/removal byte are automatically equivalent after save/load. |
| `ObjectClass__Save @ 0x005F6250` omits `+0x98`, but that function is not the stream save body; stream save uses `FUN_0065AC40 -> AbstractClass::Save` raw body. | `0x005F6250`, `0x0065AC40`, `0x00410320`; Active in YR: Yes | Rust bincode serializes structured fields, not native raw memory plus volatile constructor reset. | snapshot serialization / future native save importer | Treat Rust snapshot parity separately from native `.SAV` stream import. If native `.SAV` import is implemented, use class-size raw body semantics plus post-load constructor/fixup semantics, not the CRC surface. | A native-save importer test should distinguish fields present only in raw body from fields included in checksum-style object save. | Do not cite `0x005F6250` as proof that savegames cannot contain `+0x98`. |
| Load-specific constructors can reset `Object+0x98` after raw load for verified Techno-derived active classes. | `AircraftClass::Load @ 0x0041B430`; `BuildingClass::Load @ 0x00453E20`; `ObjectClass::Constructor @ 0x005F3900`; Active in YR: Yes | Rust has no constructor-after-deserialize volatile reset phase matching native. | snapshot load / future native save importer | Keep authoritative save/load state and volatile reinit semantics distinct. | Load a saved active unit/building, then compare active-vector iteration and subsequent unregister behavior against native once runtime byte watchpoints are collected. | Do not blindly persist every Rust runtime bit as authoritative if native constructors reset the analogous byte after raw load. |

## 10. Negative Facts / Do Not Do

- Do not implement a post-load Rust pass that simply calls the normal registration helper for every saved active object. Native `FUN_0067E440` does not call `FUN_0055BAA0` after load.
- Do not use `ObjectClass__Save @ 0x005F6250` as the savegame serialization proof for `Object+0x98`. It is a CRC/checksum-style object save surface; the stream save body is `FUN_0065AC40`.
- Do not make the swizzle pass responsible for gameplay side effects. Native `FUN_006CF350` only patches pointer slots and clears the swizzle queues.
- Do not let Rust's sorted fallback in `live_object_order_snapshot` become a parity claim. Native load preserves and swizzles saved vector slots; no sorted repair was found.

## 11. Stale Docs / Replacement Wording

- `docs/research/SAVE_LOAD_ACTIVE_VECTOR_RECONSTRUCTION_OWNER_RESWARM_20260528.md`: replace "The exact post-load owner that restores or reconciles `Object+0x98` for active-vector members remains unresolved" with "The standard savegame wrapper does not run a post-load `Object+0x98` reconciliation pass: after `FUN_0067e730`, `FUN_006cf230 -> FUN_006cf350` only resolves queued pointer slots, and the known `FUN_0055BAA0` callers exclude the save/load wrapper. The remaining uncertainty is runtime sampling of the final byte across every Object-derived class after load-specific constructor resets."
- `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`: replace "ObjectClass::Save @ 0x005F6250 does not serialize `ObjectClass+0x98`" with "`ObjectClass__Save @ 0x005F6250` is a CRC/checksum-style object save surface and omits `+0x98`; savegame IPersist stream persistence for ObjectClass goes through `FUN_0065AC40 -> AbstractClass::Save @ 0x00410320`, which writes a raw class-sized body before load-specific constructors and swizzle fixups run."
- `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`: replace "Exact save/load and replay reconstruction of `Object+0x98` plus the LogicClass vector remains unresolved" with "Savegame active-vector order is saved/loaded directly and swizzled; the standard load wrapper does not re-register vector members through `FUN_0055BAA0`, and runtime byte sampling is still needed before assigning final per-class post-load `Object+0x98` values."

## Sources

- Ghidra decompile/assembly/call evidence: `FUN_0067E440`, `FUN_0067E730`, `FUN_00551B90`, `FUN_006CF230`, `FUN_006CF350`, `FUN_006CF240`, `FUN_006CF2C0`, `AbstractClass::Save @ 0x00410320`, `AbstractClass::Load @ 0x00410380`, `FUN_0065AC40`, `ObjectClass::Load @ 0x005F5E80`, `ObjectClass__Save @ 0x005F6250`, `ObjectClass::Constructor @ 0x005F3900`, `TechnoClass::Constructor @ 0x006F2B40`, `FootClass::Constructor @ 0x004D3540`, `AircraftClass::Load @ 0x0041B430`, `BuildingClass::Load @ 0x00453E20`, `BuildingClass::Constructor @ 0x0043B680`, `FUN_00685120`, `FUN_006D03A0`, `FUN_006D04F0`, `FUN_006842F0`, `FUN_0072DEF0`.
- Prior docs referenced: `docs/research/SAVE_LOAD_ACTIVE_VECTOR_RECONSTRUCTION_OWNER_RESWARM_20260528.md`, `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`, `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`, `docs/research/BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md`.
- Rust static scan: `src/sim/world/mod.rs`, `src/sim/snapshot.rs`.
