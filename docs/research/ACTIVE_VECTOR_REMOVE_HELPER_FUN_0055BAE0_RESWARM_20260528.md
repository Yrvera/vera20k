# Active Vector Remove Helper `FUN_0055BAE0` - Reswarm 2026-05-28

**Address(es):** `FUN_0055BAE0 @ 0x0055BAE0`, `ObjectClass::Conceal @ 0x005F4D30`, `ObjectClass::Destructor @ 0x005F3B80`, `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact active-vector unregister helper body, caller map, `Object+0x98` guard semantics, edge behavior, live-iteration implications, and Rust handoff.  
**Non-Scope:** re-proving `FUN_0055BAA0` insertion except direct comparison, every class-specific `UnInit`/destructor side effect, save/load runtime sampling of `Object+0x98`, and mutating Ghidra labels.  
**Confidence:** High for helper body, caller/xref map, edge behavior, and scheduler interaction; Medium for the NAME (not boundary) of the caller at `0x00435B7E` — it is now cleanly bounded inside `FUN_00435B70` (2026-05-29 re-verify), but the function is still unnamed.  
**Active in YR:** Yes. The helper is reached by standard `ObjectClass::Conceal`, by `ObjectClass` destructor fallback, by conditional `BuildingLightClass` paths, and by WaveClass-like conceal code; the same `LogicClass` vector is ticked in standard `Main_Tick`.

## 0. Working Notes

**Target question:** What exactly does `FUN_0055BAE0` do to the `LogicClass` active vector and `Object+0x98`, and what Rust semantics must preserve it?

**Non-goals:** Do not rename Ghidra symbols, do not re-cover `FUN_0055BAA0` beyond comparison, do not implement Rust, and do not expand into every object destructor.

**Evidence needed to mark COMPLETE:** decompile plus assembly of `0x0055BAE0`, xref/caller map, edge behavior for flag clear/not-found/last element, scheduler implications, active-YR status, and Rust handoff.

**Stop conditions:** Stop after all direct static xrefs are classified, helper edge cases are drained, and Rust surfaces are scanned read-only.

## 1. Overview

`FUN_0055BAE0` is the unregister helper paired with `FUN_0055BAA0` for the `LogicClass` active-object vector at the singleton normally passed as `ECX=0x87F778`. It is guarded by `ObjectClass+0x98`: if the byte is clear, it does nothing; if the byte is set, it tries to remove the object pointer from the vector, then clears the byte even if the vector lookup fails.

The removal algorithm is stable compaction, not swap-remove. It decrements the vector count and shifts later entries one slot left. It does not zero the stale tail slot. Because the main live-object loop reloads vector count after each object AI call but still increments its loop index, self-removal or removal of an earlier/current entry can skip the object shifted into the current index.

## 2. Class Layout / Key Offsets

| Offset / address | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `LogicClass+0x04` | `ObjectClass**` | Active object pointer array. | remover shift reads/writes `[ESI+0x04]`; tick loop reads `[EDI+0x04]` at `0x0055B608` | Yes |
| `LogicClass+0x10` | `int` | Active vector count. | remover reads/writes `[ESI+0x10]`; tick loop reloads it at `0x0055B613` | Yes |
| `LogicClass vtable +0x10` | method | Find-index method for an object pointer argument. | remover calls `[EDX+0x10]` after pushing address of stack object pointer at `0x0055BAF1..0x0055BAFA` | Yes |
| `ObjectClass+0x81` | byte | `InLimbo`; separate from active membership. | `ObjectClass::Conceal` calls the remover at `0x005F4DD3`, then near the conceal tail sets `MOV byte [ESI+0x81],1` at `0x005F4E9E` (immediately followed by `+0x80=0` at `0x005F4EA5`). [2026-05-29: prior cite gave malformed range `0x005F4DD8..0x005F4DD3` (end < start); corrected via `disassemble_function 0x005F4D30` — set is at 0x005F4E9E, well after the remover CALL, so the "after unregister" ordering claim holds.] | Yes |
| `ObjectClass+0x90` | byte | alive/dead state; separate from active membership. | destructor and pending-delete reports; not read by remover | Yes |
| `ObjectClass+0x98` | byte | active-vector membership guard. | remover reads it at `0x0055BAE7` and clears it at `0x0055BB27`; add helper sets it after insert success | Yes |
| `ObjectTypeClass+0x234` | byte | type-level logic eligibility gate before `ObjectClass::Conceal` calls remover. | `ObjectClass::Conceal` decompile and xref context `0x005F4DA6..0x005F4DD3` | Conditional per type |

## 3. Core Logic

### 3.1 Helper pseudocode

```text
unregister_active_object(vector, object):
    if object.active_membership_byte == 0:
        return

    index = vector.find_index(&object)
    if index != -1 and index < vector.count:
        vector.count -= 1
        if index < vector.count:
            for src = index + 1; src < vector.count + 1; src++:
                vector.items[src - 1] = vector.items[src]

    object.active_membership_byte = 0
