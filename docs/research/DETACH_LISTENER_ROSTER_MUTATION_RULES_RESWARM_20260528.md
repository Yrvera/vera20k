# Detach Listener Roster Mutation Rules - Re-swarm Research Report

**Address(es):** `Detach_From_All_Lists @ 0x007258D0`, `ObjectClass::Constructor @ 0x005F3900`, `ObjectClass::~ObjectClass @ 0x005F3B80`, `DynamicVector__Insert @ 0x005519B0`, `FUN_00551A30 @ 0x00551A30`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** mutation behavior of the broad `DAT_00B0F724` listener roster during the `Detach_From_All_Lists` object branch: count reload, buffer reload, append visibility, remove/left-compaction effects, and ObjectClass constructor/destructor mutation paths.  
**Non-Scope:** full listener roster census, full callback-body side effects, House/Team/Factory/Anim special-roster mutation rules outside `DAT_00B0F724`, runtime debugger proof of a stock callback constructing/destructing an ObjectClass during the same `DAT_00B0F724` pass, and Rust implementation.
**Confidence:** High for loop mechanics, append/remove mechanics, and Rust handoff rules; Medium for concrete stock-frequency of same-pass mutation because this report proves the mechanism statically, not a runtime watchpoint sample.  
**Active in YR:** Yes. `ObjectClass::UnInit` reaches `Detach_From_All_Lists` before conceal/alive clear per the parent report, and Object-derived runtime classes append to `DAT_00B0F724` through the active full constructor.

## Working Notes Gate

Target question: How does `Detach_From_All_Lists` iterate the broad listener roster around `DAT_00B0F724`, and what do count reload, append/remove, constructor/destructor mutation, and left-compaction imply for a Rust listener registry?

Non-goals: Do not re-census every listener class; do not re-prove ObjectClass `UnInit` ordering, Bullet target invalidation, or LogicClass membership; do not inspect Ghidra mutably; do not edit Rust, INI, claims, or sibling docs.

Evidence needed to mark COMPLETE: decompile plus assembly context for the `DAT_00B0F724` loop; decompile plus assembly context for `ObjectClass::Constructor` append and `ObjectClass::~ObjectClass` remove; source scan for current Rust storage/cleanup shape; explicit mutation-rule handoff and negative facts.

Stop conditions: stop after the loop and ObjectClass append/remove mutation rules are proven and translated into Rust-facing constraints; defer runtime watchpoint examples of callbacks mutating this roster.

## 1. Overview

The `DAT_00B0F724` object-branch roster in `Detach_From_All_Lists` is iterated as a live, mutable vector, not as a snapshot. The loop starts at index `0`, calls the listener at the current index through primary vtable `+0x28`, then reloads `DAT_00B0F730` before increment/continue. It also reloads the vector buffer pointer before each callback.

Active in YR: Yes. Evidence: `Detach_From_All_Lists @ 0x007258D0`; loop assembly context `0x0072593E..0x0072595F`; call path from active `ObjectClass::UnInit` is already settled by parent context and `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`.

## 2. Roster Layout

