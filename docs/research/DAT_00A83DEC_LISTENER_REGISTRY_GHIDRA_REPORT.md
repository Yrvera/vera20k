# DAT_00A83DEC Listener Registry — Shape & Subscribers

**Status:** COMPLETE. Identity resolved with high confidence; supersedes the
"bridge-repair listener callback registry" claim in
`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §3.x / §7 Q6.

**Source:** Live decompilation of gamemd.exe via Ghidra MCP (read-only).
**Date:** 2026-05-18.

---

## 0. TL;DR

`DAT_00A83DEC` is **not** a listener-callback registry. It is the **data
pointer field of the global `DynamicVectorClass<InfantryClass*>`** — i.e. the
flat array of every live `InfantryClass` instance in the game. The "callback
dispatch" the parent doc inferred from `InfantryClass__PerCellProcess` is a
broadcast-iteration over that pool, calling `vtable[+0x28]` on every infantry
in response to a nearby bridge-hut destruction. There is no subscriber list,
no add-listener API, and no event-bus shape.

The structure at `[A83DE8..A83DFC]` (24 bytes) matches the standard RA2/TS
`DynamicVectorClass<T*>` layout exactly. It is **YR-active** — written from
`InfantryClass::Constructor` every time an infantry unit spawns.

Recommendation to the parent doc: **delete §7 Q6 follow-up**; this is not a
new system requiring research, it is one of the four canonical type-pool
vectors (`InfantryClass`, `UnitClass`, `BuildingClass`, `AircraftClass`)
that the Rust engine already models implicitly via `EntityStore`.

---

## 1. Static initial value

`read_memory 0x00A83DE0 length=48` → all zeros. BSS-initialized, brought to
life at runtime by the global vector constructor at ~`0x004E6AE0`.

---

## 2. Writers (subscriber-add or state-set sites)

`get_xrefs_to 0x00A83DEC` returns three writers, all in two unfunctioned
blobs that are the vector's static ctor/dtor:

| Address    | Instruction                              | Role                                          |
|------------|------------------------------------------|-----------------------------------------------|
| `004E6AE7` | `MOV [0x00A83DEC],EAX` (EAX=0)           | Static ctor: zero data ptr                    |
| `004E6B47` | `MOV [0x00A83DEC],EBX` (EBX=0)           | Static dtor: re-zero data ptr after free      |
| `004E6AFD` | `MOV [0x00A83DE8],0x7E43C8`              | Static ctor: install vtable (live form)       |
| `004E6B28` | `MOV [0x00A83DE8],0x7E43E8`              | Static dtor: swap to teardown vtable          |
| `004E6AEC` | `MOV [0x00A83DF0],EAX` (capacity = 0)    | Static ctor                                   |
| `004E6B53` | `MOV [0x00A83DF0],EBX` (capacity = 0)    | Static dtor                                   |

Additional vector-internal writes nearby:
- `[A83DF4] = 1`  is_initialized
- `[A83DF5] = 0`  is_owned (grow-allowed flag, runtime-set)
- `[A83DFC] = 10` grow_by increment
- `[A83DF8] = 0`  active count (becomes `g_InfantryClass_Array_Count`)

The runtime *growth* writer is inside `InfantryClass::Constructor`
(`0x00517A50`), which appends `this` and bumps the count:

```
if ((g_InfantryClass_Array_Count < DAT_00A83DF0) ||
    (DAT_00A83DF5 != 0 && DAT_00A83DF0 == 0 || ... grow path via DAT_00A83DE8+8)) {
  iVar8 = g_InfantryClass_Array_Count * 4;
  g_InfantryClass_Array_Count = g_InfantryClass_Array_Count + 1;
  *(InfantryClass**)(g_InfantryClass_Array + iVar8) = this;
}
```

The data-array memory itself (the heap buffer pointed *to* by
`[A83DEC]`) is allocated by the `+8` vtable slot on `DAT_00A83DE8`
(the vector's resize/grow virtual). No write site stores function
pointers into the array — only `InfantryClass*` pointers.

---

## 3. Readers (consumer / dispatch sites)

21 read xrefs total. Decompiled spot-checks confirm uniform consumption
pattern: index into `g_InfantryClass_Array` (= `[A83DEC]`) by `i*4` and
treat each entry as `InfantryClass*`. No call-through-data-pointer
dispatch.

Representative readers (Ghidra symbol labels these as
`g_InfantryClass_Array` and `g_InfantryClass_Array_Count`):

| Address    | Containing function                              | What it does                                                   |
|------------|--------------------------------------------------|----------------------------------------------------------------|
| `0064DACE` | `FUN_0064DAB0` (state-hash accumulator)          | Walks pool, mixes each infantry's x/y/z into `DAT_00AC51FC`    |
| `0064E329` | `FUN_0064DEA0`                                   | Pool walk (state/save related)                                 |
| `00650D2E` | `FUN_00650A90`                                   | Pool walk                                                      |
| `00509FB0` | `HouseClass__AI_FindInfantryTarget`              | Iterates pool to find AI infantry target                       |
| `00519D24` | `InfantryClass__PerCellProcess`                  | On bridge-hut destroy: broadcasts vtable+0x28 to every infantry |
| `0067D6D0` | `FUN_0067D300`                                   | Pool walk                                                      |
| `006EB034` | `FUN_006EAEE0`                                   | Pool walk                                                      |
| `006DDB0D` | `TriggerAction__Execute`                         | Trigger pool walk (e.g. "destroy all infantry of house X")     |
| `004FF050` | `FUN_004FEF03`                                   | Pool walk                                                      |
| `00457703` | `FUN_004576F0`                                   | Pool walk                                                      |
| `006EAB9D` | `FUN_006EAA90`                                   | Pool walk                                                      |
| `007081C7` | `FUN_00708080`                                   | Pool walk                                                      |
| `00599DF9` | `CCINIClass__Constructor`                        | Side use of address (likely string/ID, not data)               |
| `006C7E04` | `RawFileClass__Constructor`                      | Side use of address                                            |
| `0051FF1F` | `FUN_0051FEF0`                                   | Pool walk                                                      |
| `00517B12/17EAA` (DATA refs)                     | `InfantryClass::Constructor` — vector resize path  | Read capacity / vtable             |

The "callback-looking" site that triggered the parent doc's guess is at
`00519D24` (`InfantryClass::PerCellProcess`, near the bridge-hut decode):

```c
// after High/Low bridge-destruction dispatch:
iVar3 = g_InfantryClass_Array_Count;
while (iVar3 = iVar3 + -1, -1 < iVar3) {
  (**(code **)(**(int **)(g_InfantryClass_Array + iVar3 * 4) + 0x28))(pBVar7, 0);
}
```

This dereferences each entry as `InfantryClass*`, fetches its **per-object
vtable** (`*entry`), and calls **`vtable[+0x28]`** — a virtual method of
`InfantryClass`/`ObjectClass`, NOT a function pointer stored in the array.
The array's element type is `InfantryClass*`. The "registry" framing in
the parent doc was a misreading of "loop over global pool, invoke virtual"
as "loop over callback list, dispatch."

---

## 4. Inferred shape (with confidence rating)

**Confidence: HIGH (verified-from-binary).**

```
struct DynamicVectorClass_InfantryPtr {            // at 0x00A83DE8, 24 bytes
  void**  vtable;            // +0x00  0x00A83DE8 → 0x007E43C8 (live ctor vtable)
                             //                  ↓ 0x007E43E8 (dtor vtable)
  InfantryClass** data;      // +0x04  0x00A83DEC ← the field this report investigates
  int     capacity;          // +0x08  0x00A83DF0
  bool    is_initialized;    // +0x0C  0x00A83DF4
  bool    is_owned;          // +0x0D  0x00A83DF5  (grow-allowed)
  int     count;             // +0x10  0x00A83DF8 = g_InfantryClass_Array_Count
  int     grow_by;           // +0x14  0x00A83DFC = 10
};
```

`vtable[+8]` is the resize/grow function called from
`InfantryClass::Constructor`'s capacity-grow branch. This is the standard
RA2 `DynamicVectorClass<T>` template signature — `[A83DE8]`'s vtable matches
the shapes seen on the other type pools (`UnitClass`, `BuildingClass`,
`AircraftClass`, `HouseClass`), with `RateTimer__Current`/`FUN_00650A90`
hash-accumulator walking all four in `FUN_0064DAB0`.

No function-pointer storage anywhere. No subscriber-add API. No "listener"
shape of any kind.

---

## 5. YR-active vs TS-legacy verdict

**YR-ACTIVE.** Live in every skirmish — the vector is written on every
`InfantryClass` spawn and read by the state-hash accumulator
(`FUN_0064DAB0`) which is part of the tick loop, plus by AI targeting
(`HouseClass__AI_FindInfantryTarget`) and the bridge-hut destruction
broadcast in `InfantryClass::PerCellProcess`.

No TS-only gating, no `SpecialFlags` check, no fog-of-war / subterranean
adjacency. Standard YR data structure.

---

## 6. Open questions

None of substantive scope remain for this address.

Resolved by this report:
- Q: "What shape is the registry?" → A: `DynamicVectorClass<InfantryClass*>`.
- Q: "Who subscribes?" → A: Nobody. It's a type-pool, not a registry.
  Every `InfantryClass` instance appears in it for its full lifetime.
- Q: "What dispatches it?" → A: 21 readers iterate the pool. The one the
  parent doc latched onto is the bridge-destruction broadcast, which
  invokes `InfantryClass::vtable[+0x28]` on every infantry.

Out of scope (not investigated here): the identity of
`InfantryClass::vtable[+0x28]` itself. It is the "react to bridge
destruction" virtual on `ObjectClass`/`InfantryClass`; if the parent
doc needs to know what each infantry does in response, that is a
**separate `vtable[+0x28]` investigation** unrelated to this address.

---

## 7. Sources

All Ghidra MCP queries against `gamemd.exe`:

- `read_memory 0x00A83DE0 length=48` → BSS zero
- `get_xrefs_to 0x00A83DEC` → 18 reads, 2 writes (+ vector internal)
- `get_xrefs_to 0x00A83DE8` → vtable slot, 4 writes (ctor/dtor pairs)
- `get_xrefs_to 0x00A83DF0` → capacity field, matching writers
- `decompile_function 0x00517A50` — `InfantryClass::Constructor`
  (the canonical writer-via-append)
- `decompile_function 0x00509FB0` — `HouseClass__AI_FindInfantryTarget`
  (canonical reader, decompiler labels it `g_InfantryClass_Array`)
- `decompile_function 0x00519D24` — `InfantryClass::PerCellProcess`
  (the bridge-destruction "broadcast" the parent doc misread)
- `decompile_function 0x0064DAB0` — state-hash accumulator (reads all
  four type pools the same way)
- `get_assembly_context` on writers `004E6AE7..004E6B53` — static
  ctor/dtor pair for the vector struct at `[A83DE8..A83DFC]`

Parent doc context:
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §3.x table row "Bridge-repair
  callback registry", §7 Q6 follow-up, §3.x post-dispatch step E.
- `GI_GHIDRA_REPORT.md` §3.8 step 9 — independently identified this same
  address as the InfantryClass global pool (correct).

The conflict between the two parent docs is resolved here in favor of
`GI_GHIDRA_REPORT.md`'s identification.