```

Important correction to the decompiler display: Ghidra's pseudocode shows `unaff_retaddr + 0x98` for the final byte clear. Assembly proves the final object pointer is reloaded from `[ESP+0x8]` after the initial `PUSH ESI`; this is the original stack argument, not the return address.

### 3.2 Verified helper details

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| The helper is a `thiscall`-style vector method: `ECX` is the vector, stack arg is the object pointer. | Callers push object then set `ECX=0x87F778`; helper saves `ECX` in `ESI` at `0x0055BAE5`. | High | Yes |
| The first operation on the object is `MOV CL,[object+0x98]`; there is no null-object guard. | `0x0055BAE7..0x0055BAED` | High | Yes, caller contract |
| If `Object+0x98 == 0`, the helper returns immediately without vector lookup, count change, or byte write. | `0x0055BAED..0x0055BAEF` jumps to return at `0x0055BB2E`. | High | Yes |
| If the byte is set, the helper calls vector vtable `+0x10` with the address of the stack object pointer. | `0x0055BAF1..0x0055BAFA` | High | Yes |
| A returned index of `-1` skips count/shift but still clears `Object+0x98`. | `0x0055BAFD..0x0055BB00` then fallthrough to `0x0055BB23..0x0055BB27`. | High | Yes |
| A returned index `>= count` also skips count/shift but still clears `Object+0x98`. | `0x0055BB02..0x0055BB07` then `0x0055BB23..0x0055BB27`. | High | Yes |
| For a valid index, count is decremented before any shift copy. | `DEC ECX` and `MOV [ESI+0x10],ECX` at `0x0055BB09..0x0055BB0C`. | High | Yes |
| Removing the last element decrements count and performs no shift. | after decrement, `CMP index,new_count; JGE 0x0055BB23` at `0x0055BB0A..0x0055BB0F`. | High | Yes |
| Removing a non-last element shifts later pointers left one slot, preserving relative order. | loop `0x0055BB11..0x0055BB21`. | High | Yes |
| The stale tail slot is not cleared. Logical length is controlled only by count. | no write to `items[count]` in `0x0055BAE0..0x0055BB2F`; only shift-left writes occur. | High | Yes |
| The helper has no meaningful return contract for callers; callers either ignore it or set their own `AL`. | `RET 4`; wrappers such as `FUN_00437030` set `AL=1` after successful helper call. | High | Yes |

### 3.3 Direct comparison to `FUN_0055BAA0`

| Operation | Add helper `FUN_0055BAA0` | Remove helper `FUN_0055BAE0` | Evidence |
|---|---|---|---|
| Membership byte check | If `Object+0x98 != 0`, returns success without append. | If `Object+0x98 == 0`, returns without lookup. | add helper report; remover `0x0055BAE7..0x0055BAEF` |
| Vector operation | Appends at tail after optional grow; ordinary callers pass unique flag `0`. | Finds index, decrements count, shifts left. | `DynamicVector__Insert @ 0x005519B0`; remover `0x0055BAF1..0x0055BB21` |
| Byte write | Sets `Object+0x98=1` only after insert success. | Clears `Object+0x98=0` after any flagged removal attempt, even not-found. | add helper report; remover `0x0055BB23..0x0055BB27` |
| Duplicate/desync behavior | Duplicate active byte suppresses duplicate append. | Desynced set byte self-heals to clear even when pointer missing. | same |

## 4. INI Keys

No INI key is read by `FUN_0055BAE0`. Reachability is controlled by object lifecycle and type/class data.

| Key / data source | Effect on this helper | Default / stock status | Evidence |
|---|---|---|---|
| `ObjectTypeClass+0x234` | `ObjectClass::Conceal` calls the remover only for logic-enabled types. | Type-specific; stock bullets/anim/effect paths use logic-enabled objects. | `ObjectClass::Conceal @ 0x005F4DA6..0x005F4DD3`; prior reveal/type reports |
| `HasSpotlight=` | Creates `BuildingLightClass` objects whose wrapper/destructor call the remover. | Default false; stock repo/visible retail files have no assignments; maps/mods can enable. | `BUILDINGLIGHT_HASSPOTLIGHT_REGISTRATION_RESWARM_20260528.md`; xrefs `0x00437042`, `0x004370EE` |
| `IsSonic=` / `IsMagBeam=` | Creates WaveClass paths that can call the remover on conceal. | Stock-live for sonic/magbeam WaveClass paths per prior swarm. | raw xref `0x0075F9BD`; `DIRECT_NON_REVEAL_FUN_0055BAA0_CALLERS_RESWARM_20260528.md` |

## 5. Integration Points

### 5.1 Caller map

| Caller / xref | Call-site shape | Meaning | Active in YR |
|---|---|---|---|
| `ObjectClass::Conceal @ 0x005F4D30`, xref `0x005F4DD3` | `PUSH ESI; MOV ECX,0x87F778; CALL 0x0055BAE0` | Normal conceal/unlimbo removal after type/game-mode/UniqueID gates, before later conceal tail writes. | Yes |
| `ObjectClass::Destructor @ 0x005F3B80`, xref `0x005F3D75` | tests `[ESI+0x98]`, then `PUSH ESI; MOV ECX,0x87F778; CALL` | Destructor fallback clears live membership if still set. | Yes |
| `FUN_00437030`, xref `0x00437042` | calls `ObjectClass::Conceal`, and on success calls remover again directly. | BuildingLight-style conceal wrapper; second remover is idempotent because `Conceal` normally already cleared `+0x98`. | Conditional on `BuildingLightClass` |
| `BuildingLightClass::Destructor @ 0x004370C0`, xref `0x004370EE` | calls `ObjectClass::Conceal`, then direct remover on success, then removes from BuildingLight vector. | Spotlight object finalizer path. | Conditional on `HasSpotlight=yes` |
| xref `0x00435B7E` inside `FUN_00435B70` (BuildingLight constructor/destructor region) | disassembly shows a direct remover call followed by BuildingLight vector removal and `ObjectClass::Destructor`. | BuildingLight-related teardown path; call shape verified. [2026-05-29: `get_function_xrefs 0x0055BAE0` now reports `From 00435b7e in FUN_00435b70` — the call sits inside a cleanly-bounded (still unnamed) function; the earlier "bad/overlapping boundary" caveat no longer applies.] | Conditional/unclear |
| raw xref `0x0075F9BD` in `FUN_0075F980` | active-game/non-limbo path calls `Mark(1)`, `+0x124(0)`, map/display remove helper, remover, `+0x11C`, then writes `+0x81=1`, `+0x80=0`. | WaveClass-like conceal/removal path. | Yes for WaveClass objects created by stock-live sonic/magbeam paths; exact boundary is partly Ghidra-defined by surrounding functions |

### 5.2 Tick-cycle interaction

`LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` runs the active vector at `0x0055B5FB..0x0055B619`:

1. Load `count` from the vector.
2. Load `items[index]`.
3. Call object vtable `+0x5C`.
4. Reload `count`.
5. Increment index and compare against the reloaded count.

Consequences from the verified helper plus scheduler:

| Case | Consequence | Evidence | Confidence |
|---|---|---|---|
| Current object unregisters itself. | Immediate successor shifts into the current index; scheduler increments past it, so it waits for a later pass. | remover shift `0x0055BB11..0x0055BB21`; scheduler `0x0055B610..0x0055B619` | High |
| Current object unregisters an earlier object. | Later entries shift left; after current AI returns, scheduler increments from the old index and can skip the object that is now at or before that index. | same | High, inferred composition |
| Current object unregisters a later object. | The removed later object will not run; the next object after current remains reachable unless count shrink ends the pass. | same | High, inferred composition |
| Object is flagged but missing from vector. | No vector mutation occurs, but `Object+0x98` is cleared, preventing repeated expensive/removal attempts. | `0x0055BB00..0x0055BB27` | High |

## 6. Current Rust Implementation Status

> **2026-05-29 correction (verify-and-fix):** §6 below previously described a
> `live_object_order: Vec<u64>` with `Vec::contains`-based register and
> unconditional `retain` unregister, and "no membership byte." That is STALE.
> Rust now has a dedicated `LogicVector` type (`src/sim/world/logic_vector.rs:13`)
> plus a per-entity membership flag `GameEntity::in_logic_vector`
> (`src/sim/game_entity.rs:172`, default `false` at `:447`), and register/unregister
> are byte-gated. Verified by Reading those Rust files this session. Line refs and
> deltas refreshed accordingly. Genuine remaining gaps (Mark/PUT-success reveal
> gate-chain, `ObjectType+0x234` eligibility gate, `IsAlive`/`+0x90` interplay) are
> preserved in the delta column.

| Rust surface | Current behavior | Delta against native remover |
|---|---|---|
| `src/sim/world/logic_vector.rs:13` (`LogicVector`); held at `src/sim/world/mod.rs:319` (`logic: LogicVector`) | Dedicated insertion-ordered active-order type owning a `Vec<u64>`; membership tracked separately by the `in_logic_vector` flag on each entity. | Vector exists AND an object-local membership equivalent now exists (`GameEntity::in_logic_vector`, `src/sim/game_entity.rs:172`), mirroring native `Object+0x98`. |
| `src/sim/world/mod.rs:680` | `register_live_object` byte-gates on `!e.in_logic_vector`, sets the flag, then `self.logic.push(stable_id)`. | Now byte-gated and idempotent (matches native `+0x98` guard). Still NOT enforced: the native reveal gate-chain upstream of the byte set — `Mark(PUT)`-success / `ObjectType+0x234` logic-eligibility / `IsAlive`(`+0x90`) checks in `ObjectClass::Conceal`/Reveal are not modeled in Rust; Rust callers decide reveal eligibility ad hoc. |
| `src/sim/world/mod.rs:689` | `unregister_live_object` gates on the membership flag (`if !e.in_logic_vector { return; }`), clears the flag, then `self.logic.remove(stable_id)` (compacting `retain`, `logic_vector.rs:29`, documented "Never swap-remove"). | Byte-gated single-pass remove now matches native: no-op when flag clear; clears flag then compacts. Note: when the entity is already gone from the store, the order is still scrubbed (mod.rs:696-697), which is benign because conceal precedes store removal. |
| `src/sim/world/mod.rs:745` | `live_object_order_snapshot` returns `self.logic.snapshot()` (the order verbatim, no sorted fallback). | Sorted-storage-fallback DRIFT is fixed: snapshot is order-verbatim (`logic_vector.rs:34`). Doc comment notes it is a point-in-time copy that cannot observe same-pass register/unregister; for native same-pass semantics use `for_each_live_object`. |
| `src/sim/world/mod.rs:763` | `for_each_live_object` re-reads `self.logic.len()` after each body call but advances the index by one (no index repair). | Matches the native scheduler's reload-count-but-increment-index contract: tail-appended members run later this pass; a compacting unregister can skip the successor pulled into the just-processed slot. |
| `src/sim/entity_store.rs` (`EntityStore` `BTreeMap`) | Direct remove by stable ID. | Storage lifetime is separate from active-vector membership; removal from storage is not equivalent to unregister (conceal must clear membership first). |
| `advance_tick` (phased tick body) | A phased tick body, not a single central object-vector AI pass. | Native self-unregister skip semantics apply only inside `for_each_live_object`-style passes; the broad phased tick still snapshots IDs per-phase. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0055BAE0` body | verified | decompile and assembly `0x0055BAE0..0x0055BB2F` | none |
| flag-clear early return | verified | `0x0055BAE7..0x0055BAEF` | none |
| vector find-index call | verified | `0x0055BAF1..0x0055BAFA` | exact global vtable target name not renamed under read-only rules |
| not-found / invalid-index behavior | verified | `0x0055BAFD..0x0055BB27` | none |
| valid remove and left-shift loop | verified | `0x0055BB09..0x0055BB21` | none |
| stale tail behavior | verified | no tail-slot write in `0x0055BAE0..0x0055BB2F` | none |
| caller map | verified | `get_function_xrefs(0x0055BAE0)` and assembly contexts | `0x00435B7E` is bounded inside `FUN_00435B70` (2026-05-29); only the function NAME remains unresolved |
| `ObjectClass::Conceal` integration | verified | decompile and call-site `0x005F4DD3` | none |
| destructor fallback integration | verified | decompile and call-site `0x005F3D75` | none |
| WaveClass-like raw xref | touched-not-exhausted | assembly context `0x0075F980..0x0075F9DD`; prior Wave direct-registration docs | exact best function name/boundary |
| live scheduler implication | verified | scheduler assembly `0x0055B608..0x0055B619` plus remover body | concrete runtime index distributions require debugger |
| Rust active-order surfaces | touched-not-exhausted | read-only scans of `src/sim/world/mod.rs`, `src/sim/entity_store.rs` | implementation design and tests |
| save/load `Object+0x98` final value | deferred | prior post-load report | runtime sampling across object classes |

