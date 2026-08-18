# FUN_00551A30 Active-Order Maintenance Key - Reswarm 2026-05-28

**Address(es):** `FUN_00551A30 @ 0x00551A30`, active caller `Main_Tick @ 0x0055D360`, `ObjectClass::YSortComparator @ 0x005F6220`, `ObjectClass::GetYSort @ 0x005F6BD0`, `AnimClass::GetYSortWithAdjust @ 0x00422BC0`, `BuildingClass` YSort override `0x00449410`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact helper body, call-site vector identity, adjacent-pass key semantics, mutation surface, and Rust-facing consequence for the alleged LogicClass active-order blocker.  
**Non-Scope:** save/load/replay vector reconstruction, `FUN_0055BAA0`/`FUN_0055BAE0` registration/removal semantics, full display-layer lifecycle, complete per-class vtable census beyond representative verified overrides.  
**Confidence:** High for function body, caller vector identity, base key, representative overrides, and negative LogicClass conclusion.  
**Active in YR:** Yes. `Main_Tick` reaches `MOV ECX,0x008A0390; CALL 0x00551A30` on the standard late tick path before `LogicClass::PerTickUpdate`.

## 0. Investigation Contract

**Target question:** What exact key/virtual semantics does `FUN_00551A30` use for its adjacent order-maintenance pass, and does it mutate the `LogicClass` active vector, object fields, or only a sidecar vector?  
**Non-goals:** Do not re-prove save/load order, direct registration/removal caller families, or the main `LogicClass::PerTickUpdate` live-vector loop unless `FUN_00551A30` directly contradicts them.  
**Evidence needed to mark COMPLETE:** decompile plus assembly for `FUN_00551A30`, caller assembly proving the vector instance, decompile/assembly for `vtable+0xB8` key semantics, and Rust handoff with at least one test proposal.  
**Stop conditions:** stop when the in-scope helper, call vector, key, and mutation/non-mutation surfaces are resolved; defer only per-class override census or runtime visual fixture construction.

## 1. Overview

`FUN_00551A30` is not a `LogicClass` active-object scheduler pass. `Main_Tick` passes `ECX=0x008A0390`, which existing `LayerClass` research identifies as `g_DisplayLayers[2]`, the Ground display layer; `LogicClass::PerTickUpdate` is called later with `ECX=0x0087F778`.

The helper performs one stable forward adjacent-swap pass over that Ground-layer vector. It compares adjacent objects by their `vtable+0xB8` signed integer YSort key and swaps only when the later object has a smaller key. It does not append, remove, compact, deduplicate, write `ObjectClass+0x98`, or mutate object fields directly.

## 2. Class Layout / Key Offsets

| Offset / address | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `0x008A0390` | `g_DisplayLayers[2]`, Ground `LayerClass` instance passed to helper | `Main_Tick` assembly `0x0055DBC3..0x0055DBC8`; `LAYER_CLASS_GHIDRA_REPORT.md` | Yes |
| `0x0087F778` | `LogicClass` singleton passed to `LogicClass::PerTickUpdate`, not this helper | assembly `0x0055DC99..0x0055DC9E` | Yes |
| vector `+0x04` | `Items` pointer array read/written by adjacent swap | `FUN_00551A30` assembly `0x00551A41`, `0x00551A69..0x00551A73` | Yes |
| vector `+0x10` | `Count`, read at entry and reloaded at loop bottom | assembly `0x00551A37..0x00551A3D`, `0x00551A76..0x00551A7D` | Yes |
| object vtable `+0xB8` | virtual signed integer YSort key | helper assembly `0x00551A4B..0x00551A63`; `ObjectClass::GetYSort @ 0x005F6BD0` | Yes |
| `ObjectClass+0x98` | Logic active-list membership byte | not touched by helper; contrast with prior registration/removal docs | No for this helper |

## 3. Core Logic

Assembly-equivalent pseudocode:

```text
count_minus_one = signed(vector.Count - 1)
if count_minus_one <= 0:
    return

i = 0
do:
    next = vector.Items[i + 1]
    cur  = vector.Items[i]
    next_key = next.vtable[0xB8]()
    cur_key  = cur.vtable[0xB8]()
    if next_key < cur_key:
        vector.Items[i + 1] = cur
        vector.Items[i]     = next
    i += 1
while i < signed(vector.Count - 1)
```

Tiny details:

