# LogicClass PerTickUpdate Scheduler - Ghidra Research Report

**Address(es):** `LogicClass::PerTickUpdate @ 0x0055AFB0`; main LogicClass object loop `0x0055B5FB..0x0055B619`; registration helper `0x0055BAA0`; remover `0x0055BAE0`; `DynamicVector__Insert @ 0x005519B0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** core `LogicClass::PerTickUpdate` scheduling contract for the LogicClass-owned object vector: iteration order, count reload/snapshot behavior, append-during-iteration behavior, and removal/destroy implications visible from the scheduler plus sibling registration/removal helpers.  
**Non-Scope:** class-specific `vtable+0x5C` AI bodies, full object destruction call graph, save/load reconstruction, replay serialization, and non-LogicClass global vectors except where contrasted with the main object loop.  
**Confidence:** High for the loop mechanics and helper interactions verified from direct binary disassembly plus recent Ghidra-backed reports; Medium for concrete class-specific remove-during-AI reachability because this slice did not trace every `vtable+0x5C` body.  
**Active in YR:** Yes. `Main_Tick` calls `LogicClass::PerTickUpdate` with `ECX=0x87F778` at `0x0055DC99..0x0055DC9E`; recent timing docs identify this as the standard late-housekeeping path in YR.

## 1. Overview

The main LogicClass object scheduler is a live forward vector walk, not a pass-entry snapshot. It calls each registered object's `vtable+0x5C`, increments the loop index, then reloads `LogicClass+0x10` for the next comparison.

That small detail is a reusable engine contract: objects appended to the LogicClass vector during this pass can run later in the same pass, while compacting removal during the pass can shift objects relative to the already-incremented index. Rust code that collects `EntityStore::keys_sorted()` before subsystem passes does not reproduce this contract for logic-enabled objects.

## 2. Class Layout / Key Offsets

| Offset / address | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `0x0087F778` | `LogicClass` singleton | `Main_Tick` passes this as `ECX` to `PerTickUpdate`. | `0x0055DC99..0x0055DC9E`; `LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md` | Yes |
| `LogicClass+0x04` | `ObjectClass**` | Pointer array walked by the main object AI loop. | `0x0055B608..0x0055B60B` | Yes |
| `LogicClass+0x10` | `int` | Live vector count reloaded after every object AI call. | `0x0055B601..0x0055B619`; insert writes it at `0x00551A13` | Yes |
| `ObjectClass+0x98` | `bool` | Logic-list membership bit used by add/remove helpers. | `0x0055BAA5..0x0055BAC6`; `0x0055BAE7..0x0055BB27` | Yes |
| `ObjectTypeClass+0x234` | `bool` | Type-level gate for reveal-time registration into LogicClass. | Gate check at `ObjectClass::Reveal @ 0x005F4FEF..0x005F4FF5` (`MOV AL,byte ptr [EBX + 0x234]; TEST AL,AL; JZ`); registration call at `0x005F5038..0x005F5040`. `BULLETTYPECLASS_GHIDRA_REPORT.md`. (corrected 2026-05-28: was `0x005F5038..0x005F5040` cited as gate evidence; binary shows the gate check is at `0x005F4FEF..0x005F4FF5` and the cited range is the subsequent registration helper call — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT: evidence address pointed to call site, not the flag-test instruction) | Yes, conditional by object type |
| `vtable+0x5C` | function slot | Per-object tick method called by the scheduler. | `0x0055B60E..0x0055B610`; timing docs identify this as object AI | Yes |

## 3. Core Logic

### 3.1 Main LogicClass Object Loop

Pseudocode for the verified loop:

```text
i = 0
if logic.count > 0:
    do:
        object = logic.items[i]
        object.vtable_5C()
        i += 1
    while i < logic.count   // count is re-read here