## 8. Open Questions - Final State

- `[RESOLVED] AVR-001 - Is `FUN_0055BAE0` the remove helper paired with active-vector registration? -> Yes; all normal call sites pass `ECX=0x87F778`, the same singleton used by `FUN_0055BAA0`.` (evidence: xref assembly contexts `0x005F4DD3`, `0x005F3D75`, `0x00437042`, `0x0075F9BD`)
- `[RESOLVED] AVR-002 - What is the guard? -> object-local byte `Object+0x98`; zero returns without lookup.` (evidence: `0x0055BAE7..0x0055BAEF`)
- `[RESOLVED] AVR-003 - Does null object input have a safe path? -> No; object is dereferenced before any null check.` (evidence: `0x0055BAE7`)
- `[RESOLVED] AVR-004 - What happens if the byte is set but the object is absent from the vector? -> Count is unchanged and `Object+0x98` is cleared.` (evidence: `0x0055BAFD..0x0055BB27`)
- `[RESOLVED] AVR-005 - What happens if the found index is out of range? -> Count is unchanged and `Object+0x98` is cleared.` (evidence: `0x0055BB02..0x0055BB27`)
- `[RESOLVED] AVR-006 - What happens when removing the last entry? -> Count decrements and no shift loop runs.` (evidence: `0x0055BB09..0x0055BB0F`)
- `[RESOLVED] AVR-007 - What happens when removing a middle entry? -> Count decrements, then later entries shift left preserving relative order.` (evidence: `0x0055BB11..0x0055BB21`)
- `[RESOLVED] AVR-008 - Does it zero the stale tail? -> No.` (evidence: no write to `items[count]` in helper body)
- `[RESOLVED] AVR-009 - Does the helper return success/failure? -> No meaningful return; callers ignore or set their own `AL`.` (evidence: `RET 4`; wrapper `0x00437047` sets `AL=1`)
- `[RESOLVED] AVR-010 - Is `ObjectClass::Conceal` a live caller? -> Yes under object type and mode/UniqueID gates.` (evidence: `0x005F4DCD..0x005F4DD3`)
- `[RESOLVED] AVR-011 - Is destructor fallback a live caller? -> Yes, `ObjectClass::Destructor` tests `+0x98` and calls the helper if still set.` (evidence: `0x005F3D65..0x005F3D75`)
- `[RESOLVED] AVR-012 - Are BuildingLight paths callers? -> Yes; wrapper/destructor call remover after successful `ObjectClass::Conceal`.` (evidence: `0x00437042`, `0x004370EE`; HasSpotlight report)
- `[RESOLVED] AVR-013 - Is there a WaveClass-like caller? -> Yes, raw xref `0x0075F9BD` in the WaveClass region calls remover during active conceal flow.` (evidence: assembly context `0x0075F980..0x0075F9DD`; prior WaveClass direct-registration reports)
- `[RESOLVED] AVR-014 - What is the same-pass scheduler effect? -> The loop reloads count after AI but increments index, so current/earlier removal can skip shifted successors.` (evidence: `0x0055B608..0x0055B619`; remover shift loop)
- `[RESOLVED] AVR-015 - Does current Rust match native unregister edge behavior? -> Largely yes now. `unregister_live_object` (`src/sim/world/mod.rs:689`) gates on the per-entity membership flag `in_logic_vector` (`src/sim/game_entity.rs:172`), clears it, then calls `LogicVector::remove` (`src/sim/world/logic_vector.rs:29`), an order-preserving compacting `retain` explicitly documented "Never swap-remove." This matches the native byte-gate + single compacting remove. Remaining delta is upstream of unregister: the native reveal gate-chain (Mark/PUT-success, `ObjectType+0x234` logic-eligibility, `IsAlive`/`+0x90`) is NOT enforced in Rust.` (evidence: 2026-05-29 Read of `src/sim/world/mod.rs:689`, `src/sim/world/logic_vector.rs:29`, `src/sim/game_entity.rs:172`)
- `[DEFERRED] AVR-016 - What is the exact named function for xref `0x00435B7E`?` (category: `bounded-cost-too-high`; reason: the call is now cleanly bounded inside `FUN_00435B70` per `get_function_xrefs 0x0055BAE0` [2026-05-29] — the earlier bad-boundary blocker is gone; only the human-readable function NAME is still unresolved and naming is forbidden under read-only rules; next-step-if-pursued: decode `FUN_00435B70`'s role)
- `[DEFERRED] AVR-017 - What is the final post-load `Object+0x98` value for every object-derived class?` (category: `needs-runtime-debugger`; reason: previous static save/load swarm found no standard post-load setter; final byte values require runtime sampling)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| Unregister is gated by object-local membership byte; `+0x98==0` returns without vector lookup or mutation. | `0x0055BAE7..0x0055BAEF`; Active in YR: Yes | IMPLEMENTED: `in_logic_vector` flag (`src/sim/game_entity.rs:172`) gates `unregister_live_object` (`src/sim/world/mod.rs:689`); returns early when flag clear. | `src/sim/world/mod.rs:689`, `src/sim/game_entity.rs:172` | Keep active-membership state separate from `EntityStore` and only attempt list removal when active. | Conceal an object that is stored but not live-active; active vector and ordering remain unchanged. | `active_unregister_inactive_object_is_noop` | Do not treat storage membership or non-limbo state as active-list membership. |
| Flagged-but-missing unregister clears the membership byte even if no vector entry is removed. | `0x0055BAFD..0x0055BB27`; Active in YR: Yes | PARTIAL: `LogicVector::remove` is a no-op when absent (`logic_vector.rs:29`); the flag is cleared before the remove (`mod.rs:694`). Self-heal holds, but Rust uses a debug invariant (`debug_assert_logic_membership_consistent`, `mod.rs:724`) rather than tolerating a persistent flag/order desync at runtime. | `src/sim/world/mod.rs:689`, `src/sim/world/logic_vector.rs:29` | If active flag/list become desynced, unregister self-heals by clearing the flag and does not panic or scan forever. | Force a test object with active flag true but absent from active list; unregister clears flag and leaves list count unchanged. | `active_unregister_flagged_missing_clears_membership` | Do not leave an object permanently active after a failed removal search. |
| Removal decrements count and shifts later entries left; stale tail is ignored, not cleared. | `0x0055BB09..0x0055BB21`; Active in YR: Yes | IMPLEMENTED: `LogicVector::remove` (`logic_vector.rs:29`) is an order-preserving compacting `retain` explicitly documented "Never swap-remove." Native is single-entry; Rust `retain` removes all matching ids, but the byte gate + register idempotency (`mod.rs:680`) keeps the order duplicate-free, so the two are equivalent for non-corrupt state. | `src/sim/world/logic_vector.rs:29`, `src/sim/world/mod.rs:689` | Remove the found active entry with stable compaction semantics. | Register A,B,C, unregister B, active order is A,C. | `active_unregister_compacts_once_preserving_order` | Do not use swap-remove. |
| Current/self removal during live object AI can skip the immediate shifted successor because the scheduler reloads count after AI but increments the index. | remover `0x0055BB11..0x0055BB21`; scheduler `0x0055B610..0x0055B619`; Active in YR: Yes | IMPLEMENTED for live passes: `for_each_live_object` (`src/sim/world/mod.rs:763`) re-reads `self.logic.len()` after each body call and advances index by one, mutating/iterating the same vector. Still global gap: the broad `advance_tick` is phased and most systems snapshot IDs per-phase, so this same-pass skip semantics only applies where `for_each_live_object` is actually used. | `src/sim/world/mod.rs:763` (live pass), broad phased `advance_tick` | A native-equivalent live-object pass must mutate and iterate the same vector, not a pass-entry snapshot. | A,B,C in active order; A's AI unregisters itself; B is not called until a later pass, C is next if still in vector. | `active_scheduler_self_unregister_skips_shifted_successor` | Do not process all pass-entry IDs after removals; that changes same-tick behavior. |
| `ObjectClass::Conceal` unregisters before setting `InLimbo`, and destructor has a fallback if `+0x98` remains set. | `0x005F4DD3`; `0x005F3D65..0x005F3D75`; Active in YR: Yes | PARTIAL: `conceal`/`unregister_live_object` exist (`src/sim/world/mod.rs:689`,`:709`) and gate on the flag. Still a gap: there is no modeled `InLimbo` (`+0x81`) sequencing nor a destructor-fallback re-check — Rust relies on conceal having already cleared membership before store removal rather than a fallback if the flag is still set at teardown. | `src/sim/world/mod.rs:689`, `:709`; future conceal/uninit/destructor surfaces | Separate conceal/unregister from storage deletion and pending-delete finalization; model a destructor-time fallback unregister. | Conceal a live object: membership clears before limbo/storage removal; destructor fallback is idempotent if membership already clear. | `conceal_unregisters_before_limbo_and_destructor_fallback_is_idempotent` | Do not model unregister only as a post-`EntityStore::remove` cleanup. |

### Negative Facts / Do Not Do

- Do not call `Object+0x98` "IsOnMap." The helper proves it is an active-vector membership byte; `Object+0x81` is the separate `InLimbo` byte. Evidence: remover `0x0055BAE7`, `ObjectClass::Conceal`.
- Do not remove active entries with unordered swap-remove. Evidence: stable left-shift loop `0x0055BB11..0x0055BB21`.
- Do not rely on `Vec::retain` as an exact remover once a membership byte exists. Evidence: native removes at most the found entry after the byte gate, then clears the byte; `retain` removes all equal IDs.
- Do not assume a flagged object missing from the active vector keeps its flag set. Evidence: `0x0055BB23..0x0055BB27`.
- Do not collapse BuildingLight/Wave direct remover paths into ordinary `ObjectClass::Conceal` only. Evidence: direct xrefs `0x00437042`, `0x004370EE`, `0x0075F9BD`.

### Remaining Uncertainty

- The BuildingLight-region xref `0x00435B7E` is now cleanly bounded inside `FUN_00435B70` (2026-05-29 verify via `get_function_xrefs 0x0055BAE0`); only the human-readable function name remains unresolved, and naming is forbidden under read-only rules.
- Runtime final `Object+0x98` after save/load remains a separate watchpoint/debugger question; static wrapper work found no standard post-load re-registration pass.
- Concrete retail-map index distributions are not proven here. Static evidence proves mechanism; runtime traces would show how often particular adjacent-object skip cases occur.

### Stale Docs / Follow-up Docs

- `docs/research/TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`: replace `0x98 | byte | IsOnMap` with `0x98 | byte | active LogicClass vector membership guard used by FUN_0055BAA0/FUN_0055BAE0; distinct from InLimbo (+0x81) and IsAlive (+0x90)`.
- Any lifecycle wording that says unregister "removes by ID if present" should be replaced with: "`FUN_0055BAE0` first checks `Object+0x98`; if clear it is a no-op, and if set it clears the byte even when vector lookup fails. Valid removal decrements count and shifts later entries left without zeroing the stale tail."

## Sources

- Ghidra read-only:
  - `get_function_by_address(0x0055BAE0)` -> function body `0x0055BAE0..0x0055BB31`
  - `decompile_function(0x0055BAE0)`
  - `disassemble_function(0x0055BAE0)`
  - `get_function_xrefs(0x0055BAE0)` -> xrefs `0x005F3D75`, `0x005F4DD3`, `0x00437042`, `0x004370EE`, `0x0075F9BD`, `0x00435B7E`
  - `get_assembly_context` for every xref above
  - `ObjectClass::Conceal @ 0x005F4D30`
  - `ObjectClass::Destructor @ 0x005F3B80`
  - `BuildingLightClass::Destructor @ 0x004370C0`
  - `FUN_00437030`, `FUN_00437050`, `BuildingLightClass::Constructor @ 0x00435820`
  - `FUN_0075F8B0` / WaveClass-region raw assembly around `0x0075F980..0x0075F9DD`
  - `DynamicVector__Insert @ 0x005519B0`, `DynamicVector__SortedInsert @ 0x00551A90`
  - `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`, especially `0x0055B608..0x0055B619`
- Prior docs referenced:
  - `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
  - `docs/research/COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`
  - `docs/research/REVEAL_GAMEMODE_OWNER_STATUS_GATE_RESWARM_20260528.md`
  - `docs/research/BUILDINGLIGHT_HASSPOTLIGHT_REGISTRATION_RESWARM_20260528.md`
  - `docs/research/DIRECT_NON_REVEAL_FUN_0055BAA0_CALLERS_RESWARM_20260528.md`
  - `docs/research/POST_LOAD_OBJECT_98_OWNER_RECONCILIATION_RESWARM_20260528.md`
- Rust source scanned read-only:
  - `src/sim/world/mod.rs`
  - `src/sim/entity_store.rs`
  - `src/sim/passenger.rs`
  - `src/sim/world/world_spawn.rs`
  - `src/sim/world/world_orders.rs`

Status: COMPLETE.