| Finding | Evidence | Active in YR |
|---|---|---|
| The call target is the Ground display layer, not `LogicClass`. | `0x0055DBC3 MOV ECX,0x8A0390; 0x0055DBC8 CALL 0x00551A30`; `0x0055DC99 MOV ECX,0x87F778; 0x0055DC9E CALL 0x0055AFB0` | Yes |
| Empty and one-element vectors return before reading `Items`. | `MOV EAX,[EBP+0x10]; DEC EAX; TEST EAX,EAX; JLE 0x00551A81` | Yes |
| The pass is one forward adjacent pass, not a full sort or fixed-point bubble loop. | single loop `0x00551A41..0x00551A7D`; no outer loop | Yes |
| `Items[i+1]` is keyed first, then `Items[i]`. | `ESI=Items[i+1]`, `EDI=Items[i]`; calls at `0x00551A4F` and `0x00551A5D` | Yes |
| Comparison is signed `next_key < cur_key`; equal keys do not swap. | `CMP [ESP+0x10],EAX; JGE 0x00551A76` | Yes |
| Count is not snapshotted for the whole pass; it is reloaded at the loop bottom. | `MOV EAX,[EBP+0x10]` at `0x00551A76` | Yes |
| Object pointers are not null-guarded before virtual dispatch. | unconditional `MOV EDX,[ESI]` and `MOV EAX,[EDI]` | Yes, relying on layer invariants |
| Only `Items` slots are written; no count/capacity/object-field writes exist. | negative instruction scan of `0x00551A30..0x00551A84` | Yes |

## 4. Key / Virtual Semantics

The key is not a hidden active-list priority field. `FUN_00551A30` calls each object's vtable slot `+0xB8`, the same comparator key used by Ground-layer sorted insert.

Verified key functions:

| Class / function | Key returned | Evidence | Active in YR |
|---|---|---|---|
| `ObjectClass::GetYSort @ 0x005F6BD0` | `GetRenderCoords().Y + GetRenderCoords().X` as signed int | decompile `0x005F6BD0`; `ObjectClass::YSortComparator @ 0x005F6220` | Yes |
| `ObjectClass::YSortComparator @ 0x005F6220` | compares only `vtable+0xB8` results; no secondary tiebreaker | decompile and assembly `0x005F6220..0x005F6243` | Yes |
| `AnimClass::GetYSortWithAdjust @ 0x00422BC0` | base `ObjectClass::GetYSort()` + `AnimClass+0x104` | decompile/assembly `0x00422BC0..0x00422BD1`; constructor copy `0x00422131..0x00422137` | Yes for `AnimClass` |
| `BuildingClass` override `0x00449410` | base `ObjectClass::GetYSort()` + `0x20` if `Type+0x16C5` byte set, minus `0x10` if `Type+0x16B7` byte set | decompile/assembly `0x00449410..0x0044943D`; vtable data xref `0x007E3F74` | Yes for `BuildingClass`; flag activation is data-conditional |

The `AnimClass+0x104` value comes from `AnimTypeClass+0x340`; `AnimClass` construction copies it when an anim type is present. INI `YSortAdjust=` and building anim `*YSort=` keys exist in `artmd.ini`/`art.ini` and feed related runtime fields, but `FUN_00551A30` itself reads no INI key and only consumes the virtual return.

## 5. Integration Points

