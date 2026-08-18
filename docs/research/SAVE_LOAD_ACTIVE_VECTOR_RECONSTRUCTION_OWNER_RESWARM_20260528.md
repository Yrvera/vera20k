# Save/Load Active Vector Reconstruction Owner - Reswarm 2026-05-28

**Address(es):** `FUN_0067d300` save stream owner, `FUN_0067e730` load stream owner, `FUN_00551B20` vector save helper, `FUN_00551B90` vector load helper, swizzle registrar `FUN_006CF240`, `ObjectClass::Save @ 0x005F6250`, `ObjectClass::Load @ 0x005F5E80`
**Investigation Mode:** exhaustive-slice for the save/load active-vector owner; final status is PARTIAL because the `Object+0x98` post-load membership-byte owner was not verified.
**Claimed Scope:** who serializes/loads the `LogicClass` active-object vector during savegame restore, what order is preserved by that vector path, and which attractive Rust shortcuts are ruled out.
**Non-Scope:** ordinary `ObjectClass::Reveal -> FUN_0055BAA0` registration, direct non-`Reveal` registration callers, death/uninit ordering, ownership transfer ordering, and class-specific AI bodies.
**Confidence:** High for vector save/load owner and order; Medium for the negative `Object+0x98` rebuild result; Low for any speculation about why the membership byte is coherent after load.
**Active in YR:** Yes for the save/load stream path and `LogicClass` vector helpers. Evidence: `FUN_0067e730` is the load stream owner that starts by clearing game state, initializes side/theater data, loads stream sections, and calls the `LogicClass` vector load helper at `0x0067E8D2..0x0067E8D8`; `FUN_0067d300` is the matching save stream owner and calls the vector save helper at `0x0067D435..0x0067D43A`.

## 0. Working Notes

Target question: who rebuilds `Object+0x98` and the `LogicClass` active-object vector after savegame/load restore, and in what order?

Non-goals: re-proving ordinary reveal registration, re-proving `ObjectClass::Save`/`Load` negative facts unless contradicted, and investigating unrelated lifecycle registration gaps.

Evidence needed to mark COMPLETE: binary-backed owner/caller path for post-load vector reconstruction and `Object+0x98` restoration, or a bounded proof that one side remains unresolved, plus Rust-facing handoff and do-not-do notes.

Stop conditions: exact owner/order resolved with decompile plus xref/assembly evidence, or Ghidra function boundaries/call paths prevent proof and the report records a precise Remaining Uncertainty.

## 1. Overview

The native active-object vector is not rebuilt from sorted object storage after a savegame load. `FUN_0067d300` serializes the `LogicClass` vector itself by calling `FUN_00551B20` with `ECX=0x87F778`, and `FUN_0067e730` reloads the same vector by calling `FUN_00551B90` with `ECX=0x87F778`.

The vector helper preserves saved vector order by reading the saved count, appending each saved pointer value into the vector in stream order, and registering each vector slot with the swizzle manager. The unresolved part is the object-local membership byte: this slot did not verify a post-load owner that re-sets `Object+0x98` for vector members.

## 2. Class Layout / Key Offsets

| Field | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `LogicClass+0x04` | pointer array for active-object vector | `FUN_00551B20` writes entries from `[ECX+0x04]`; `FUN_00551B90` appends into `[ESI+0x04]` | Yes |
| `LogicClass+0x08` | vector capacity | `FUN_00551B90 @ 0x00551BE6` reads `[ESI+0x08]` before append/grow | Yes |
| `LogicClass+0x0D` | vector owns/allocated storage flag | `FUN_00551B90 @ 0x00551BF0..0x00551C10` grow gate | Yes, conditional on capacity |
| `LogicClass+0x10` | vector count | `FUN_00551B20 @ 0x00551B38` saves count; `FUN_00551B90 @ 0x00551C12..0x00551C1B` increments count | Yes |
| `LogicClass+0x14` | grow step | `FUN_00551B90 @ 0x00551BFB..0x00551C0B` grow path | Yes, conditional on capacity |
| `ObjectClass+0x98` | logic membership byte used by registration/removal helpers | prior helper report; `FUN_0055BAA0 @ 0x0055BAA5..0x0055BAC6`; remover `0x0055BAE0` | Yes outside save/load; post-load owner unresolved |

