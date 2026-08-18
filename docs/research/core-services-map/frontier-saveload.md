# frontier-saveload — Save/load + swizzle serializer (substrate profile)

**Slug:** `frontier-saveload`
**Status:** PROMOTED from catalog stub (was `_frontier.md` §H2). Profile-level, not a full
decode — the orchestrator (stub-marked UNVERIFIED) is **LOCATED**, and the cross-service
edges are confirmed against byte-level-verified corpus citations.
**Authority order:** binary → Ghidra → docs.
**Active in YR:** YES — the `.SAV` save/load path is the standard menu Save/Load Game in
campaign and skirmish; it is OUT-OF-SIM (menu/command-triggered), not a per-tick rung.

> ⚠️ **Ghidra connectivity note (this session):** The Ghidra MCP bridge was **not reachable**
> for this promotion (`list_instances` → 0 instances; `connect_instance` TCP
> `127.0.0.1:8089` actively refused; the static-analysis tool group could not be loaded
> without a connected instance). The orchestrator addresses and the cross-service edges below
> are therefore re-verified against **already-live-decompiled** corpus reports — three
> independent docs (`SAVELOAD_LOGIC_ACTIVE_VECTOR_RECONSTRUCTION`, `BUILDINGCLASS_SAVE_LOAD`,
> `HOUSE_RESULT_BYTES_SAVE_LOAD_PERSISTENCE_1F6_1F7_1F8`) cite the same `FUN_0067D300` /
> `FUN_0067E730` / `DAT_00B0C110` / `FUN_006CF240` from separate live sessions and corroborate
> each other. Items needing a fresh *live* re-pull are flagged **NEEDS-LIVE-REVERIFY** inline.

---

## 1. Purpose

Whole-game serialization to/from a `.SAV` file. The save format is the **cross-cut of every
studied service's persisted fields** — there is no central "save schema"; instead every
`Abstract`-derived game object serializes itself through a COM **IPersistStream** contract,
the container is an **OLE 2 Compound Document**, and inter-object pointers are persisted as
raw save-time addresses then rewritten (swizzled) to their new post-load addresses by a single
global fixup pass.

This service answers: *how does the entire live game state (houses, objects, cells, the Logic
active vector, supers, animations) round-trip through a menu Save → Load with every pointer
re-connected, in the exact same live order?*

The mechanism is uniform and address-based (no per-type tags in the fixup), so it scales
across *any* Abstract-derived pointer field without per-class fixup code.

---

## 2. Orchestrator — LOCATED (stub said UNVERIFIED)

**Stub claim (H2):** "Top-level orchestrator address UNVERIFIED — locate it via the Save menu
command xrefs." Representative fns given: `HouseClass__Save @ 0x00504080`,
`CellClass__Save @ 0x00483C10`, `AbstractClass__Save @ 0x00410320`, load swizzle helper
`FUN_006CF240`.

**Verdict: the orchestrator is LOCATED.** It is a Save_Game / Load_Game function pair, already
live-decompiled in the corpus:

| Orchestrator | Address | Role | Evidence (prior live sessions) |
|---|---|---|---|
| **Save_Game** | `FUN_0067D300` | top-level save: writes `g_HouseClass_Array_Count`, iterates `g_HouseClass_Array[i]` and `OleSaveToStream`s each via IPersistStream; saves AnimTypes/Tubes; calls `Logic.Save` (active vector); saves the `0x00887324` object. | `SAVELOAD_LOGIC_…` §1/§5 (`get_assembly_context 0x0067d435`: `MOV ECX,0x87f778; CALL 0x00551b20`); `HOUSE_RESULT_BYTES_…` §3.1 (`decompile 0x0067D300`) |
| **Load_Game** | `FUN_0067E730` | top-level load: `Clear_Scene` → object-heap loads (`OleLoadFromStream`) → `Map`-singleton load (`ECX=0x87F7E8; CALL 0x00581F50`) → `Logic.Load` → release of the `0x00887324` object → (outer pointer-fixup pass). | `SAVELOAD_LOGIC_…` §3.3/§5 (`get_assembly_context 0x0067e8d2`: `MOV ECX,0x87F778; CALL 0x00551B90`) |
| **Clear_Scene** | `FUN_006851F0` | pre-load reset: deletes `g_ObjectClass_Array` objects, clears `g_CurrentObjects`, resets the Logic vector via vtable `+0x0C`. Runs first inside Load_Game. | `SAVELOAD_LOGIC_…` §3.2 (`decompile 0x006851F0`) |