| Point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| direct caller | only caller returned by Ghidra is `Main_Tick @ 0x0055D360` | `get_function_callers 0x00551A30` | Yes |
| call order | helper runs after replay/record side work and before `LogicClass::PerTickUpdate` | `Main_Tick` decompile; assembly `0x0055DBC3..0x0055DC9E` | Yes |
| replay playback | replay playback render path falls through to the helper | `Main_Tick` decompile; prior replay reswarm | Conditional: replay playback |
| early scenario display branch | `ScenarioClass+0x62C` branch returns before the helper | `Main_Tick` decompile | Conditional |
| Ground-layer ordering relation | sorted insert and this repair pass both use `vtable+0xB8` YSort | `0x00551A90`, `0x005F6220`, `0x00551A30` | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Delta for this slice |
|---|---|---|
| `src/sim/world/mod.rs` `live_object_order` | stores Rust surrogate for `LogicClass`-like active-object order | `FUN_00551A30` does not justify sorting or adjacent-repairing this vector |
| `src/sim/world/mod.rs::live_object_order_snapshot` | appends sorted `EntityStore` IDs that are missing from the explicit order | native helper is not a sorted fallback or repair for missing active logic members |
| `src/sim/passenger.rs` | consumes `live_object_order_snapshot` for garrison owner/order behavior | must continue to model logic order, not display YSort order |
| `src/app_render/build_instances.rs` | rebuilds/sorts render instance lists each frame | exact Ground-layer render parity would need a persistent layer vector plus one-pass repair, not full resort, when that surface is prioritized |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00551A30` body | verified | decompile and disassembly `0x00551A30..0x00551A84` | none |
| call vector identity | verified | `0x0055DBC3..0x0055DBC8`; `LAYER_CLASS_GHIDRA_REPORT.md` | none |
| LogicClass non-target | verified | `0x0055DC99..0x0055DC9E` | none |
| `vtable+0xB8` base semantics | verified | `0x005F6BD0`, `0x005F6220` | none |
| Anim override | verified representative | `0x00422BC0`, `0x00422131..0x00422137` | none for this slice |
| Building override | verified representative | `0x00449410..0x0044943D`, vtable xref `0x007E3F74` | exact field/key names for `+0x16C5` not decoded here |
| full per-class `+0xB8` census | deferred | xrefs to `0x005F6BD0`, `0x00422BC0`, `0x00449410` | separate vtable census if render key parity requires every class |
| Rust scan | touched-not-exhausted | `rg live_object_order`, `rg build_instances` | implementation design |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-551A30-MK-001 - Which vector does Main_Tick pass? -> Ground display `LayerClass` at `0x008A0390`.` (evidence: `0x0055DBC3..0x0055DBC8`; `LAYER_CLASS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-551A30-MK-002 - Is the LogicClass active vector passed? -> No; `LogicClass` is passed later to `0x0055AFB0` with `ECX=0x0087F778`.` (evidence: `0x0055DC99..0x0055DC9E`)
- `[RESOLVED] OQ-551A30-MK-003 - What key does the helper compare? -> each object's virtual `vtable+0xB8` signed YSort key.` (evidence: `0x00551A4B..0x00551A63`; `0x005F6BD0`)
- `[RESOLVED] OQ-551A30-MK-004 - Does the helper fully sort? -> No, one forward adjacent pass only.` (evidence: `0x00551A41..0x00551A7D`)
- `[RESOLVED] OQ-551A30-MK-005 - Are equal keys stable? -> Yes, equal keys take the `JGE` no-swap branch.` (evidence: `0x00551A63..0x00551A67`)
- `[RESOLVED] OQ-551A30-MK-006 - Does it mutate object fields? -> No direct object-field writes; it only virtual-calls objects and swaps vector slots.` (evidence: `0x00551A30..0x00551A84`)
- `[RESOLVED] OQ-551A30-MK-007 - Does it mutate `ObjectClass+0x98`? -> No.` (evidence: negative scan; prior `FUN_0055BAA0`/`FUN_0055BAE0` reports)
- `[RESOLVED] OQ-551A30-MK-008 - Does the base key include Z? -> No; base `ObjectClass::GetYSort` returns render `X + Y`.` (evidence: `0x005F6BD0`)
- `[RESOLVED] OQ-551A30-MK-009 - Can `AnimClass` change the key? -> Yes, adds `AnimClass+0x104` copied from `AnimTypeClass+0x340`.` (evidence: `0x00422BC0`; `0x00422131..0x00422137`)
- `[RESOLVED] OQ-551A30-MK-010 - Can `BuildingClass` change the key? -> Yes, adds/subtracts small constants based on two building-type bytes.` (evidence: `0x00449410..0x0044943D`)
- `[RESOLVED] OQ-551A30-MK-011 - Is the helper active in replay playback? -> Conditional yes; replay playback falls through before PerTick.` (evidence: `Main_Tick`; `REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md`)
- `[RESOLVED] OQ-551A30-MK-012 - What should Rust not infer? -> Do not sort or repair `live_object_order` from this helper because it is display-layer state.` (evidence: call vector identity plus Rust scan)
- `[DEFERRED] OQ-551A30-MK-013 - Complete every class `+0xB8` override.` (category: `out-of-scope`; reason: the target only needs helper key semantics and representative active overrides; next-step-if-pursued: vtable census for all drawable classes.)
- `[DEFERRED] OQ-551A30-MK-014 - Runtime scene that exposes multi-inversion one-pass render transient.` (category: `needs-runtime-debugger`; reason: static semantics are proven, but a visual fixture requires runtime layer contents.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `FUN_00551A30` mutates Ground display `LayerClass` order, not `LogicClass` active-object order. Active in YR: Yes. | `0x0055DBC3..0x0055DBC8`; `0x0055DC99..0x0055DC9E` | Rust `live_object_order` is a LogicClass-like gameplay order surface | `src/sim/world/mod.rs`, `src/sim/passenger.rs` | Keep logic scheduler order independent from render YSort order. | Two active gameplay objects with `live_object_order=[B,A]` and render keys `A<B`; the future scheduler still calls `B` then `A`. Proposed test: `fun_551a30_does_not_sort_logic_live_object_order` | High: sorting gameplay order by render YSort changes same-tick AI/garrison/projectile behavior. |
| The display helper performs one stable adjacent pass by signed `vtable+0xB8` key. Active in YR: Yes. | `0x00551A30..0x00551A84`; `0x005F6220`; `0x005F6BD0` | Rust render appears to rebuild and sort render lists rather than maintain native layer vectors | future render/display layer state, `src/app_render/build_instances.rs` | If exact render traversal is pursued, preserve persistent Ground-layer order and apply one native adjacent pass at the correct tick point. | Initial Ground keys `[40,30,20,10]` produce one-pass result `[30,20,10,40]`, not an arbitrary full sort. Proposed test: `ground_layer_y_sort_repair_is_single_adjacent_pass` | Medium/high for pixel parity: full sort hides native transient and tie-order states. |
| Equal YSort keys are stable and have no secondary ID/type tiebreaker in this helper. Active in YR: Yes. | `JGE` at `0x00551A67`; `ObjectClass::YSortComparator @ 0x005F6220` | Rust sort stability/tiebreakers not audited here | render sorting/merge surfaces | Preserve existing layer order for equal virtual keys when modeling native layer ordering. | Three same-key Ground-layer objects retain relative order through the prepass. Proposed test: `ground_layer_y_sort_equal_keys_keep_existing_order` | Do not add stable-id, object-id, class, or entity-key tie breakers without binary evidence. |

## 10. Negative Facts / Do Not Do

- Do not implement `FUN_00551A30` as a `LogicClass` active-vector sorter. Evidence: call site passes `0x008A0390`; `LogicClass` is `0x0087F778` later.
- Do not use this helper to justify sorted stable-ID fallback in `live_object_order_snapshot`. Evidence: helper operates on display layer `Items`, not entity storage or active membership.
- Do not set/clear `ObjectClass+0x98` in a `FUN_00551A30` equivalent. Evidence: no object writes in `0x00551A30..0x00551A84`.
- Do not implement the helper as a full sort. Evidence: one forward loop, no outer loop.
- Do not add a secondary tiebreaker for equal keys. Evidence: `JGE` skips swap for equal `next_key == cur_key`.

## 11. Remaining Uncertainty

- Full per-class `vtable+0xB8` override census is deferred; representative active overrides prove the key is virtual YSort, not a hidden field.
- Exact INI names for the two `BuildingClass` type bytes used by `0x00449410` were not decoded in this slot.
- No runtime visual fixture was captured for a multi-inversion one-pass transient; static helper semantics are complete.

## 12. Stale Docs / Follow-up Docs

- `docs/research/REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md`: replace "single adjacent pass over `LogicClass+0x04` entries" with "`FUN_00551A30` is a single adjacent pass over the Ground display `LayerClass` at `0x008A0390`; `LogicClass::PerTickUpdate` receives `0x0087F778` later. It can change render-layer traversal order, not LogicClass active-object AI order."
- `docs/research/OBJECT_ACTIVE_VECTOR_SAVE_LOAD_REBUILD_OWNER_RESWARM_20260528.md`: replace OQ-OAVSL-015 with "`FUN_00551A30` is not a LogicClass active-vector maintenance pass. `Main_Tick` passes Ground display `LayerClass` `0x008A0390`; it performs one stable adjacent swap by virtual YSort key and has no active-vector save/load implication."
- `docs/research/OBJECT_TECHNO_LIFECYCLE_SHARED_STATE_SYSTEM_MODEL_SYNTHESIS.md`: add qualification after tail-append/order wording: "`ObjectClass` lifecycle reports govern LogicClass active-object order. The separately named `FUN_00551A30` is display-layer YSort repair and must not be used as evidence for gameplay scheduler reordering."

## Sources

- Ghidra decompile/read-only: `FUN_00551A30 @ 0x00551A30`, `Main_Tick @ 0x0055D360`, `ObjectClass::YSortComparator @ 0x005F6220`, `ObjectClass::GetYSort @ 0x005F6BD0`, `AnimClass::GetYSortWithAdjust @ 0x00422BC0`, `AnimClass::Constructor @ 0x00421EA0`, `BuildingClass` override `0x00449410`, `DynamicVector::SortedInsert @ 0x00551A90`.
- Ghidra disassembly/read-only ranges: `0x00551A30..0x00551A84`, `0x0055DBC3..0x0055DC9E`, `0x005F6220..0x005F6243`, `0x00422BC0..0x00422BD1`, `0x00449410..0x0044943D`, `0x00422131..0x00422137`.
- Ghidra caller query: `FUN_00551A30` caller is `Main_Tick @ 0x0055D360`.
- Research-index preflight: query `FUN_00551A30 active order maintenance vtable +0xB8 adjacent swap key`, anchors `0x00551A30`, `0x005F6220`, `0x005F6BD0`.
- Prior docs read: `FUN_00551A30_ACTIVE_ORDER_PREPASS_RESWARM_20260528.md`, `REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md`, `LAYER_CLASS_GHIDRA_REPORT.md`, `ANIMCLASS_DRAW_TRAVERSAL_LAYER_ORDERING_RESWARM_20260527.md`.
- Rust scan: `src/sim/world/mod.rs`, `src/sim/passenger.rs`, `src/app_render/build_instances.rs`.