```

Material findings:

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| The main LogicClass object vector is iterated in ascending index order starting from `0`. | `xor esi, esi` at `0x0055B5FF`, item load from `[items + esi*4]` at `0x0055B608..0x0055B60B`, `inc esi` at `0x0055B616`. | High | Yes |
| The initial empty/non-empty gate reads `LogicClass+0x10` once before entry, but the loop comparison reloads `LogicClass+0x10` after each object call. | Entry read `0x0055B601..0x0055B606`; post-call reload `0x0055B613`; compare/jump `0x0055B616..0x0055B619`. | High | Yes |
| The item pointer is loaded immediately before the `vtable+0x5C` call. There is no null guard for the main LogicClass vector item. | `0x0055B608..0x0055B610`. | High | Yes, with caller/list integrity as a precondition |
| The scheduler does not store a "current object" sentinel or repair the index after the vtable call. The only post-call scheduler work is reload count, increment index, compare. | `0x0055B610..0x0055B619`. | High | Yes |
| `PerTickUpdate` itself is active in standard YR late-housekeeping, not a TS-only leftover. | Direct call from `Main_Tick` at `0x0055DC99..0x0055DC9E`; timing docs place it after render/replay bookkeeping. | High | Yes |

### 3.2 Append-During-Iteration Contract

The ordinary reveal/register path uses `FUN_0055BAA0`, which checks `Object+0x98`, then calls `DynamicVector__Insert`. With the normal flag `0`, insertion appends the object pointer at the old count and increments count.

Material findings:

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| Ordinary registration appends to the vector tail, not a sorted position. | `DynamicVector__Insert @ 0x00551A0A..0x00551A1D`; `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md` | High | Yes |
| Because the scheduler reloads count after each `vtable+0x5C`, a tail append made before the loop reaches the old end can be processed in the same pass. | Scheduler `0x0055B608..0x0055B619` plus insert `0x00551A0A..0x00551A1D`; AAHeatSeeker2 latency report confirms this for bullets. | High | Yes |
| Repeated registration does not double-append an object already in the logic list. | `Object+0x98` early return at `0x0055BAA5..0x0055BAB2`. | High | Yes |
| Insert failure leaves the membership flag clear, so failed appends do not create a phantom active object. | `0x0055BAC0..0x0055BAD3`. | High | Yes, conditional on capacity/allocation failure |

### 3.3 Remove / Destroy Edge Cases

`PerTickUpdate` does not contain explicit destruction handling around the main object loop. The visible behavior comes from the scheduler's lack of index repair combined with the adjacent remover's compacting deletion.

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| The remover decrements count and shifts later elements left; it preserves relative order and does not swap-remove. | `0x0055BB09..0x0055BB21`. | High | Yes |
| The remover clears `Object+0x98` even if a flagged object is not found or has an invalid index. | `0x0055BB00..0x0055BB27`; helper report. | High | Yes |
| The remover does not clear the stale tail slot after compaction; count controls logical membership. | No tail-zeroing write in `0x0055BAE0..0x0055BB2F`. | High | Yes |
| If the currently processed object unregisters itself and removal compacts the vector, the scheduler's subsequent `inc esi` can skip the object shifted into the just-processed index. This follows from verified scheduler/remover mechanics; this slice did not prove every class-specific self-removal path. | Scheduler `0x0055B610..0x0055B619` plus remover `0x0055BB09..0x0055BB21`. | Medium-High | Conditional: when a live `vtable+0x5C` body unregisters/removes an item at or before the current index |
| If a not-yet-visited earlier index is removed by some object AI, the same index arithmetic can shift later entries and change which object is next. No scheduler-side correction exists. | Same evidence as above. | Medium | Conditional: requires class-specific AI to remove from the same vector during the pass |

### 3.4 Other PerTick Loops Are Not the Same Contract

`PerTickUpdate` contains other loops before and after the main LogicClass object loop. They must not be generalized blindly.

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| A temporary/scratch object list built earlier in the function is iterated using a copied count stored on the stack, not the live source global count. | Build loop `0x0055B502..0x0055B580`; iteration uses `[esp+0x30]` at `0x0055B582..0x0055B59F`. | High | Yes |
| Some global arrays are iterated reverse from `count-1` down to `0`. | Reverse loops at `0x0055B5A1..0x0055B5BC` and `0x0055B5CD..0x0055B5E8`. | High | Yes |
| Several later global-array loops reload their global count after each `vtable+0x5C`, but they are separate arrays, not the LogicClass object vector at `0x0087F778`. | Examples `0x0055B634..0x0055B649`, `0x0055B675..0x0055B68B`, `0x0055B698..0x0055B6B1`. | High | Yes |

## 4. INI Keys

No INI key directly controls the scheduler loop, the count reload, or the add/remove helper behavior.

| Key / data source | Effect on this scheduler | Evidence | Active in YR |
|---|---|---|---|
| Object type data `ObjectTypeClass+0x234` | Controls whether reveal attempts LogicClass registration. This is type data, not a scheduler INI key. | Gate check: `ObjectClass::Reveal @ 0x005F4FEF..0x005F4FF5`; registration call at `0x005F5038..0x005F5040`; `BULLETTYPECLASS_GHIDRA_REPORT.md` | Yes, conditional by type |
| Projectile data such as `[AAHeatSeeker2]` | Makes the append-during-pass effect player-visible because fired bullets are logic-enabled and can be appended during a firing unit's AI. | `AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`; `rulesmd.ini` projectile rows in prior reports | Yes for stock YR projectiles |

## 5. Integration Points

| Integration point | Finding | Evidence | Active in YR |
|---|---|---|---|
| `Main_Tick` | Calls `LogicClass::PerTickUpdate` once in late housekeeping with `ECX=0x87F778`. | `0x0055DC99..0x0055DC9E`; timing docs | Yes |
| `ObjectClass::Reveal` | Can append logic-enabled objects to the same LogicClass vector. | `0x005F5038..0x005F5040`; helper report | Yes, conditional by type/gates |
| `ObjectClass::Conceal` / destruction paths | Can unregister objects from the same vector through the adjacent remover. | `0x005F4DD3`, `0x005F3D65..0x005F3D7A`; helper report | Yes |
| `BulletClass::Fire` path | Reveals/inserts bullets, giving the loop contract direct projectile timing consequences. | AAHeatSeeker2 latency report | Yes |
| Current Rust tick loop | Uses staged subsystem passes and many pre-collected ID snapshots, not a single live appendable object-AI vector. | `src/sim/world/mod.rs::advance_tick`; `src/sim/entity_store.rs`; `src/sim/movement/homing_movement.rs::tick_homing_movement` | Rust-facing implication |

## 6. Current Rust Implementation Status

| Rust surface | Current status | Evidence |
|---|---|---|
| `src/sim/entity_store.rs` | `EntityStore` stores all entities in a deterministic `BTreeMap<u64, GameEntity>` and exposes sorted key/value iteration. It is not a separate LogicClass-style active-object vector with membership bits. | `EntityStore` struct at `src/sim/entity_store.rs:33` (`entities: BTreeMap<u64, GameEntity>` @ :35, `by_owner` @ :39). (anchor corrected 2026-05-29: was `:31`; verified via Read of `src/sim/entity_store.rs` — behavioral claim unchanged.) |
| `src/sim/world/mod.rs::advance_tick` | Simulation advances through fixed subsystem phases. The scanned code does not provide one live, appendable, forward object-AI list equivalent to `LogicClass+0x04/+0x10`. | `advance_tick` at `src/sim/world/mod.rs:1508`. (anchor corrected 2026-05-29: was `:1008`; verified via grep `fn advance_tick` — behavioral claim unchanged.) |
| `src/sim/movement/homing_movement.rs::tick_homing_movement` | Homing movement snapshots `keys_sorted()` before iterating, so a homing entity inserted during that function's pass will not be processed by that same pass. | `tick_homing_movement` at `src/sim/movement/homing_movement.rs:379`, `keys_sorted()` snapshot at `:386`. (anchor corrected 2026-05-29: was `:261`; verified via grep — behavioral claim unchanged.) |
| Projectile spawn/detonation surfaces | Current projectile work is feature-specific and partly deferred per recent AAHeatSeeker2 reports; no core scheduler service contract was found in this slot. | Parent context plus static scan. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `PerTickUpdate` main LogicClass object loop direction | verified | `0x0055B5FB..0x0055B619` | none |
| Main loop count reload vs snapshot | verified | `0x0055B601..0x0055B619` | none |
| Main loop null/item guard behavior | verified | `0x0055B608..0x0055B610` | none |
| Append-during-main-loop behavior | verified | `0x0055B608..0x0055B619`; `0x00551A0A..0x00551A1D`; AAHeatSeeker2 latency report | none for scheduler; class-specific examples beyond bullets are out-of-scope |
| Registration helper duplicate guard | verified | `0x0055BAA5..0x0055BAB2`; slot-2 helper report | none |
| Remover compaction behavior | verified | `0x0055BAE0..0x0055BB2F`; slot-2 helper report | none |
| Remove-current skip implication | touched-not-exhausted | composed scheduler/remover evidence | A future class-specific AI/destructor trace should identify common live self-removal cases |
| Other `PerTickUpdate` global loops | touched-not-exhausted | `0x0055B502..0x0055B6B1` | Only contrasted with the main LogicClass loop; not a full report on every late-housekeeping array |
| TS legacy filter | verified for main loop | `Main_Tick` direct call in standard YR path; helper reports | none for scheduler activity |
| Save/load/replay reconstruction | deferred | not traced | Separate save/load lifecycle investigation |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-LCPU-001 - Is 0x0055AFB0 active in standard YR? -> Yes; Main_Tick calls it with ECX=0x87F778 in late housekeeping.` (evidence: `0x0055DC99..0x0055DC9E`; timing docs)
- `[RESOLVED] OQ-LCPU-002 - Which vector is the core object AI loop? -> LogicClass+0x04 items with count at +0x10.` (evidence: `0x0055B5FB..0x0055B619`)
- `[RESOLVED] OQ-LCPU-003 - What is the iteration order? -> Ascending index from 0.` (evidence: `0x0055B5FF`, `0x0055B608`, `0x0055B616`)
- `[RESOLVED] OQ-LCPU-004 - Does the loop snapshot count at entry? -> No for the main LogicClass object loop; count is reloaded after each vtable call.` (evidence: `0x0055B613..0x0055B619`)
- `[RESOLVED] OQ-LCPU-005 - Can an appended object run in the same pass? -> Yes when appended before the forward loop reaches the new tail.` (evidence: `0x0055B608..0x0055B619`; `0x00551A0A..0x00551A1D`)
- `[RESOLVED] OQ-LCPU-006 - Is append sorted? -> No for ordinary reveal; normal insert appends at old count.` (evidence: `0x00551A0A..0x00551A1D`)
- `[RESOLVED] OQ-LCPU-007 - Does repeated reveal duplicate entries? -> No; Object+0x98 short-circuits registration.` (evidence: `0x0055BAA5..0x0055BAB2`)
- `[RESOLVED] OQ-LCPU-008 - What happens on unregister? -> Count decrements and later entries shift left; +0x98 is cleared.` (evidence: `0x0055BB09..0x0055BB27`)
- `[RESOLVED] OQ-LCPU-009 - Does the scheduler repair the index after a vtable call? -> No; it only reloads count, increments index, and compares.` (evidence: `0x0055B610..0x0055B619`)
- `[RESOLVED] OQ-LCPU-010 - What is the visible remove-current implication? -> A compacting remove at/before the current index can skip a shifted object because the scheduler increments the index after the call.` (evidence: `0x0055B610..0x0055B619`; `0x0055BB09..0x0055BB21`)
- `[RESOLVED] OQ-LCPU-011 - Are all PerTick loops live-count forward loops? -> No; the function also has copied-count and reverse loops.` (evidence: `0x0055B582..0x0055B59F`; `0x0055B5A1..0x0055B5E8`)
- `[RESOLVED] OQ-LCPU-012 - Are there scheduler-side null checks for main-vector items? -> No.` (evidence: `0x0055B608..0x0055B610`)
- `[RESOLVED] OQ-LCPU-013 - Is this TS-only legacy? -> No; standard YR Main_Tick reaches the function and object reveal/conceal paths reach the same vector helpers.` (evidence: `0x0055DC99..0x0055DC9E`; helper report)
- `[RESOLVED] OQ-LCPU-014 - Does Rust currently have this exact scheduler contract? -> No equivalent live appendable LogicClass object-vector pass was found; scanned surfaces use `EntityStore` plus subsystem phases/snapshots.` (evidence: `src/sim/world/mod.rs:1508`; `src/sim/entity_store.rs:33`; `src/sim/movement/homing_movement.rs:379`) (anchors corrected 2026-05-29: were `:1008`/`:31`/`:261`; verified via grep/Read — behavioral claim unchanged.)
- `[DEFERRED] OQ-LCPU-015 - Which concrete class-specific AI bodies remove themselves or earlier objects mid-pass in common play?` (category: `requires-different-system-context`; reason: this target is the scheduler contract, not every `vtable+0x5C` implementation; next-step-if-pursued: trace high-frequency `AnimClass`, `BulletClass`, `TechnoClass`, and debris/self-delete AI exits.)
- `[DEFERRED] OQ-LCPU-016 - How is the LogicClass vector reconstructed across save/load and replay restore?` (category: `requires-different-system-context`; reason: persistence is not visible in `PerTickUpdate`; next-step-if-pursued: trace object load/reveal and LogicClass vector serialization.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Logic-enabled objects are processed by a live forward active-object vector whose count is reloaded after each object AI. | `0x0055B608..0x0055B619` | Missing/unchecked: `advance_tick` uses subsystem phases over `EntityStore`, and homing movement snapshots keys. | `src/sim/world/mod.rs::advance_tick`; future sim-level logic scheduler service. | A future LogicClass-equivalent pass should allow newly appended active objects to be seen before the pass ends. | Entity A's AI registers entity B; in the same scheduler pass, B's AI counter increments if A was before the new tail -> `logic_scheduler_append_during_pass_ticks_new_tail_same_tick`. | Do not implement newly spawned logic objects as universally "next tick only"; this breaks first-tick projectile timing. |
| Registration is object-membership-gated and tail-appending. | `0x0055BAA0`; `0x005519B0`; `Object+0x98` | Missing: no separate active logic-list membership bit/list was found. | `src/sim/entity_store.rs`; future `GameEntity` active-logic membership or scheduler-owned list. | Duplicate reveal/activation must not double-schedule an object, but a first activation appends at tail. | Re-register A twice, run one pass, assert one AI call and one membership entry -> `logic_scheduler_duplicate_registration_is_idempotent`. | Do not use `EntityStore` membership alone as the AI-list contract; concealed/unregistered stored objects are different from active logic objects. |
| Removal compacts the vector and the scheduler does not repair the current index after the vtable call. | Remover `0x0055BB09..0x0055BB21`; scheduler `0x0055B610..0x0055B619` | Missing/unchecked: future Rust active-list removal semantics are not defined. | Future logic scheduler list removal/despawn/conceal handling. | Removing B from A's AI preserves order of remaining entries; removing the current object follows gamemd-style index advancement semantics. | Register A,B,C; make B unregister itself; assert the pass behavior matches compacting-remove plus post-call index increment -> `logic_scheduler_self_unregister_uses_compacting_index_semantics`. | Do not use unordered swap-remove for the LogicClass-equivalent list; do not silently "fix" skipped-shift behavior without a class-specific parity reason. |

### Negative Facts / Do Not Do

- Do not snapshot the main LogicClass object count at pass entry. Active in YR: Yes; evidence: count reload at `0x0055B613`.
- Do not treat all `PerTickUpdate` loops as the same scheduler shape. Active in YR: Yes; evidence: copied-count loop at `0x0055B582..0x0055B59F`, reverse loops at `0x0055B5A1..0x0055B5E8`, main live-count loop at `0x0055B608..0x0055B619`.
- Do not implement the active logic list with `swap_remove`. Active in YR: Yes; evidence: remover shifts entries left at `0x0055BB11..0x0055BB21`.
- Do not assume `EntityStore` sorted order is equivalent to gamemd's active-object vector order. Active in YR: Yes; evidence: separate LogicClass vector `+0x04/+0x10` and tail append at `0x00551A0A..0x00551A1D`.
- Do not add scheduler-side null filtering for main-vector items unless a caller/list-integrity layer supplies it elsewhere. Active in YR: Yes; evidence: direct dereference/call at `0x0055B608..0x0055B610`.

### Remaining Uncertainty

- Class-specific common self-removal/destruction cases inside `vtable+0x5C` were not exhausted. The scheduler/remover mechanics are verified; the frequency and player-visible cases need targeted follow-up.
- Save/load and replay restoration of active logic-list membership were not traced.
- This report does not assign concrete class names to every non-object global loop inside `PerTickUpdate`; it only contrasts their scheduler shape with the main LogicClass object loop.

### Stale Docs / Follow-up Docs

- `docs/research/UNITCLASS_GHIDRA_REPORT.md:318` currently says: `LogicClass::AI() tick loop -> iterates all entities -> calls AI on each.` Replace with: `LogicClass::PerTickUpdate @ 0x0055AFB0 contains the per-object active-vector loop; it iterates the LogicClass-owned object vector forward and calls vtable+0x5C, re-reading count after each call. LogicClass::AI is the input/event dispatcher, not this object-AI loop.`
- `docs/research/INFANTRYCLASS_GHIDRA_REPORT.md:335` has the same stale wording. Use the same replacement text.
- `docs/research/timing/logic-vs-render-loop.md` contains both the corrected PerTickUpdate loop and an overview sentence saying pause freezes the unit-AI loop. If revised, replace that overview sentence with: `Pause skips the gameplay/input/render block, but the late LogicClass::PerTickUpdate path still runs; any unit/object AI reached through that late vtable+0x5C loop advances unless the class-specific AI body gates itself.`

## Sources

- Direct read-only binary disassembly from `<ra2-install>/gamemd.exe`:
  - `LogicClass::PerTickUpdate @ 0x0055AFB0`
  - Main object loop `0x0055B5FB..0x0055B619`
  - Other loop contrast ranges `0x0055B502..0x0055B6B1`
  - `Main_Tick` call site `0x0055DC99..0x0055DC9E`
  - `FUN_0055BAA0`, `0x0055BAE0`, `DynamicVector__Insert @ 0x005519B0`
- Prior reports referenced:
  - `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
  - `docs/research/AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`
  - `docs/research/timing/logic-vs-render-loop.md`
  - `docs/research/BULLETTYPECLASS_GHIDRA_REPORT.md`
  - `docs/research/LOGICCLASS_VS_MAPCLASS_GHIDRA_REPORT.md`
- Rust surfaces scanned read-only:
  - `src/sim/world/mod.rs`
  - `src/sim/entity_store.rs`
  - `src/sim/movement/homing_movement.rs`
