# Logic Object Registration Helper FUN_0055BAA0 - Ghidra Research Report

**Address(es):** `0x0055BAA0` primary registration helper; adjacent remover `0x0055BAE0`; `DynamicVector__Insert @ 0x005519B0`; helper unique-insert path `0x00551A90`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** logic-object registration/unregistration around `FUN_0055BAA0`, `ObjectClass+0x98` membership state, direct vector insertion/removal side effects, and the tick-list contract for objects that receive `vtable+0x5C` AI from the LogicClass-owned layer.
**Non-Scope:** full `LogicClass::PerTickUpdate` scheduler ordering outside the LogicClass-owned layer, object class-specific AI bodies, full save/load reconstruction behavior, and non-LogicClass global pools except where needed to distinguish the helper's vector contract.
**Confidence:** High for the helper/remover/vector mechanics verified from binary bytes (re-confirmed 2026-05-29 via `disassemble_function 0x0055BAA0`, `0x0055BAE0`, `0x005519B0`). High for the current Rust delta as of 2026-05-29: the port now implements `LogicVector` + `in_logic_vector` + register/unregister, re-audited against current code; the one remaining gap is the reveal gate-chain (see Section 6 / Section 9).
**Active in YR:** Yes. The helper is called by `ObjectClass::Reveal @ 0x005F5040` with `ECX=0x87F778`, the global LogicClass instance documented in `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md`; `LogicClass::PerTickUpdate @ 0x0055B608..0x0055B619` iterates that same vector every game tick.

## 1. Overview

`FUN_0055BAA0` is the add-to-logic-layer helper for objects that should receive the per-tick `vtable+0x5C` AI call from the LogicClass-owned dynamic vector. It is not a general scheduler: it is a membership guard plus append operation, keyed by `ObjectClass+0x98`.

The adjacent function at `0x0055BAE0` is the matching remover for the same layer. Together they establish a strict object-local membership contract: `+0x98 == 1` means "already considered in the LogicClass layer"; `+0x98 == 0` means "not registered for this layer." This flag is separate from map visibility, limbo, cell occupancy, sorted layer membership, and global entity storage.

## 2. Class Layout / Key Offsets

| Offset / address | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `LogicClass+0x04` | `ObjectClass**` | Pointer array for this LogicClass layer. | `LogicClass::PerTickUpdate @ 0x0055B608` loads `[edi+4]`; `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md` layout. | Yes - singleton `0x0087F778` is called from standard tick. |
| `LogicClass+0x08` | `int` | Vector capacity. | `DynamicVector__Insert @ 0x005519CF..0x005519D7` compares count to capacity. | Yes - used by insertion into the live layer. |
| `LogicClass+0x0D` | `bool` | Vector owns/allocated-storage flag used by growth path. | `DynamicVector__Insert @ 0x005519D9..0x005519E4`; constructor `0x0055BB40` writes it. | Yes - insert growth guard is on the live path. |
| `LogicClass+0x10` | `int` | Current count. | Insert writes `count+1` at `0x00551A13`; PerTick reloads it at `0x0055B613`. | Yes. |
| `LogicClass+0x14` | `int` | Grow step; must be positive to grow when full. | `DynamicVector__Insert @ 0x005519E4..0x005519F4`. | Yes, conditional on full vector. |
| `ObjectClass+0x81` | `bool` | InLimbo, not the logic-membership bit. | `OBJECTCLASS_GHIDRA_REPORT.md` constructor/reveal/conceal offsets. | Yes, but not sufficient for AI-list membership. |
| `ObjectClass+0x90` | `bool` | IsAlive; `ObjectClass::Reveal` continues to display submission based on this later. | `AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md` and `OBJECTCLASS_GHIDRA_REPORT.md`. | Yes, but separate from logic registration. |
| `ObjectClass+0x98` | `bool` | LogicClass-layer membership flag for duplicate prevention and unregister gating. | `FUN_0055BAA0 @ 0x0055BAA5..0x0055BAC6`; remover `0x0055BAE7..0x0055BB27`; constructor initializes 0 in `OBJECTCLASS_GHIDRA_REPORT.md`. | Yes. |
| `ObjectTypeClass+0x234` | `bool` | Type-level "logic-enabled object" gate used by `ObjectClass::Reveal`. | `ObjectClass::Reveal @ 0x005F4DA6..0x005F4DD3` and `0x005F5038..0x005F5040`; BulletType constructor sets it in `BULLETTYPECLASS_GHIDRA_REPORT.md`. | Yes for bullets and other logic-enabled types; conditional per object type. |