The four representative fns the stub listed are all **real and correct** as per-class
contributors (`AbstractClass::Save @ 0x00410320`, `HouseClass::Save @ 0x00504080`,
`CellClass::Save @ 0x00483C10`) — they are the leaf `IPersistStream::Save` slots the
orchestrator drives, not the orchestrator itself. The swizzle helper `FUN_006CF240` is correct
(see §4). **NEEDS-LIVE-REVERIFY** on next live session: re-pull `FUN_0067D300` / `FUN_0067E730`
bodies to confirm the exact stream order and the location of the outer fixup-apply loop (§5
shows it is *inferred*, not yet decompiled in any corpus doc).

> **CellClass::Save @ 0x00483C10** — taken from the stub; not independently re-cited in the
> corpus docs read this session. **NEEDS-LIVE-REVERIFY.** (The Map-singleton load at
> `0x00581F50` is the verified cell-side load entry; the per-cell `Save` slot address is
> stub-grade.)

---

## 3. Architecture (the four invariants)

1. **IPersistStream is the per-object contract.** Every `Abstract`-derived class exposes
   `IsDirty`/`Load`/`Save`/`GetClassID`/`GetSizeMax` via a secondary vtable at `this+0x04`;
   the **primary** vtable re-uses the same slots. Per IPersistStream method order
   (`IsDirty @4, Load @5, Save @6, GetSizeMax @7`), **slot 5 = Load, slot 6 = Save** — note
   this is **reversed** from a naive "5=Save,6=Load" reading, and several *caller*-side Ghidra
   labels (e.g. `CaptureManagerClass__Save/Load`) are **swapped** — trust the code, not the
   label. (`BUILDINGCLASS_SAVE_LOAD` slot-order note.)