| Field | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00B0F720` | DynamicVector-like method table / vector object base used for grow/find operations. | Constructor grow call uses `[0x00B0F720 + 0x08]`; destructor find call uses `[0x00B0F720 + 0x10]`. | Yes |
| `DAT_00B0F724` | pointer buffer for broad removal listeners. | `Detach_From_All_Lists` loads it at `0x00725947`; constructor writes into it at `0x005F3A85..0x005F3A8B`; destructor compacts it at `0x005F3C6E..0x005F3C84`. | Yes |
| `DAT_00B0F728` | capacity. | Constructor compares capacity vs count at `0x005F3A3B..0x005F3A48`. | Yes |
| `DAT_00B0F730` | live count. | Loop reads before/after callbacks at `0x0072593E` and `0x00725957`; constructor increments at `0x005F3A7E..0x005F3A7F`; destructor decrements at `0x005F3C63..0x005F3C66`. | Yes |
| `DAT_00B0F734` | growth increment. | Constructor reads it before vector grow at `0x005F3A56..0x005F3A6F`. | Yes |

## 3. Core Logic

### 3.1 `Detach_From_All_Lists` uses live count reload, not a captured count

Verified behavior: for Object-registered targets, the broad roster loop is:

1. Check `target != null` and `AbstractFlags+0x14` bit `0x02`.
2. Read `DAT_00B0F730`; if count is `<= 0`, skip the loop.
3. For index `i`, reload `DAT_00B0F724`, read listener `buffer[i]`, call listener vtable `+0x28(target, removal_flag)`.
4. Reload `DAT_00B0F730` after the callback.
5. Increment `i`; continue while `i < reloaded_count`.

Evidence:

| Detail | Address / assembly context | Active in YR |
|---|---|---|
| Initial count read before first callback. | `0x0072593E: MOV EAX,[0x00B0F730]`; `0x00725943..0x00725945` tests/skips. | Yes |
| Buffer pointer is loaded inside the loop body before each callback. | `0x00725947: MOV ECX,[0x00B0F724]`; `0x0072594F` reads `[ECX + EBP*4]`. | Yes |
| Callback slot is listener primary vtable `+0x28`. | `0x00725952` reads listener vtable; `0x00725954: CALL dword ptr [EDX + 0x28]`. | Yes |
| Count is reloaded after callback, before the loop continuation test. | `0x00725957: MOV EAX,[0x00B0F730]`; `0x0072595C..0x0072595F` increments index, compares to reloaded count, jumps back to `0x00725947`. | Yes |

Implementation consequence: Rust must not snapshot the listener vector length at dispatch start if it is trying to match native same-pass mutation semantics. A snapshot would suppress callbacks to listeners appended during the pass and would not reproduce count-shrink early termination.

### 3.2 Append during a callback can be observed in the same pass

Verified mechanism: because the loop reloads `DAT_00B0F730` after each callback and appends write at the old end then increment count, a listener appended at index `old_count` before the loop reaches the old end can become eligible for the same pass.

Example from the verified mechanism, not a live runtime sample:

- Start count `N`, current callback at index `i`.
- Callback appends one listener, making count `N+1`.
- `Detach_From_All_Lists` reloads count after callback, increments `i`, and continues while `i+1 < N+1`.
- If the loop reaches index `N`, the appended listener is called in the same pass.

Active in YR: Yes for the dispatch mechanism; Conditional for occurrence in a stock scenario. Evidence: loop count reload `0x00725957`; ObjectClass append to the same vector `0x005F3A76..0x005F3A8B`. Runtime frequency requires a debugger watchpoint.

### 3.3 Remove during a callback can skip shifted listeners

Verified mechanism: ObjectClass destructor removal uses find, decrements count, and left-compacts entries above the removed index. Since `Detach_From_All_Lists` increments the current loop index after the callback and compares against the new count, removals affect the current pass.

Implications by removed index:

| Mutation during callback at current index `i` | Same-pass effect | Evidence | Active in YR |
|---|---|---|---|
| Remove an entry at index `< i`. | Entries after the removed slot shift down; after callback the loop increments from old `i` to `i+1`, so the next original suffix entry can be skipped. | Destructor compaction `0x005F3C63..0x005F3C84`; loop increment/count reload `0x00725957..0x0072595F`. | Yes mechanism; occurrence Conditional |
| Remove the current listener at index `i`. | The original `i+1` listener shifts into `i`; the loop increments to `i+1`, so that shifted successor is skipped. | Same as above. | Yes mechanism; occurrence Conditional |
| Remove a later entry at index `> i`. | The removed later entry is not called; following entries compact left and may still be visited if their shifted index is at or beyond the next loop index. | Same as above. | Yes mechanism; occurrence Conditional |
| Remove enough entries to make `new_count <= i+1`. | Loop terminates early after callback. | Count reload `0x00725957`; compare `0x0072595D..0x0072595F`. | Yes mechanism; occurrence Conditional |

This is not a stable-iterator model. It is index-over-live-array with count and buffer reload.

### 3.4 ObjectClass constructor appends to the same vector

Verified behavior: the full `ObjectClass::Constructor @ 0x005F3900` appends `this` (`ESI`) to `DAT_00B0F724` after the global ObjectClass array append and before the master abstract/tag listener appends.

Assembly-confirmed sequence:

| Step | Address / evidence | Active in YR |
|---|---|---|
| Compare count against capacity. | `0x005F3A3B` loads `DAT_00B0F728`; `0x005F3A40` loads `DAT_00B0F730`; `0x005F3A46..0x005F3A48` compares and jumps to append when capacity remains. | Yes |
| Grow when needed and allowed. | `0x005F3A56..0x005F3A6F` reads `DAT_00B0F734`, pushes args, calls `[DAT_00B0F720 + 0x08]`; `0x005F3A72..0x005F3A74` tests success before append. | Yes |
| Append at old count then increment count. | `0x005F3A76` loads old `DAT_00B0F730`; `0x005F3A7E` increments; `0x005F3A7F` stores new count; `0x005F3A85..0x005F3A8B` writes `ESI` into `DAT_00B0F724[old_count]`. | Yes |

Active in YR: Yes. Object-derived classes call this constructor in normal runtime creation paths (for example bullets/anims/units/terrain per prior roster census), and the constructor itself mutates the same vector used by the broad removal broadcast.

### 3.5 ObjectClass destructor removes from the same vector by left compaction

Verified behavior: `ObjectClass::~ObjectClass @ 0x005F3B80` removes `this` from `DAT_00B0F724` using the vector find method at vtable `+0x10`, decrements `DAT_00B0F730`, then shifts every later pointer down one slot.

Assembly-confirmed sequence:

| Step | Address / evidence | Active in YR |
|---|---|---|
| Find this object in the vector. | `0x005F3C48` sets `ECX=0x00B0F720`; `0x005F3C51` calls `[DAT_00B0F720 + 0x10]`; result index is in `EAX`. | Yes |
| Ignore not-found or out-of-range index. | `0x005F3C54..0x005F3C61` checks `EAX != -1` and `EAX < DAT_00B0F730`. | Yes |
| Decrement count before compaction. | `0x005F3C63` decrements `ECX`; `0x005F3C66` stores `DAT_00B0F730 = count - 1`. | Yes |
| Left-compact subsequent entries. | `0x005F3C6E..0x005F3C84` repeatedly loads `DAT_00B0F724`, increments source index, copies `[source]` to `[source - 1]`, reloads count, and loops while source index `< DAT_00B0F730`. | Yes |

Active in YR: Yes. Destructors are reached by active pending-delete cleanup; ObjectClass destructor also removes from pending-delete, object array, abstract registry, and tag listener vector, but this report only claims the `DAT_00B0F724` removal rule.

### 3.6 `DynamicVector__Insert` matches append-at-count semantics

`DynamicVector__Insert @ 0x005519B0` confirms the engine's generic unsorted append convention: grow if needed, read count at `+0x10`, store `count + 1`, then write the new element to `buffer[old_count]` at `+0x04`.

Active in YR: Yes as a general vector helper where called. Evidence: decompile `0x005519B0`; assembly context `0x00551A0A..0x00551A1D`.

### 3.7 `FUN_00551A30` is not a listener or LogicClass membership helper

`FUN_00551A30 @ 0x00551A30` sorts adjacent entries by comparing each pair's vtable `+0xB8` value, using `param_1+0x04` as a buffer and `param_1+0x10` as count. It is not the `DAT_00B0F724` listener loop and not the LogicClass active-vector add/remove helper.

Active in YR: Conditional; it is active where render/layer ordering calls it, but it is not part of this listener mutation mechanism. Evidence: decompile `0x00551A30`; already-settled parent fact that `FUN_00551A30` is render layer, not LogicClass.

## 4. Current Rust Implementation Status

Focused scan only, no code edits:

| Rust surface | Current shape | Delta for this mechanism |
|---|---|---|
| `src/sim/entity_store.rs:27`, `src/sim/entity_store.rs:57`, `src/sim/entity_store.rs:107` | Primary entity storage is `BTreeMap<u64, GameEntity>`; remove deletes by stable id; iteration helpers snapshot sorted keys. | Not a native listener roster. Stable-id sorted snapshots do not reproduce live vector index/compaction mutation rules. |
| `src/sim/world/mod.rs:612`, `src/sim/world/mod.rs:618`, `src/sim/world/mod.rs:622` | `live_object_order` push/retain plus snapshot helper. | Similar ownership surface for ordered runtime lists, but it does not model `DAT_00B0F724` live listener dispatch or same-pass append/remove behavior. |
| `src/sim/world/mod.rs:675`, `src/sim/world/mod.rs:695..697` | `despawn_entity` clears radio contacts, removes the entity, then unregisters live object order. | Native dispatch is pre-conceal/pre-alive-clear and vector-mutating; Rust currently removes storage and unregisters immediately in this path. |
| `src/sim/combat/mod.rs:1005..1006`, `src/sim/combat/mod.rs:1115..1125` | Death handling clears radio contacts and entity attack targets by stable id; `clear_targets_on_dead_entity` snapshots keys. | Covers only one ref family and uses snapshot order; no broad listener registry with native same-pass mutation semantics. |
| `src/sim/game_entity.rs:173`, `src/sim/game_entity.rs:181`, `src/sim/game_entity.rs:187`, `src/sim/game_entity.rs:343` | Movement target, attack target, radio contacts, capture target are fields on entities. | Future listener callbacks must update role-specific refs, not simply delete missing ids after storage removal. |
| `src/sim/passenger.rs:93`, `src/sim/passenger.rs:103`, `src/sim/passenger.rs:478`, `src/sim/passenger.rs:729` | Passenger vectors use Rust `Vec` remove and direct role transitions; boarding hide clears radio contacts and target fields. | Passenger/cargo cleanup exists but is not wired through native pre-conceal listener dispatch or broad roster mutation rules. |

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `DAT_00B0F724` loop count reload | verified | decompile `0x007258D0`; assembly `0x0072593E..0x0072595F` | none |
| `DAT_00B0F724` loop buffer reload | verified | assembly `0x00725947` before each callback | none |
| listener callback slot and argument order | verified | decompile `0x007258D0`; assembly `0x0072594D..0x00725954` pushes removal flag and target before vtable `+0x28` call | full body effects out of scope |
| ObjectClass full constructor append | verified | decompile `0x005F3900`; assembly `0x005F3A3B..0x005F3A8B` | exact class caller frequency per scenario needs runtime census |
| ObjectClass destructor removal/compaction | verified | decompile `0x005F3B80`; assembly `0x005F3C48..0x005F3C84` | exact destructor timing is covered by pending-delete reports, not repeated here |
| Same-pass append/remove consequences | verified mechanism | loop + append/remove evidence above | runtime callback watchpoint for stock occurrence |
| `DynamicVector__Insert` append convention | verified | decompile `0x005519B0`; assembly `0x00551A0A..0x00551A1D` | none |
| `FUN_00551A30` non-listener status | verified for scoped negative | decompile `0x00551A30`; parent settled fact | caller census out of scope |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does the broad roster loop snapshot count before dispatch? -> No. It reloads `DAT_00B0F730` after each callback.` (evidence: `0x00725957`; Active in YR: Yes)
- `[RESOLVED] OQ-02 - Does the loop reload the vector buffer? -> Yes. It reloads `DAT_00B0F724` at the top of each callback iteration.` (evidence: `0x00725947`; Active in YR: Yes)
- `[RESOLVED] OQ-03 - Can append during a callback affect the same pass? -> Yes by mechanism: appended entries increase the reloaded count and can be reached later in the same forward pass.` (evidence: `0x00725957`, `0x005F3A76..0x005F3A8B`; Active in YR: Yes mechanism / Conditional occurrence)
- `[RESOLVED] OQ-04 - Can remove during a callback affect the same pass? -> Yes by mechanism: destructor removal decrements count and left-compacts; the forward loop then increments the old index and can skip shifted entries or terminate early.` (evidence: `0x00725957..0x0072595F`, `0x005F3C63..0x005F3C84`; Active in YR: Yes mechanism / Conditional occurrence)
- `[RESOLVED] OQ-05 - Does ObjectClass full constructor mutate the same vector? -> Yes, it appends `this` to `DAT_00B0F724` and increments `DAT_00B0F730`.` (evidence: `0x005F3900`, `0x005F3A76..0x005F3A8B`; Active in YR: Yes)
- `[RESOLVED] OQ-06 - Does ObjectClass destructor mutate the same vector? -> Yes, it removes `this` from `DAT_00B0F724` by find and left-compaction.` (evidence: `0x005F3B80`, `0x005F3C48..0x005F3C84`; Active in YR: Yes)
- `[RESOLVED] OQ-07 - Is `FUN_00551A30` the listener/LogicClass mutation helper? -> No; it sorts a vector by vtable `+0xB8` and is outside this roster rule.` (evidence: decompile `0x00551A30`; Active in YR: Conditional render-layer use)
- `[DEFERRED] OQ-08 - Which stock callback bodies construct or destruct ObjectClass instances during this exact `DAT_00B0F724` loop?` (category: `needs-runtime-debugger`; reason: static code proves same-pass mutation rules, but a stock runtime watchpoint is needed to rank frequency; next-step-if-pursued: set watchpoints on `0x00B0F730` and `0x00B0F724` while destroying an object with active bullets/anims/temporal/particle/house refs)
- `[DEFERRED] OQ-09 - Do House/Anim/special RTTI branches use identical mutation semantics?` (category: `out-of-scope`; reason: this target is the broad `DAT_00B0F724` object branch; next-step-if-pursued: repeat this slice for `g_AnimClass_RemoveListeners`, `g_HouseClass_RemoveListeners`, and tag/factory/team vectors)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Broad object-expiry listener dispatch is live-vector iteration: reload buffer before each callback, reload count after each callback, increment index, compare to reloaded count. Active in YR: Yes. | `Detach_From_All_Lists @ 0x007258D0`; assembly `0x00725947`, `0x00725954`, `0x00725957..0x0072595F` | Missing; Rust cleanup uses stable-id key snapshots and direct id removals. | future listener registry in `sim/`, death/despawn flow in `src/sim/world/mod.rs` and `src/sim/combat/mod.rs` | Dispatch listeners from an ordered mutable registry with native index/count semantics, not from a pre-collected `Vec` snapshot, when exact parity is required. | Listener A appends listener D during death broadcast; after original listeners B/C, D is called in the same pass if still within the reloaded count. Proposed test: `detach_listener_append_is_seen_in_same_pass`. | Snapshotting listener IDs at pass start will drift. |
| ObjectClass constructor appends to the same roster at old count and increments count; destructor removes by find, decrements count, and left-compacts. Active in YR: Yes. | Constructor `0x005F3A76..0x005F3A8B`; destructor `0x005F3C48..0x005F3C84` | Missing as a distinct roster; `EntityStore` insert/remove and `live_object_order` are separate abstractions. | future native object lifecycle layer; `src/sim/entity_store.rs`; `src/sim/world/mod.rs` live order and despawn paths | Keep listener registration separate from entity storage and unregister by native left-compaction order, preserving duplicate/not-found behavior if modeled later. | Listener B removes itself during callback; listener C shifts into B's old index and is skipped by the current pass, matching native. Proposed test: `detach_listener_self_remove_skips_shifted_successor`. | Do not use swap-remove, BTreeMap sorted order, or retain-on-snapshot as a substitute. |
| Count shrink can terminate the current pass early; removing entries before/current index can skip shifted suffix entries. Active in YR: Yes mechanism / Conditional occurrence. | Loop `0x00725957..0x0072595F`; compaction `0x005F3C63..0x005F3C84` | Unchecked/missing; current Rust generally iterates snapshots or direct BTreeMap order. | listener registry iteration primitive and tests | Define mutation-visible cursor semantics explicitly: cursor is an index into the current vector after callback, not a stable listener identity. | Four listeners A/B/C/D; while C is called, C removes A; pass ends or skips D according to native count/index result. Proposed test: `detach_listener_remove_prior_entry_uses_native_index_compaction`. | Do not "fix" skips by decrementing cursor after removal unless binary evidence for that exists; this loop does not repair the index. |