## 3. Core Logic

### 3.1 Registration helper `0x0055BAA0`

Calling convention (verified via `disassemble_function 0x0055BAA0` — corrected 2026-05-28):
- `ECX` = the LogicClass vector (passed through from caller; helper uses `RET 0x8`, confirming only 2 stack args)
- `[ESP+8]` = `object` (first stack arg; checked for `+0x98` and passed as item to Insert)
- `[ESP+C]` = `unique_scan_flag` (second stack arg; passed as the flag to `DynamicVector__Insert`)

The earlier pseudocode signature `register_logic_object(vector, object, unique_scan_flag)` was MISLEADING — it implied `vector` is a normal first positional stack parameter. The vector arrives in ECX (carried through from the caller, e.g. `ObjectClass::Reveal` sets ECX = `0x87F778` before the call), not on the stack. ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT.

Pseudocode, omitting register noise:

```text
// ECX = vector; stack: object, unique_scan_flag
register_logic_object(ECX=vector, object, unique_scan_flag):
    if object.is_in_logic:
        return true

    if DynamicVector_Insert(ECX=vector, object, unique_scan_flag) succeeds:
        object.is_in_logic = true
        return true

    return false
```

Material findings:

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| Duplicate prevention happens before vector insertion by reading `Object+0x98`. If the flag is already nonzero, the helper returns success and performs no vector write. | `0x0055BAA5..0x0055BAB2`; direct binary disassembly. | High | Yes - helper is called from `ObjectClass::Reveal @ 0x005F5040`. |
| `Object+0x98` is set only after `DynamicVector__Insert` returns success. Insert failure leaves the object flagged not-in-logic. | `0x0055BABB..0x0055BAD3`; direct binary disassembly. | High | Yes. |
| The caller-provided `unique_scan_flag` (second stack arg of the helper, `[ESP+C]`) is passed through to `DynamicVector__Insert` as the flag; ordinary reveal calls pass `0`. `ObjectClass::Reveal` calls `FUN_0055baa0(param_1, 0)` — param_1 is the object, `0` is the flag. The vector is passed in ECX (not as a stack arg). Verified via `disassemble_function 0x0055BAA0` — corrected 2026-05-28: was "second argument" implying stack position 2 is the object; ECX carries the vector. | `ObjectClass::Reveal @ 0x005F5038..0x005F5040` (decompile); `disassemble_function 0x0055BAA0` confirms `RET 0x8`, ECX=vector. | High | Yes. |
| For the ordinary reveal path, no vector scan happens inside `DynamicVector__Insert`; duplicate prevention relies entirely on `Object+0x98`. | `DynamicVector__Insert @ 0x005519B0..0x005519CF` takes the scan path only when the flag argument is nonzero; reveal passes zero. | High | Yes. |
| The helper returns true when the object was already registered. Repeated reveal/register calls are idempotent from the caller's point of view. | `0x0055BAAD..0x0055BAB2`. | High | Yes. |
| Null object input is not guarded in this helper; it dereferences the object before any null check. Null safety is a caller contract. | First object read at `0x0055BAA5`. | High | Yes, conditional on caller correctness; standard callers pass real objects. |

### 3.2 DynamicVector insertion `0x005519B0`

Pseudocode:

```text
insert(vector, item, unique_scan_flag):
    if unique_scan_flag:
        return insert_unique_scan_path(vector, item)

    if count >= capacity:
        if vector can grow and grow_step > 0:
            grow(capacity + grow_step)
        else:
            return false

    data[count] = item
    count += 1
    return true
```

Material findings:

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| With `unique_scan_flag == 0`, insertion appends at exactly the old `count` index and increments count by one. | `0x00551A0A..0x00551A1D`. | High | Yes - all verified `FUN_0055BAA0` callers pass `0`. |
| The vector count is written before the element pointer store. This is a tiny ordering detail; no intervening call occurs between the two writes. | `0x00551A13` writes count; `0x00551A1A` writes pointer. | High | Yes, but only observable under re-entrant/native debugging hazards; no ordinary call interleaves. |
| Full vector growth is attempted only if either the vector owns/grows storage or current capacity is zero, and `grow_step > 0`. | `0x005519D9..0x005519F9`. | High | Yes, conditional on full vector. |
| If growth is unavailable or fails, insert returns false and the caller does not set `Object+0x98`. | `0x005519FB..0x00551A07` plus caller `0x0055BAC0..0x0055BAD3`. | High | Yes, conditional on allocation/capacity failure. |
| The unique-scan path exists but is not used by ordinary object reveal registration. Its only direct static caller is `DynamicVector__Insert` when the flag argument is nonzero. | direct call `0x005519C0 -> 0x00551A90`; direct call scan found ordinary Logic helper callers all pass `0`. | High for call shape, Medium for all possible indirect callers. | Conditional - live helper supports it, but standard Logic registration path does not take it. |