## 3. Core Logic

### 3.1 Save Owner

`FUN_0067d300` writes the save stream in a fixed global order. After earlier global/type sections and before saving `g_Tactical`, it calls `FUN_00551B20` with `ECX=0x87F778`. Assembly context at `0x0067D435..0x0067D43A` is `MOV ECX,0x87F778; CALL 0x00551B20`. Active in YR: Yes; this is the save stream owner and the same singleton used by the tick scheduler/reveal path.

`FUN_00551B20` saves the vector count first, then writes each pointer slot from `LogicClass+0x04` in ascending index order while `EDI < saved_count`. Assembly context:

- `0x00551B38..0x00551B4B`: loads `[EBX+0x10]` and writes a 4-byte count through the stream vtable slot `+0x10`.
- `0x00551B5C..0x00551B78`: writes `*(LogicClass+0x04 + index*4)` for each index and loops while `index < count`.

This is order-preserving serialization of the current active vector; it is not a sorted object-array serialization. Active in YR: Yes.

### 3.2 Load Owner

`FUN_0067e730` starts by calling `FUN_006851F0`, which clears existing game state and calls the LogicClass clear slot on `0x87F778`. Later, before loading `g_Tactical`, it calls `FUN_00551B90` with `ECX=0x87F778`. Assembly context at `0x0067E8CD..0x0067E8D8` is `CALL 0x00581F50; MOV ECX,0x87F778; PUSH ESI; CALL 0x00551B90`. Active in YR: Yes.

`FUN_00551B90` reads a 4-byte count, then loops over that count. For each stream entry it:

1. Reads one 4-byte saved pointer token.
2. Appends the token to the vector if capacity permits or growth succeeds.
3. Increments `LogicClass+0x10` and writes the token at the old count index.
4. After all entries, registers each vector slot address with `FUN_006CF240`.

Assembly evidence:

- `0x00551BAA..0x00551BC3`: stream read of the saved count.
- `0x00551BCF..0x00551BE4`: loop only runs for count greater than zero; read failure returns immediately.
- `0x00551BE6..0x00551C10`: capacity/grow gate.
- `0x00551C12..0x00551C22`: old count is used as append index, then count is incremented.
- `0x00551C2E..0x00551C4C`: every loaded slot is passed to `FUN_006CF240`.

`FUN_006CF240` registers a nonzero pointer token plus the address of the pointer slot into the swizzle table, then clears the slot to zero. Evidence: decompile of `FUN_006CF240` stores `old_pointer, slot_address` pairs and then writes `*slot = 0`. Active in YR: Yes; `ObjectClass::Load` uses the same swizzle registrar for object pointer fields.

### 3.3 Membership Byte Result

The load vector helper does not write `Object+0x98`; it only mutates `LogicClass+0x04/+0x10` and swizzle-table state. The already-settled contradiction check still holds: `ObjectClass::Save @ 0x005F6250` serializes selected object fields and jumps from byte `+0x90` to dwords `+0x9C/+0xA0/+0xA4`, with no observed `+0x98` write; `ObjectClass::Load @ 0x005F5E80` registers pointer fields `+0x30/+0x34/+0x38/+0x18/+0x88`, initializes two `VocHandle`s, and clears `+0xA8`, with no reveal/register call.

This slot did not find a verified post-load owner that restores the `Object+0x98` flag for active-vector members. Active in YR: Conditional/Unresolved for the byte restoration path; the vector load path is active, but the byte-owner path remains unverified.

## 4. INI Keys