## 8. Negative Facts / Do Not Do

- Do not snapshot `DAT_00B0F724` length or listener IDs at broadcast start. Active in YR: Yes; evidence: count reload after callback at `0x00725957`.
- Do not use `BTreeMap` stable-id iteration order as a proxy for the native listener roster. Active in YR: Yes; evidence: native vector append order `0x005F3A76..0x005F3A8B`, current Rust storage `src/sim/entity_store.rs:27`.
- Do not use swap-remove for listener unregistration. Active in YR: Yes; evidence: ObjectClass destructor left-compacts at `0x005F3C6E..0x005F3C84`.
- Do not repair the cursor after self-removal or prior-entry removal unless a different branch proves that behavior. Active in YR: Yes; evidence: the loop simply increments `EBP` after callback at `0x0072595C`.
- Do not treat `FUN_00551A30` as `LogicClass` or listener membership. Active in YR: Conditional render-layer use; evidence: decompile `0x00551A30` sorts by vtable `+0xB8`, while listener append/remove is at `0x005F3A76..0x005F3C84`.

## 9. Remaining Uncertainty

- No runtime watchpoint sample was collected proving which stock callback body mutates `DAT_00B0F724` during the same broad-roster pass. The static same-pass semantics are verified; stock frequency remains Conditional.
- This report does not prove mutation rules for every special RTTI branch (`g_AnimClass_RemoveListeners`, `g_HouseClass_RemoveListeners`, factory/team/tag/trigger vectors). Several appear structurally similar, but this report only claims `DAT_00B0F724`.
- Exact allocator/grow failure behavior during constructor append is decoded enough for append/no-append, but no runtime out-of-memory scenario was sampled.