### 3.3 Removal helper `0x0055BAE0`

Pseudocode:

```text
unregister_logic_object(vector, object):
    if not object.is_in_logic:
        return

    index = vector.find_index_of(object)
    if index != -1 and index < count:
        count -= 1
        shift entries after index left by one

    object.is_in_logic = false
```

Material findings:

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| The remover first checks `Object+0x98`; if the flag is zero it performs no vector search and returns. | `0x0055BAE7..0x0055BAEF`, `0x0055BB2E`. | High | Yes - called from `ObjectClass::Conceal @ 0x005F4DD3` and destructor/uninit paths. |
| If `Object+0x98` is set, the remover asks the vector for the object's index via the vector vtable slot `+0x10`, then accepts only indexes other than `-1` and less than `count`. | `0x0055BAF1..0x0055BB07`. | High | Yes. |
| Successful removal decrements count before shifting later entries left. | `0x0055BB09..0x0055BB1F`. | High | Yes. |
| If the object is the last element, count is decremented and no shift loop runs. | `0x0055BB09..0x0055BB0F` followed by `jge 0x0055BB23`. | High | Yes. |
| If the flag was set but the object is not found or the index is invalid, the remover still clears `Object+0x98`. | Failure branches converge at `0x0055BB23`; flag clear is outside the successful-remove-only block. | High | Yes. |
| The remover does not zero the stale tail slot after shifting or decrementing count. Logical membership is controlled by count plus flag, not by clearing unused array memory. | No write to `data[count]` in `0x0055BAE0..0x0055BB2F`; only shift-left writes. | High | Yes. |

### 3.4 Tick-list contract

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| The LogicClass-owned layer is iterated forward by index and reloads `count` after each object's `vtable+0x5C` call. Newly appended tail entries can be reached in the same pass if appended before the loop index reaches them. | `LogicClass::PerTickUpdate @ 0x0055B608..0x0055B619`; prior report `AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`. | High | Yes. |
| The helper appends to the same LogicClass singleton used by the tick loop when called from `ObjectClass::Reveal`. | `ObjectClass::Reveal @ 0x005F503B` sets `ECX=0x87F778`; LogicClass singleton verified at `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md`. | High | Yes. |
| `ObjectTypeClass+0x234` gates whether reveal tries to register the object. For bullet types, the type constructor sets this flag to `1`, making bullets logic-updated by default. | `ObjectClass::Reveal @ 0x005F4DA6..0x005F4DD3`/`0x005F5038..0x005F5040`; `BULLETTYPECLASS_GHIDRA_REPORT.md` constructor sets `field_0x234 = 1`. | High | Yes, conditional per type. |
| Map editor / game mode gates can skip the call in `ObjectClass::Reveal` or `Conceal` when the display/vector compatibility check returns the sentinel value. This does not change the helper contract, only caller reachability. | `ObjectClass::Reveal @ 0x005F501B..0x005F5045`; `ObjectClass::Conceal @ 0x005F4DB0..0x005F4DD8`. | High for branch shape, Medium for exact editor-mode semantics in this slot. | Conditional - standard skirmish path reaches the helper; editor/intro mode paths can bypass. |

## 4. INI Keys

No direct INI key is read by `FUN_0055BAA0`, `0x0055BAE0`, or `DynamicVector__Insert`.

| Key / data source | Scope | Default / value | Effect on this helper | Evidence | Active in YR |
|---|---|---|---|---|---|
| Type-level `ObjectTypeClass+0x234` | Parsed/constructed object type data, not an INI key identified in this slice. | BulletType constructor writes `1`; other type defaults are type-specific. | Gates whether `ObjectClass::Reveal` calls the helper. | `ObjectClass::Reveal @ 0x005F4DA6`; `BULLETTYPECLASS_GHIDRA_REPORT.md`. | Yes, conditional per type. |
| `rulesmd.ini` projectile entries such as `AAHeatSeeker2` | Projectile data. | Uses BulletType default `+0x234=1` unless type code overrides. | Makes fired bullets eligible for logic registration on reveal. | `BULLETTYPECLASS_GHIDRA_REPORT.md`; `AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`. | Yes for stock YR projectiles. |