No INI key gates the `LogicClass` vector save/load helper path in the inspected functions. Active in YR: Yes; this is savegame stream mechanics, not INI-driven behavior.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Save stream owner | Serializes `LogicClass` vector before `g_Tactical` and later object/type arrays | `FUN_0067d300`; assembly `0x0067D435..0x0067D43A` | Yes |
| Load stream owner | Clears old state, then loads the saved `LogicClass` vector before `g_Tactical` | `FUN_0067e730`; assembly `0x0067E8CD..0x0067E8D8` | Yes |
| Swizzle manager | Turns saved pointer tokens into pointer-location fixups and zeroes slots until resolution | `FUN_006CF240` decompile | Yes |
| Object load | Does not call reveal/register and does not write `+0x98` | `ObjectClass::Load @ 0x005F5E80` | Yes |
| Ordinary reveal registration | Already settled: appends through `FUN_0055BAA0` and sets `Object+0x98` only after insert success | prior helper report | Yes |

## 6. Current Rust Implementation Status

Static scan only; no Rust files were modified.

| Rust surface | Current shape | Delta |
|---|---|---|
| `src/sim/world/mod.rs:289` | `live_object_order: Vec<u64>` is serialized by default because it has `#[serde(default)]`, not `#[serde(skip)]`. | Broadly matches "persist vector order" better than rebuilding from sorted IDs, but lacks native swizzle/object-pointer semantics and object-local membership byte. |
| `src/sim/world/mod.rs:612` | `register_live_object` appends if ID is not already present. | Duplicate prevention is vector-scan based, not `Object+0x98` byte based. |
| `src/sim/world/mod.rs:622` | `live_object_order_snapshot` appends missing sorted `EntityStore` IDs after registered order. | DRIFT risk for parity judgments; native save/load uses saved vector order, not sorted fallback. |
| `src/sim/snapshot.rs:84` and `:107` | `GameSnapshot::save/load` serializes/deserializes the whole `Simulation`. | Current Rust likely persists `live_object_order`, but acceptance tests should prove no fallback sorted IDs are introduced after load. |
| `src/sim/world/mod.rs:796` | `rebuild_caches_after_load` restores skipped caches. | No evidence found that this should rebuild active-object order from `EntityStore`; native owner is save/load stream vector path. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0067d300` save owner call to `LogicClass` vector save | verified | decompile and assembly `0x0067D435..0x0067D43A` | none |
| `FUN_00551B20` vector save helper | verified | decompile and assembly `0x00551B38..0x00551B78` | none |
| `FUN_0067e730` load owner call to `LogicClass` vector load | verified | decompile and assembly `0x0067E8CD..0x0067E8D8` | none |
| `FUN_00551B90` vector load helper | verified | decompile and assembly `0x00551BAA..0x00551C4C` | none |
| `FUN_006CF240` swizzle registrar | verified | decompile of `FUN_006CF240` | final swizzle resolution timing remains out of this narrow slice |
| `ObjectClass::Save` contradiction check | verified | decompile and assembly `0x005F6250..0x005F6350` | none for scoped negative |
| `ObjectClass::Load` contradiction check | verified | decompile and assembly `0x005F5E80..0x005F5EFA` | none for scoped negative |
| post-load `Object+0x98` owner | touched-not-exhausted | byte-pattern searches for direct byte set found only known registration helper for object-shaped `MOV [ESI+0x98],AL`; global `0x87F778` xrefs show no second direct post-load pass | exact byte restoration owner remains unresolved |
| missed function boundary near `0x0055B8F0` | deferred | executable bytes loop object arrays and check `+0x98`, but Ghidra has no function boundary and read-only swarm rules forbid creating one | separate read-only/manual boundary investigation |

## 8. Open Questions - Final State

- `[RESOLVED] SLAV-001 - Is the active vector serialized directly? -> Yes; save owner calls `FUN_00551B20` with `ECX=0x87F778`, and the helper writes count then each pointer slot in vector order.` (evidence: `0x0067D435..0x0067D43A`, `0x00551B38..0x00551B78`)
- `[RESOLVED] SLAV-002 - Is the active vector rebuilt from sorted object storage on load? -> No evidence for a sorted rebuild in the load owner; load reads saved vector entries and appends them in stream order.` (evidence: `0x0067E8D2..0x0067E8D8`, `0x00551BAA..0x00551C22`)
- `[RESOLVED] SLAV-003 - Does vector load swizzle saved object pointers? -> Yes; after reading all entries it registers every vector slot with `FUN_006CF240`.` (evidence: `0x00551C2E..0x00551C4C`; `FUN_006CF240`)
- `[RESOLVED] SLAV-004 - Does `ObjectClass::Load` call reveal/register? -> No; it calls base load, registers pointer fields, initializes handles, and clears `+0xA8`.` (evidence: `0x005F5E80..0x005F5EFA`)
- `[RESOLVED] SLAV-005 - Does `ObjectClass::Save` serialize `+0x98`? -> No in the scoped ObjectClass save body; the observed sequence writes `+0x90`, then `+0x9C/+0xA0/+0xA4`.` (evidence: `0x005F62B4..0x005F6350`)
- `[DEFERRED] SLAV-006 - Who restores `Object+0x98` after save/load?` (category: `requires-different-system-context`; reason: vector load owner does not write it and no verified post-load byte owner was found; next-step-if-pursued: trace final swizzle-resolution completion and any post-load logic reactivation passes.)
- `[DEFERRED] SLAV-007 - Does the unresolved `0x0055B8F0` code participate in save/load reactivation?` (category: `bounded-cost-too-high`; reason: Ghidra has executable bytes but no function boundary and swarm read-only rules forbid creating one; next-step-if-pursued: manually bound and decompile in a non-swarm or approved labeling session.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native save/load persists the `LogicClass` active vector order directly. | `FUN_0067d300 @ 0x0067D435..0x0067D43A`; `FUN_00551B20 @ 0x00551B38..0x00551B78`; Active in YR: Yes | Rust `live_object_order` appears serialized with `Simulation`, but fallback snapshots append sorted missing IDs. | `src/sim/world/mod.rs:289`, `src/sim/world/mod.rs:622`, `src/sim/snapshot.rs:84` | Save/load must preserve active order as a first-class saved vector, not regenerate it from `EntityStore` sort. | Save a sim where reveal/order differs from stable ID order, load it, and assert the next logic-order consumer sees saved order exactly. | Do not let `live_object_order_snapshot` sorted fallback define post-load parity. |
| Native vector load reads count then appends saved entries in stream order and swizzles each slot. | `FUN_00551B90 @ 0x00551BAA..0x00551C4C`; `FUN_006CF240`; Active in YR: Yes | Rust has stable IDs, not raw pointer tokens/swizzling; delta is acceptable only if the effective active order and membership are byte-equivalent. | snapshot load/rebuild surfaces; future native savegame importer | Preserve exact vector order across deserialization and reject/repair missing IDs explicitly instead of silently sorting them in. | Corrupt/remove one saved active ID and verify the loader does not silently append all remaining entities sorted as if native behavior were known. | Do not treat "deterministic sorted repair" as native unless a binary repair path is proven. |
| `Object+0x98` post-load restoration is not proven by this slice. | `ObjectClass::Save @ 0x005F6250`; `ObjectClass::Load @ 0x005F5E80`; `FUN_00551B90` does not write object bytes; Active in YR: unresolved for byte owner | Rust has no object-local membership byte equivalent; current duplicate guard scans `Vec<u64>`. | future active-object membership model in `src/sim/world/mod.rs` / entity runtime state | Add a separate membership state only after the native byte restoration owner is verified; until then, tests should expose the unresolved gap. | Round-trip an active object, then conceal/despawn it after load; assert native-equivalent removal behavior once byte-owner evidence exists. | Do not claim parity from vector persistence alone; `+0x98` controls unregister/duplicate semantics. |

Proposed test names:

- `save_load_preserves_live_object_order_not_sorted_ids`
- `save_load_active_order_missing_member_does_not_sorted_repair`
- `post_load_conceal_uses_restored_logic_membership_byte`

## 10. Negative Facts / Do Not Do

- Do not rebuild active order after load by walking `EntityStore`/global objects sorted by ID. Evidence: native load owner calls `FUN_00551B90`, which reads saved vector entries in stream order; Active in YR: Yes.
- Do not rely on `ObjectClass::Load` to register active objects. Evidence: `0x005F5E80..0x005F5EFA` has no reveal/register call and only swizzle-registers selected object pointer fields; Active in YR: Yes.
- Do not rely on `ObjectClass::Save` to serialize `Object+0x98`. Evidence: save field sequence writes through `+0x90` then jumps to `+0x9C/+0xA0/+0xA4`; Active in YR: Yes.
- Do not treat loaded vector slots as immediately resolved object pointers. Evidence: `FUN_006CF240` records slot fixups and clears each slot to zero until swizzle resolution; Active in YR: Yes.
- Do not claim that Rust vector serialization alone proves parity. Evidence: native also has object-local `+0x98` duplicate/removal semantics from the helper/remover reports, and this byte's post-load owner remains unresolved; Active in YR: Yes for the byte semantics, unresolved for restore owner.

## 11. Remaining Uncertainty

- The exact post-load owner that restores or reconciles `Object+0x98` for active-vector members remains unresolved. The vector path itself does not write it.
- Final swizzle-resolution timing was not traced beyond `FUN_006CF240`; a follow-up should start from the swizzle manager's completion pass and ask whether it has object-class side effects.
- Ghidra has a missed/undefined function boundary near `0x0055B8F0` that appears to loop object arrays and check `+0x98`; swarm read-only rules prevented creating a function boundary, so its save/load relevance is unknown.

## 12. Stale Docs / Replacement Wording

- `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`: replace "The native post-load active-vector rebuild owner remains a required follow-up before implementing parity save/load order" with "The native save/load stream owner serializes and reloads the `LogicClass` active-object vector directly through `FUN_00551B20`/`FUN_00551B90` with `ECX=0x87F778`; the remaining follow-up is the post-load owner that restores or reconciles `Object+0x98` membership semantics."
- `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`: replace "Do not serialize Rust `live_object_order` as the final parity answer until the native save/load rebuild order is proved" with "Persisting Rust `live_object_order` is directionally closer to native than sorted-ID rebuild because gamemd saves and reloads the vector itself; it is still not a complete parity answer until `Object+0x98` post-load restoration semantics are verified."
- `docs/research/LOGICCLASS_LIVE_VECTOR_VS_RUST_ENTITY_PASSES_GHIDRA_REPORT.md`: replace "Save/load and replay reconstruction of active-list membership/order was not traced" with "Save/load active-list order is traced: `FUN_0067d300` saves and `FUN_0067e730` loads the `LogicClass` vector directly. Active-list membership byte restoration for `Object+0x98` remains untraced."

## Sources

- Ghidra decompile/assembly: `FUN_0067d300`, `FUN_0067e730`, `FUN_00551B20`, `FUN_00551B90`, `FUN_006CF240`, `FUN_006851F0`, `ObjectClass::Save @ 0x005F6250`, `ObjectClass::Load @ 0x005F5E80`.
- Prior docs: `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`, `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`, `docs/research/LOGICCLASS_LIVE_VECTOR_VS_RUST_ENTITY_PASSES_GHIDRA_REPORT.md`, `docs/research/OBJECTCLASS_GHIDRA_REPORT.md`.
- Rust static scan: `src/sim/world/mod.rs`, `src/sim/snapshot.rs`.