## 10. Stale Docs / Follow-up Docs

- `docs/research/DETACH_FROM_ALL_LISTS_LISTENER_ROSTER_CENSUS_RESWARM_20260528.md`: replace "Tiny ordering detail: the `DAT_00B0F724` loop reloads `DAT_00B0F730` in the loop condition after each callback. This proves forward index order, but not safe behavior if a callback mutates the same vector. Active in YR: Yes; mutation behavior remains deferred." with: "The `DAT_00B0F724` loop is live-vector iteration: it reloads the buffer before each callback and reloads `DAT_00B0F730` after each callback. ObjectClass construction appends to the same vector, and ObjectClass destruction removes by count decrement plus left-compaction. Therefore append during a callback can add same-pass callbacks, while removal can skip shifted listeners or terminate the pass early; stock runtime frequency still needs a watchpoint."
- `docs/research/TEMPORAL_SQDG_REMOVELISTENER_LIFECYCLE_GHIDRA_REPORT.md`: replace "Runtime mutation safety if listener callback edits `g_AnimClass_RemoveListeners` during `Detach_From_All_Lists` iteration needs debugger instrumentation." with: "The broad `DAT_00B0F724` branch has verified live-vector mutation semantics; the separate `g_AnimClass_RemoveListeners` branch still needs the same focused mutation audit before assuming identical behavior."

## Sources

- Ghidra read-only decompile: `Detach_From_All_Lists @ 0x007258D0`, `ObjectClass::Constructor @ 0x005F3900`, `ObjectClass::~ObjectClass @ 0x005F3B80`, `DynamicVector__Insert @ 0x005519B0`, `FUN_00551A30 @ 0x00551A30`.
- Ghidra read-only assembly contexts: `0x0072593E..0x0072595F`, `0x005F3A3B..0x005F3A8B`, `0x005F3C48..0x005F3C84`, `0x00551A0A..0x00551A1D`.
- Prior reports read/cross-checked: `DETACH_FROM_ALL_LISTS_LISTENER_ROSTER_CENSUS_RESWARM_20260528.md`, `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`, `BULLETCLASS_TARGET_INVALIDATION_AAHEATSEEKER2_GHIDRA_REPORT.md`, `TEMPORAL_SQDG_REMOVELISTENER_LIFECYCLE_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/world/mod.rs`, `src/sim/entity_store.rs`, `src/sim/game_entity.rs`, `src/app_sim_tick.rs`, `src/sim/combat/*`, `src/sim/passenger.rs`.