## 5. Integration Points

| Integration point | Finding | Evidence | Active in YR |
|---|---|---|---|
| `ObjectClass::Reveal @ 0x005F4EC0` | Calls registration helper after type-level logic flag and game-mode/display checks pass. Ordinary call passes `unique_scan_flag=0`. | `0x005F5038..0x005F5040`; prior latency report. | Yes. |
| `ObjectClass::Conceal @ 0x005F4D30` | Calls adjacent remover when type-level logic flag and matching checks pass. | `0x005F4DA6..0x005F4DD3`. | Yes. |
| Object destructor/uninit family | Removes from logic layer if `Object+0x98` is set before base destruction completes. | `ObjectClass` destructor path `0x005F3D65..0x005F3D7A`; `OBJECTCLASS_GHIDRA_REPORT.md` identifies destructor `0x005F3B80`. | Yes. |
| Derived reveal/unlimbo wrappers | Several direct static callers invoke registration/removal around reveal/conceal style paths. | direct call scan: registration callers `0x00435B01`, `0x00437070`, `0x005F5040`, `0x00710492`, `0x0075F95F`; removal callers `0x00435B7E`, `0x00437042`, `0x004370EE`, `0x005F3D75`, `0x005F4DD3`, `0x0075F9BD`. | Yes for standard class lifecycle paths; exact class names for non-ObjectClass callers are deferred. |
| `LogicClass::PerTickUpdate @ 0x0055AFB0` | Calls `vtable+0x5C` on each registered object in the LogicClass-owned layer, using a live count reload. | `0x0055B608..0x0055B619`. | Yes. |
| Other dynamic vectors | `DynamicVector__Insert` is generic; one direct caller outside logic registration at `0x004A9759` inserts into layer-specific map/display arrays and uses unique-scan flag based on layer comparison. | direct call scan to `0x005519B0`; local disassembly around `0x004A9759`. | Yes, but outside this report's helper contract. |

## 6. Current Rust Implementation Status

**Updated 2026-05-29 (verify-and-fix):** the earlier "Rust has no explicit
equivalent of gamemd's `Object+0x98` membership bit plus live appendable
LogicClass vector" status is now STALE. The port has since added a dedicated
`LogicVector` plus a per-entity membership byte and the full register/unregister
primitive. The remaining delta is the *reveal gate-chain* (type-gate + IsAlive +
Mark(PUT)-success), which is still not enforced — see the row below and Section 9.