2. **`AbstractClass::Save @ 0x00410320` writes the whole instance as a raw memory dump**:
   `stream.Write(&old_this, 4)` then `stream.Write(this, GetSizeMax())`, then clears IsDirty.
   Each subclass `Save` *only* appends the contents of nested DynamicVectors (whose `Items`
   point at separate heap allocations the raw dump can't capture).
3. **`AbstractClass::Load @ 0x00410380` reads the raw memory back**, reads the saved `old_this`,
   and registers `(old_this → new_this)` in the global swap-map. Each subclass `Load` re-runs
   its constructor (to re-seat vtables + sub-object vtables), re-reads the nested vectors, and
   registers every embedded pointer slot via `FUN_006CF240` for later fixup.
4. **A single global pointer-fixup (swizzle) pass** runs after *every* object is loaded: for
   each registered slot, look up its saved old-pointer value in the swap-map and write the new
   pointer back. O(slots × objects), purely address-based, type-agnostic.

---

## 4. What it owns (globals / structures, with addresses)

| Owned state | Address | Meaning | Grade / source |
|---|---|---|---|
| **Pointer-fixup dictionary** (the "SwizzleManager") | `DAT_00B0C110` (~0x38 bytes) | two sub-lists: a **pointer-slot list** (`(saved_value, &slot)` pairs, `+0x08..+0x18`) and a **swap-map** (`(old_this, new_this)` pairs, 8 B each, `+0x20..+0x30`). This *is* the YRpp `SwizzleManagerClass`; no class is literally named that in the Ghidra DB. | verified — `BUILDINGCLASS_SAVE_LOAD` §5 (decomp), `SAVELOAD_LOGIC_…` §2 |
| ID directory | `DAT_00B0E840` | keyed by `AbstractClass::ID` (`this+0x0C`); tracks all living Abstract objects by ID for `ID → this` lookup after load. Load removes+re-inserts to stay consistent. | verified — `BUILDINGCLASS_SAVE_LOAD` §5 |
| LogicClass active vector | `0x0087F778` | the active-object `DynamicVectorClass` (vtable `0x007E18FC`; items `+0x04`, capacity `+0x08`, count `+0x10`). Save_Game writes it via `DynamicVectorClass::Save`; Load_Game restores it verbatim. | verified — `SAVELOAD_LOGIC_…` §2 (`read_memory 0x007E18FC`) |
| Map singleton (load target) | `0x0087F7E8` | restored via `ECX=0x87F7E8; CALL 0x00581F50` inside Load_Game (cells/terrain). | verified — `SAVELOAD_LOGIC_…` §5 |
| HouseClass array + count | `g_HouseClass_Array` / `g_HouseClass_Array_Count` | Save_Game writes the count then `OleSaveToStream`s each house through vtable `+0x18` (`0x00504080`). | verified — `HOUSE_RESULT_BYTES_…` §3.1 |
| ObjectClass array (cleared on load) | `g_ObjectClass_Array` / `g_CurrentObjects` | deleted/cleared by Clear_Scene before the load stream is read. | verified — `SAVELOAD_LOGIC_…` §3.2 |
| Misc save object | `0x00887324` | saved by Save_Game, released after Load_Game; role unspecified in corpus. | `SAVELOAD_LOGIC_…` §5 — **NEEDS-LIVE-REVERIFY** |

### Per-class Save/Load vtable slots (representative, verified leaf entries)
| Class | Load (slot 5) | Save (slot 6) | Source |
|---|---|---|---|
| `AbstractClass` | `0x00410380` | `0x00410320` (raw dump) | `BUILDINGCLASS_SAVE_LOAD` slot-order note |
| `ObjectClass` | `0x005F5E80` | `0x005F6250` | `SAVELOAD_LOGIC_…` §2, `ACTIVE_OBJECT_ORDER_…` §3.5 |
| `HouseClass` | `0x00503040` (slot `+0x14`) | `0x00504080` (slot `+0x18`) | `HOUSE_RESULT_BYTES_…` §3.2 |
| `BuildingClass` | `0x00453E20` | `0x00454190` | `BUILDINGCLASS_SAVE_LOAD` §1 |
| `DisplayClass` (LayerClass list) | `0x004AE6F0` | (paired Save) | `LAYER_CLASS_GHIDRA_REPORT` §5b |
| `DynamicVectorClass` | `0x00551B90` | `0x00551B20` | `SAVELOAD_LOGIC_…` §2 |

---

## 5. Swizzle (pointer-fixup) mechanism — the core primitive

- **`FUN_006CF240(&DAT_00B0C110, int* slot_addr)`** — register a pointer slot for fixup:
  `if slot==NULL return E_POINTER; old=*slot; if old==0 return 0 (NULL stays NULL);
  append (old, &slot) to the slot list; *slot = 0` (clears the dangling raw-dump pointer so it
  can't be deref'd before fixup). (`BUILDINGCLASS_SAVE_LOAD` §5, decomp.)
- **`FUN_006CF2C0(&DAT_00B0C110, old_this, new_this)`** — register a swap-map entry; called
  once per object inside `AbstractClass::Load`.
- **The fixup-apply pass** (inferred, not yet decompiled in corpus): `for (old, &slot) in
  slotlist: for (old_this, new_this) in swapmap: if old==old_this { *slot = new_this; break }`.
  Must run **after** every IPersistStream::Load completes (a target not yet loaded would miss
  its swap-map entry). **NEEDS-LIVE-REVERIFY** — locate the apply loop in the Load_Game tail.
- **Runtime caches are reset, not carried:** SoundEvent/VocHandle loop handles, light-source
  handles (`BuildingClass+0x614` zeroed), NavQueue rebuilt, etc. — OS-level/derived state is
  re-initialized on load even though it rides in the raw dump. (`BUILDINGCLASS_SAVE_LOAD` §6;
  `LIGHTSOURCE_LIFECYCLE_…` §8; `NAVCOM_NAVQUEUE_…` §3.)

---

## 6. Container format (high level)

- **Disk format:** OLE 2 Compound Document (`.SAV`), opened via `StgOpenStorage`, created via
  `StgCreateDocfile` (`ole32.dll`; import strings `0x0081086A`, `0x008108B0`, `0x008108C4`).
- **Per object:** one IStream named by `StringFromCLSID(object CLSID)` prefixed with the
  object's `AbstractClass::ID`. `OleSaveToStream(pPersistStream, pIStream)` writes the CLSID
  (`WriteClassStm`) then the IPersistStream::Save payload. Load mirrors via
  `OleLoadFromStream` → `CoCreateInstance`-style factory → `IPersistStream::Load`.
- **Versioning:** at the CLSID level only — a format-breaking change ships a new CLSID; there
  is no in-stream version field. **Corruption recovery:** none; first negative HRESULT aborts
  the object's load. (`BUILDINGCLASS_SAVE_LOAD` §7.)

---

## 7. Plug point (out-of-sim, not a PerTickUpdate rung)

Save/load is **menu/command-triggered**, OUT-OF-SIM. It does **not** appear on the verified
28-rung `LogicClass::PerTickUpdate` ladder (`LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md`) nor the
`TacticalClass_Draw @ 0x006D3D10` render pass. It runs from the in-game menu / command
dispatch (`frontier-input-command`), pausing the sim, walking the whole object graph once.

**Spine relationship (read-only):** save captures the **live order** of the Logic active
vector exactly as it stands at the save instant (the reveal-timing order — Rung-N membership
order), and load restores that order **verbatim** (forward append + swizzle), with **no
re-derivation and no sort**. So the only spine tie is a *data-capture* relationship to the
Logic active vector that the per-tick ladder mutates (`SAVELOAD_LOGIC_…` §1).

---

## 8. Outgoing edges (depends-on)

| → Service (slug) | Via (symbol) | Evidence |
|---|---|---|
| `abstract-object` | the whole Save/Load contract lives on the `Abstract`/`Object` vtable (`AbstractClass::Save @ 0x00410320` / `Load @ 0x00410380`, slots 5/6); `ObjectClass::Save @ 0x005F6250` / `Load @ 0x005F5E80` | `BUILDINGCLASS_SAVE_LOAD` §1/§2; `SAVELOAD_LOGIC_…` §2 |
| `logicclass` | Save_Game writes / Load_Game restores the Logic active vector `0x0087F778` (`DynamicVectorClass::Save 0x00551B20` / `Load 0x00551B90`); Clear_Scene resets it via Logic vtable `+0x0C` | `SAVELOAD_LOGIC_…` §1/§2/§3 |
| `factory-house` | Save_Game iterates `g_HouseClass_Array` and `OleSaveToStream`s each house (`HouseClass::Save @ 0x00504080`, `Load @ 0x00503040`); house economy/result bytes round-trip via the raw dump | `HOUSE_RESULT_BYTES_…` §3.1/§3.2 |
| `cell-map` | Load_Game restores the Map singleton (`ECX=0x87F7E8; CALL 0x00581F50`); per-cell `CellClass::Save @ 0x00483C10` (stub-grade) | `SAVELOAD_LOGIC_…` §5; stub H2 (cell save addr NEEDS-LIVE-REVERIFY) |
| `frontier-render-layer` | `DisplayClass::Load @ 0x004AE6F0` / paired Save persist the 5 `g_DisplayLayers` z-sorted draw vectors (`VectorClass::Load 0x00551B90` per layer) | `LAYER_CLASS_GHIDRA_REPORT` §5b |
| `mission-radio` | RadioHistory is **not** persisted (negative fact: no 3-slot history serialization); but mission state rides in the techno raw dump | `RADIOHISTORY_READ_USE_SCAN_…` §9 |
| `frontier-super` | per-house `SuperClass` instances persist via the IPersistStream path (`SuperWeaponTypeClass::Load @ 0x006CE800` deserialize-ctor) | `SUPERWEAPON_TYPE_CLASS_…` Q3 |
| `frontier-input-command` | Save/Load Game is invoked from the in-game menu/command dispatch (out-of-sim trigger) | `_frontier.md` I1 (menu/command path); §7 |

> **"Depends on ALL studied services" (stub H2):** literally true at the *data* level — the
> raw-dump scheme serializes whatever in-memory fields each service owns, so every service's
> persisted state is a column of the save format. The edges above are the **structural**
> dependencies (who owns the orchestrator, the vector, the vtable contract); the long-tail
> per-service field dependencies are implicit in the raw dump and not enumerated here.

---

## 9. Incoming edges (used-by)

| ← Service (slug) | Via (symbol) | Evidence |
|---|---|---|
| `frontier-input-command` | the menu Save/Load Game command triggers `FUN_0067D300` / `FUN_0067E730` | `_frontier.md` I1; §7 |
| every studied service (data producers) | each class implements its own `IPersistStream::Load`/`Save` slots that the orchestrator drives (`abstract-object`, `factory-house`, `cell-map`, `logicclass`, `techno-foot`, …) | §4 table; `BUILDINGCLASS_SAVE_LOAD` §2 |

(This service is a *sink* of state, not a producer feeding other runtime systems — its only
runtime consumer is the input/menu layer that invokes it.)

---

## 10. Active-in-YR / TS-legacy

- **Active in YR — YES.** Standard `.SAV` Save/Load Game in campaign and skirmish. The
  orchestrator (`FUN_0067D300`/`FUN_0067E730`), the OLE docfile container, the IPersistStream
  per-class slots, and the `DAT_00B0C110` swizzle DB are all live, current YR code.
- **No TS-legacy dead path identified in the core scheme.** The OLE/IPersistStream + raw-dump +
  pointer-fixup architecture is the live retail save mechanism; it is inherited from TS but is
  fully active in YR (the format is what retail `.SAV` files use).
- **Caveat — raw-dump round-trips bugs:** because Save commits whatever bytes are in the
  instance (uninitialized INI fields included) and Load reads them back verbatim, any in-memory
  uninitialized-field bug survives a save/load cycle unchanged (`BUILDINGCLASS_SAVE_LOAD` §8).
  A Rust re-implementation that serializes *semantic* fields avoids this, but must then match
  the runtime-cache **reset** discipline (§5) exactly.

---

## 11. Remaining uncertainty / follow-ups

1. **Ghidra unreachable this session** — re-pull `FUN_0067D300`, `FUN_0067E730`,
   `FUN_006851F0` bodies live to confirm exact stream order and locate the **fixup-apply loop**
   (currently inferred, §5). **NEEDS-LIVE-REVERIFY.**
2. **`CellClass::Save @ 0x00483C10`** is stub-grade (not re-cited in corpus this session) —
   verify the per-cell Save slot, or confirm cells persist only via the Map-singleton load
   (`0x00581F50`).
3. **`ObjectClass+0x98` logic-membership byte is NOT serialized** (`SAVELOAD_LOGIC_…` §3.4):
   restored objects are present in the Logic vector but no traced path writes `+0x98` on load.
   What sets it (most-derived load ctor / post-remap finalization / zero-alloc + other fixup)
   is **DEFERRED** (OQ-SL-007). Rust must derive membership from restored vector presence to
   avoid stale-pointer / double-add hazards regardless.
4. **`0x00887324` save object** role unspecified — identify what it serializes.
5. **`GetSizeMax`/`vtable[12]` byte-count puzzle** (`BUILDINGCLASS_SAVE_LOAD` §8 "Dump size"):
   confirm the actual raw-dump byte count (`vtable[12]` appears to return the AbstractType enum,
   not a size — likely a separate slot supplies the byte count).

---

## 12. Scale flags (30-player / 20k-unit target)

- The fixup pass is **O(slots × objects)** with a linear swap-map scan — fine at gamemd scale,
  but at 20k objects × many pointer slots each this is quadratic. A Rust port should index the
  swap-map by old-pointer (hash map) for O(slots) fixup; the *behavior* (every old→new pointer
  resolved exactly once, NULL stays NULL) is the contract, not the linear scan.
- `g_HouseClass_Array` is sized for 8 in gamemd; at 30 players the save must serialize the full
  house count (Save_Game already writes a leading count, so the format generalizes) and restore
  in the same registration order.

---

## 13. Sources

- `docs/research/SAVELOAD_LOGIC_ACTIVE_VECTOR_RECONSTRUCTION_GHIDRA_REPORT.md` (orchestrator
  `FUN_0067D300`/`FUN_0067E730`/Clear_Scene, Logic vector serialize, swizzle helper, `+0x98`
  negative fact) — primary.
- `docs/research/BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md` (IPersistStream contract, raw-dump +
  fixup scheme, `DAT_00B0C110`/`FUN_006CF240`/`FUN_006CF2C0`, OLE container, edge cases) — primary.
- `docs/research/HOUSE_RESULT_BYTES_SAVE_LOAD_PERSISTENCE_1F6_1F7_1F8_GHIDRA_REPORT.md` §3
  (Save_Game iterates `g_HouseClass_Array` + `OleSaveToStream`; HouseClass slots `+0x14`/`+0x18`).
- `docs/research/BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md` §29 (OLE docfile container summary).
- `docs/research/LAYER_CLASS_GHIDRA_REPORT.md` §5b (`DisplayClass::Load @ 0x004AE6F0`).
- `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md` §3.5 (ObjectClass
  save/load field set).
- `docs/research/SUPERWEAPON_TYPE_CLASS_GHIDRA_REPORT.md` Q3; `RADIOHISTORY_READ_USE_SCAN_GHIDRA_REPORT.md` §9;
  `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md` §8; `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` §3.
- `docs/research/LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` (plug-point confirmation: not on the rung ladder).
- `docs/research/core-services-map/_frontier.md` §H2 (seed stub).