| Rust surface | Current status | Evidence |
|---|---|---|
| `src/sim/world/logic_vector.rs` (`LogicVector`) | Present and faithful: insertion-ordered `Vec<u64>`, tail-append `push`, order-preserving compacting `remove` (`retain`, never swap-remove), `snapshot` returns order verbatim with **no sorted fallback**, serializes transparently as its inner `Vec<u64>`. This is the `LogicClass+0x04/+0x10` vector analog. | `src/sim/world/logic_vector.rs:13-74` (read 2026-05-29). |
| `src/sim/game_entity.rs::GameEntity.in_logic_vector` | Present: the `Object+0x98` membership-bit analog. Default-initialized to `false`. | `src/sim/game_entity.rs:172` (field), `:447` (init `false`) (grep 2026-05-29). |
| `src/sim/world/mod.rs::register_live_object` | Present and matches `FUN_0055BAA0`: `+0x98` guard → tail-append → set flag; idempotent (returns early if already a member or entity absent). `reveal`/`unlimbo` delegate to it. | `src/sim/world/mod.rs:680-686` (read 2026-05-29). |
| `src/sim/world/mod.rs::unregister_live_object` | Present and matches the remover `0x0055BAE0`: gate on flag → clear flag → compacting `LogicVector::remove`; `conceal` delegates to it. | `src/sim/world/mod.rs:689-711` (read 2026-05-29). |
| `src/sim/world/mod.rs::for_each_live_object` | Present and matches the `LogicClass::PerTickUpdate` live-count contract: forward index pass that re-reads `self.logic.len()` after each body call, so tail-appends in the same pass are visited and there is no index repair on compacting removal. | `src/sim/world/mod.rs:763-770` (read 2026-05-29). |
| `src/sim/world/mod.rs::rebuild_logic_membership` | Present: on load, rebuilds `+0x98` flags from the restored serialized order (vector presence authoritative; `+0x98` not round-tripped). Addresses the formerly-DEFERRED save/load reconstruction question OQ-LOGICREG-017. | `src/sim/world/mod.rs:981-987` (read 2026-05-29). |
| Reveal gate-chain (type-gate `ObjectTypeClass+0x234` / IsAlive `+0x90` / Mark(PUT)-success) | **STILL MISSING.** `register_live_object` has only the membership-flag guard. Callers (`src/sim/passenger.rs:1163`, `:1188`; spawn/unlimbo paths) call it unconditionally, with no type-level logic-enabled check and no IsAlive / placement-success gate before registration. gamemd's `ObjectClass::Reveal` only registers when those upstream checks pass. | `src/sim/world/mod.rs:680-705` (no gate); `src/sim/passenger.rs:1155-1164` (unconditional call) (read 2026-05-29). |
| `src/sim/entity_store.rs` (`EntityStore`) | `EntityStore` is still a `BTreeMap<u64, GameEntity>` for global storage; logic membership is now a *separate* concern owned by `LogicVector` + `in_logic_vector`, not by store presence. | `src/sim/entity_store.rs:33` (grep 2026-05-29). |
| `src/sim/world/mod.rs::advance_tick` | Tick driver now at `mod.rs:1508` (was cited ~1008-1545; line drifted). | `src/sim/world/mod.rs:1508` (grep 2026-05-29). |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0055BAA0` duplicate guard | verified | `0x0055BAA5..0x0055BAB2` | none |
| `FUN_0055BAA0` insertion success/failure behavior | verified | `0x0055BAB5..0x0055BAD3` | none |
| `DynamicVector__Insert @ 0x005519B0` normal append path | verified | `0x005519CF..0x00551A29` | none |
| `DynamicVector__Insert @ 0x005519B0` unique-scan branch | touched-not-exhausted | `0x005519B0..0x005519CC`; callee `0x00551A90` touched | Full semantics of comparator helper `0x005F6220` inside `0x00551A90` are out of this helper's ordinary reveal path. |
| Adjacent remover `0x0055BAE0` | verified | `0x0055BAE0..0x0055BB2F` | none |
| `ObjectClass::Reveal` caller | verified | `0x005F5038..0x005F5040`; `AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md` | none for this helper; broader reveal behavior remains covered by ObjectClass docs. |
| `ObjectClass::Conceal` caller | verified | `0x005F4DA6..0x005F4DD3` | none for logic unregistration. |
| Destructor/uninit removal path | verified | `0x005F3D65..0x005F3D7A`; `OBJECTCLASS_GHIDRA_REPORT.md` | Exact derived destructor names for every caller are outside scope. |
| `LogicClass::PerTickUpdate` live-count iteration | verified | `0x0055B608..0x0055B619`; prior latency report | Full scheduler belongs to slot 1. |
| Save/load restoration of `+0x98` versus vector rebuild | deferred | not investigated with runtime save/load | Requires a separate save/load lifecycle investigation. |
| TS legacy filtering | verified for helper | Call path is through standard YR `ObjectClass::Reveal`/`Conceal` and `LogicClass::PerTickUpdate`; no TS-only gate in the helper itself. | Exact editor/intro bypass behavior is a different caller-context report. |
| Current Rust logic-list equivalent | implemented (re-audited 2026-05-29) | `src/sim/world/logic_vector.rs`; `src/sim/world/mod.rs:680-770`; `src/sim/game_entity.rs:172` | `LogicVector` + `in_logic_vector` + register/unregister + same-pass `for_each_live_object` are in place. Remaining: the reveal gate-chain (`+0x234` type-gate / IsAlive / placement-success) is not yet enforced before registration. |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-LOGICREG-001 - Is FUN_0055BAA0 on a live YR path? -> Yes; ObjectClass::Reveal calls it with ECX=0x87F778 on standard logic-enabled object reveal.` (evidence: `0x005F5038..0x005F5040`; `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-LOGICREG-002 - What prevents duplicate registration? -> The object-local byte at +0x98 is checked before insertion; already-set returns success without vector mutation.` (evidence: `0x0055BAA5..0x0055BAB2`)
- `[RESOLVED] OQ-LOGICREG-003 - Does DynamicVector__Insert scan for duplicates on ordinary reveal? -> No; reveal passes flag 0, so normal append path is used.` (evidence: `0x005F5038`; `0x005519B0..0x005519CF`)
- `[RESOLVED] OQ-LOGICREG-004 - When is +0x98 set? -> Only after insert succeeds.` (evidence: `0x0055BAC0..0x0055BAC6`)
- `[RESOLVED] OQ-LOGICREG-005 - What happens on insert failure? -> Helper returns false and leaves +0x98 clear.` (evidence: `0x0055BAC0..0x0055BAD3`)
- `[RESOLVED] OQ-LOGICREG-006 - Does normal insertion append or insert at a sorted position? -> It appends at old count.` (evidence: `0x00551A0A..0x00551A1D`)
- `[RESOLVED] OQ-LOGICREG-007 - Does the tick loop snapshot the count? -> No; it reloads count after each AI call.` (evidence: `0x0055B608..0x0055B619`)
- `[RESOLVED] OQ-LOGICREG-008 - Is the adjacent function at 0x0055BAE0 the sibling remover? -> Yes; it checks +0x98, finds object index, decrements count, shifts entries, and clears +0x98.` (evidence: `0x0055BAE0..0x0055BB2F`)
- `[RESOLVED] OQ-LOGICREG-009 - What happens if remover cannot find a flagged object? -> It still clears +0x98 after the failed/invalid index path.` (evidence: `0x0055BB00..0x0055BB27`)
- `[RESOLVED] OQ-LOGICREG-010 - Does removal clear the stale tail slot? -> No direct stale-tail zeroing occurs in the remover.` (evidence: `0x0055BAE0..0x0055BB2F`)
- `[RESOLVED] OQ-LOGICREG-011 - Is null input handled? -> No; helper/remover dereference object before any null guard, so callers must pass valid objects.` (evidence: `0x0055BAA5`; `0x0055BAE7`)
- `[RESOLVED] OQ-LOGICREG-012 - Is Object+0x98 initialized? -> Yes, ObjectClass constructor initializes it to 0.` (evidence: `OBJECTCLASS_GHIDRA_REPORT.md` constructor field table)
- `[RESOLVED] OQ-LOGICREG-013 - Is +0x98 separate from InLimbo and IsAlive? -> Yes; offsets +0x81, +0x90, and +0x98 are distinct fields with distinct callers.` (evidence: `OBJECTCLASS_GHIDRA_REPORT.md`; helper binary addresses)
- `[RESOLVED] OQ-LOGICREG-014 - Does ObjectClass::Conceal unregister logic-enabled objects? -> Yes, under the same type/game-mode gates, it calls remover at 0x005F4DD3.` (evidence: `0x005F4DA6..0x005F4DD3`)
- `[RESOLVED] OQ-LOGICREG-015 - Does destruction/uninit unregister? -> Yes, the base destruction path tests +0x98 and calls remover before base cleanup.` (evidence: `0x005F3D65..0x005F3D7A`; `OBJECTCLASS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-LOGICREG-016 - Is this TS legacy only? -> No; the helper is reached by standard YR object reveal/conceal and the standard YR tick loop.` (evidence: `0x005F5040`; `0x0055B608..0x0055B619`)
- `[DEFERRED-BINARY / RESOLVED-RUST] OQ-LOGICREG-017 - How exactly does save/load restore +0x98 and the LogicClass vector?` The gamemd-side mechanism was NOT re-investigated this session and remains deferred. On the Rust side (re-audited 2026-05-29), `LogicVector` serializes as its inner `Vec<u64>` and `Simulation::rebuild_logic_membership` (`src/sim/world/mod.rs:981-987`) rebuilds the `in_logic_vector` flags from the restored order on load — i.e. vector presence is authoritative and `+0x98` is not round-tripped. Whether this matches the binary's actual load behavior is unverified. (category: `requires-different-system-context`; next-step-if-pursued: trace ObjectClass load and global LogicClass rebuild after scenario load in gamemd.)
- `[DEFERRED] OQ-LOGICREG-018 - Which concrete derived classes own each non-ObjectClass direct caller at 0x00435B01/0x00437070/0x00710492/0x0075F95F and matching removers?` (category: `out-of-scope`; reason: caller names are not needed to prove the helper contract; next-step-if-pursued: map those function starts through vtables/class reports.)
- `[RESOLVED] OQ-LOGICREG-019 - Empty vector edge case? -> If count is zero and capacity permits, append writes slot 0; if capacity/grow are unavailable, insert fails and +0x98 remains clear.` (evidence: `0x005519CF..0x00551A1D`)
- `[RESOLVED] OQ-LOGICREG-020 - First-tick registration edge case? -> An object appended while the forward LogicClass loop is before the new tail can receive AI in that same pass because count is reloaded.` (evidence: `0x0055B608..0x0055B619`; `AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-LOGICREG-021 - Paused-game behavior? -> The LogicClass-owned per-object loop is in PerTickUpdate, which prior timing research says runs unconditionally during pause.` (evidence: `timing/logic-vs-render-loop.md`)
- `[DEFERRED] OQ-LOGICREG-022 - Replay/save restore corner case?` (category: `requires-different-system-context`; reason: replay/save restore belongs to timing/save-load systems, not the helper body; next-step-if-pursued: trace replay object creation and post-load reveal/rebuild.)

Remaining uncertainty is limited to caller-context reconstruction outside the helper and save/load/replay reconstruction. The membership mechanics of `FUN_0055BAA0`, `DynamicVector__Insert`, the adjacent remover, and the live tick-loop effect are resolved for this slice.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Logic AI eligibility is an explicit membership state, not "every entity in storage" and not just "not in limbo." | `Object+0x98` guard/set/clear at `0x0055BAA0` and `0x0055BAE0`; `ObjectClass+0x81` separate in `OBJECTCLASS_GHIDRA_REPORT.md`. | **IMPLEMENTED** (2026-05-29): `GameEntity::in_logic_vector` (`game_entity.rs:172`) + `LogicVector` order (`logic_vector.rs:13`) are a separate membership concern from `EntityStore` presence. | `src/sim/world/logic_vector.rs`; `src/sim/game_entity.rs:172`; `src/sim/world/mod.rs:680-711`. | Objects should receive per-tick object AI only after reveal/activation and should stop after conceal/unregistration/despawn. | `logic_membership_reveal_registers_once_and_conceal_unregisters` - spawn a logic-enabled object, reveal/register twice, assert one AI membership entry; conceal/despawn, assert it no longer ticks. | Do not use `EntityStore` membership alone as the AI tick contract; stored but concealed/unregistered objects must not necessarily tick. |
| Repeated registration is idempotent and returns success without appending a duplicate when `+0x98` is already set. | `0x0055BAA5..0x0055BAB2`. | **IMPLEMENTED** (2026-05-29): `register_live_object` returns early when `in_logic_vector` is already set or entity absent, before any `LogicVector::push`. | `src/sim/world/mod.rs:680-686`. | Duplicate reveal or activation must not double-tick the object in one frame. | `logic_membership_duplicate_reveal_does_not_double_tick` - call activation twice, advance one tick, assert the object's AI counter increments once. | Do not rely only on `Vec::contains` scans as the main contract; gamemd's ordinary reveal path uses the object-local flag as the fast guard. |
| Registration appends to the current tail; the PerTick loop reloads count and can tick newly appended objects in the same pass. | Append path `0x00551A0A..0x00551A1D`; PerTick reload `0x0055B613..0x0055B619`. | **IMPLEMENTED** (2026-05-29): `for_each_live_object` (`mod.rs:763-770`) re-reads `self.logic.len()` after each body call, so same-pass tail-appends are visited and removals are not index-repaired. Verify any newly added per-tick logic driver actually routes object AI through this primitive rather than a `keys_sorted()` snapshot. | `src/sim/world/mod.rs:763-770`; per-tick logic driver in `advance_tick` (`mod.rs:1508`). | A bullet or other logic-enabled object spawned/revealed during the object-AI pass can run its first AI before the tick ends if appended before the pass reaches the tail. | `logic_membership_append_during_ai_can_tick_same_pass` - test entity A's AI spawns/registers entity B; same tick, B's AI counter increments if A was before the new tail. | Do not implement all newly spawned logic objects as "next tick only"; that breaks first-tick projectile behavior. |
| Unregistration clears the membership flag even if the object was flagged but not found in the vector. | `0x0055BB00..0x0055BB27`. | **IMPLEMENTED** (2026-05-29): `unregister_live_object` clears `in_logic_vector` then calls `LogicVector::remove`, which is a no-op if absent — self-healing. | `src/sim/world/mod.rs:689-698`; `logic_vector.rs:29-31`. | Corrupt/stale membership state should self-heal by clearing the object's active flag on unregister attempt. | `logic_membership_unregister_flagged_missing_object_clears_flag` - manually desync flag/list in a unit test, unregister, assert inactive flag and no panic. | Do not leave an object permanently "active" after a failed vector removal search. |
| Removal is compacting and preserves relative order of later entries by shifting them left; stale tail data is ignored by count. | `0x0055BB09..0x0055BB1F`; no tail zero in remover. | **IMPLEMENTED** (2026-05-29): `LogicVector::remove` uses `retain` (order-preserving compacting remove), explicitly never swap-remove. | `src/sim/world/logic_vector.rs:29-31`. | Removing an object during/around a pass should not reorder remaining logic objects. | `logic_membership_unregister_preserves_later_order` - register A,B,C, unregister B, assert tick order A,C. | Do not use swap-remove for the LogicClass-equivalent list unless a separate proof shows order is unobservable; binary shifts left. |
| Reveal only registers after the upstream gate-chain passes: type-level logic-enabled flag (`ObjectTypeClass+0x234`), IsAlive (`+0x90`), and a successful placement/Mark(PUT) in `ObjectClass::Reveal`. | `ObjectClass::Reveal @ 0x005F4DA6..0x005F4DD3`, `0x005F5038..0x005F5040`; `+0x234` in `BULLETTYPECLASS_GHIDRA_REPORT.md`. | **STILL MISSING / GENUINE REMAINING DELTA** (2026-05-29): `register_live_object` (`mod.rs:680`) has only the membership-flag guard; callers (e.g. `passenger.rs:1163`) invoke it unconditionally with no type-gate, no IsAlive check, and no placement-success gate. | `src/sim/world/mod.rs:680-705`; `src/sim/passenger.rs:1155-1164`; future spawn/reveal gate. | A non-logic-enabled type, a dead object, or a failed placement must NOT be registered into the live AI order. | `logic_membership_reveal_gated_by_type_and_alive` - attempt to register a non-logic-enabled / dead / unplaced object, assert it is NOT added to the order. | Do not register on raw spawn/insert; gate registration behind the type-logic flag, IsAlive, and placement success as gamemd's Reveal does. |

### Negative Facts / Do Not Do

- Do not treat `InLimbo == false` as equivalent to "receives AI ticks." Active in YR: Yes; evidence: distinct fields `+0x81` and `+0x98`, and helper/remover only use `+0x98`.
- Do not treat `EntityStore`/global object storage as the same thing as the LogicClass-owned AI vector. Active in YR: Yes; evidence: `LogicClass+0x04/+0x10` vector iterated at `0x0055B608..0x0055B619`, while separate global pools exist in `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md`.
- Do not snapshot the LogicClass-owned vector count at pass entry if reproducing gamemd object AI order. Active in YR: Yes; evidence: count reload after each AI call at `0x0055B613`.
- Do not implement logic-list removal with unordered swap-remove. Active in YR: Yes; evidence: left-shift loop at `0x0055BB11..0x0055BB21`.
- Do not expect `DynamicVector__Insert`'s unique-scan branch to protect ordinary reveal registration. Active in YR: Yes; evidence: reveal passes `0` at `0x005F5038`, making `Object+0x98` the ordinary duplicate gate.

### Remaining Uncertainty

- Exact save/load and replay reconstruction of `Object+0x98` plus the LogicClass vector remains unresolved. This is likely important for persistence parity but requires a separate save/load lifecycle trace.
- Direct non-ObjectClass caller names at `0x00435B01`, `0x00437070`, `0x00710492`, and `0x0075F95F` were not mapped to concrete class names in this slot. Their call shapes support the same helper contract, but class naming is deferred.
- The generic `DynamicVector__Insert` unique-scan callee `0x00551A90` was touched only enough to establish that ordinary LogicClass registration does not use it. Do not generalize this report to all dynamic-vector containers.

## Sources

- Direct binary disassembly from `gamemd.exe`:
  - `FUN_0055BAA0 @ 0x0055BAA0`
  - Adjacent remover `0x0055BAE0`
  - `DynamicVector__Insert @ 0x005519B0`
  - Unique-scan insertion path `0x00551A90` (touched)
  - `LogicClass::PerTickUpdate @ 0x0055B608..0x0055B619`
  - `ObjectClass::Reveal @ 0x005F4EC0`, call site `0x005F5038..0x005F5040`
  - `ObjectClass::Conceal @ 0x005F4D30`, call site `0x005F4DA6..0x005F4DD3`
  - Object destructor/uninit path `0x005F3D65..0x005F3D7A`
- Prior research docs referenced:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/timing/logic-vs-render-loop.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/OBJECTCLASS_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BULLETTYPECLASS_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`
- Rust surfaces scanned (re-audited 2026-05-29):
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/logic_vector.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/mod.rs` (register/unregister `:680-711`, `for_each_live_object` `:763-770`, `rebuild_logic_membership` `:981-987`, `advance_tick` `:1508`)
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/game_entity.rs` (`in_logic_vector` `:172`, init `:447`)
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/passenger.rs` (register call sites `:1163`, `:1188`)
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/entity_store.rs`
