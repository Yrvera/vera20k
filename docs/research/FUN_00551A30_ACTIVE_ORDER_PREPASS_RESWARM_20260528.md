# FUN_00551A30 Active-Order Prepass - Reswarm 2026-05-28

**Address(es):** `FUN_00551A30 @ 0x00551A30`, active caller `Main_Tick @ 0x0055D360`, call site `0x0055DBC3..0x0055DBC8`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact semantics of `FUN_00551A30`, the vector instance passed by `Main_Tick`, count/index behavior, mutation behavior, direct comparison with the LogicClass active-vector helpers, and Rust scheduler/render implications.  
**Non-Scope:** re-proving `FUN_0055BAA0`/`FUN_0055BAE0` beyond direct comparison, replay startup/restore, full `LayerClass` lifecycle, or Rust implementation.  
**Confidence:** High for helper body, call target vector, active YR reachability, direct non-LogicClass conclusion, and Rust-facing implications.  
**Active in YR:** Yes. `Main_Tick` reaches `MOV ECX,0x8A0390; CALL 0x00551A30` on the standard tick path before `LogicClass::PerTickUpdate`; replay playback also reaches it after playback render.

## 0. Investigation Contract

**Target question:** What exactly does `FUN_00551A30` do to the active-order vector before `LogicClass::PerTickUpdate` in `Main_Tick`/replay paths?  
**Non-goals:** Do not re-prove `FUN_0055BAA0` or `FUN_0055BAE0` except direct comparison; do not investigate replay startup beyond call context.  
**Evidence needed to mark COMPLETE:** decompile plus disassembly for `0x00551A30`, caller/xref evidence for active YR tick/replay path, callee comparison where directly touched, Rust scheduler scan, and no unresolved material open questions.  
**Stop conditions:** stop after exact helper semantics and integration implications are proven, or mark PARTIAL if Ghidra/Rust evidence cannot establish bounds, offsets, path liveness, or mutation behavior.

## 1. Overview

`FUN_00551A30` is a one-pass adjacent repair over the ground display-layer vector at `0x008A0390`, not the `LogicClass` active-object AI vector at `0x0087F778`. It walks adjacent object pointers, calls each object's `vtable+0xB8` `GetYSort` key, and swaps the pair when the later element has a smaller key than the earlier element.

The helper does not append, remove, compact, filter, deduplicate, clear membership flags, or fully sort the vector in one call. It is a single forward bubble pass over live `Count`, so it can move an out-of-order object only one or more adjacent positions per tick depending on how many inversions it crosses during that pass.

## 2. Class Layout / Key Offsets

`Main_Tick` passes `ECX=0x008A0390`. Existing `LayerClass` research and direct caller assembly identify this as `g_DisplayLayers[2]`, the Ground display layer. Active in YR: Yes; this is the standard tactical ground draw layer.

| Offset | Field | Type | `FUN_00551A30` use | Evidence | Active in YR |
|---:|---|---|---|---|---|
| `+0x04` | `Items` | `ObjectClass**` | read for `Items[i]`/`Items[i+1]`; re-read before swap writes | decompile `0x00551A30`; assembly `0x00551A41`, `0x00551A69..0x00551A73` | Yes |
| `+0x10` | `Count` | signed `int` | read at entry and loop bottom; `Count - 1` is signed-compared | assembly `0x00551A37..0x00551A3D`, `0x00551A76..0x00551A7D` | Yes |
| object vtable `+0xB8` | `GetYSort` | virtual returns signed `int` | called on `Items[i+1]` first, then `Items[i]` | assembly `0x00551A4B..0x00551A5D`; `ObjectClass::GetYSort @ 0x005F6BD0` | Yes |
| `ObjectClass+0x98` | LogicClass membership byte | byte | not read or written | negative scan of `0x00551A30..0x00551A84`; direct comparison to `0x0055BAA0/0x0055BAE0` | No for this helper |

## 3. Core Logic

Assembly-verified behavior of `FUN_00551A30 @ 0x00551A30`:

```text
if signed(Count - 1) <= 0:
    return

i = 0
do:
    next = Items[i + 1]
    cur  = Items[i]
    next_key = next.vtable[0xB8]()
    cur_key  = cur.vtable[0xB8]()
    if next_key < cur_key:
        Items[i + 1] = cur
        Items[i]     = next
    i += 1
while i < signed(Count - 1)
```

Tiny details:

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| The vector is `0x008A0390`, not `0x0087F778`. | caller assembly `0x0055DBC3 MOV ECX,0x8A0390`; `0x0055DBC8 CALL 0x00551A30`; `0x0055DC99 MOV ECX,0x87F778` then `CALL 0x0055AFB0` | High | Yes |
| The helper is a single adjacent-swap pass, not a full sort loop to fixed point. | one forward loop `0x00551A41..0x00551A7D`; no outer loop | High | Yes |
| Empty, one-element, and negative-count vectors return without touching `Items`. | `MOV EAX,[EBP+0x10]; DEC EAX; TEST EAX,EAX; JLE 0x00551A81` | High | Yes |
| The comparison is signed `next_key < cur_key`. | `CMP [ESP+0x10],EAX; JGE skip` after `next_key` saved and `cur_key` returned | High | Yes |
| Equal keys are stable for this pass. | swap only on `<`; `JGE` skips swap for equal or greater `next_key` | High | Yes |
| Null object pointers are not guarded. | unconditional `MOV EDX,[ESI]` and `MOV EAX,[EDI]` before virtual calls | High | Yes, relying on layer invariant |
| `Count` is live-reloaded at the bottom, not snapshotted for the full pass. | `MOV EAX,[EBP+0x10]` at `0x00551A76` before loop compare | High | Yes |
| `Items` is reloaded before swap writes after the virtual calls. | `MOV ECX,[EBP+4]`; write `Items[i+1]`; `MOV EDX,[EBP+4]`; write `Items[i]` | High | Yes |
| The current pair's object pointers are loaded before either `GetYSort` call. | `MOV ESI,[Items+i*4+4]`; `MOV EDI,[Items+i*4]` before calls | High | Yes |
| The helper never changes `Count`, `Capacity`, `CapacityIncrement`, or allocation flags. | no writes to vector offsets except through `Items` slots | High | Yes |
| The helper does not compact stale/dead entries or remove missing objects. | no `InWhichPosition`, no shift-left loop, no count decrement | High | Yes |
| The helper does not set/clear `ObjectClass+0x98`. | no object field access except vtable dereference and virtual calls | High | Yes |

### Relation to `DynamicVector__SortedInsert`

`DynamicVector__SortedInsert @ 0x00551A90` inserts a new element before the first existing element whose `GetYSort` is greater, preserving ascending `GetYSort` order at insertion time. Active in YR: Yes for Ground layer submit paths.

`FUN_00551A30` is different: it does not allocate, scan for an insertion position, shift a suffix, or increment `Count`; it only repairs adjacent inversions already present in the `Items` array. Active in YR: Yes, because `Main_Tick` calls it unconditionally on the Ground layer once the function reaches that part of the tick.

## 4. INI Keys

No INI key is read by `FUN_00551A30`. The keys that can influence `GetYSort` are indirect render/layer data such as `Layer=` and `YSortAdjust=` through object/anim setup, but this helper only consumes the runtime virtual `GetYSort` return. Active in YR: Yes; the helper is data-independent at the call site.

## 5. Integration Points

| Point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| only direct caller | Ghidra caller query returns only `Main_Tick @ 0x0055D360` | `get_function_callers 0x00551A30` | Yes |
| standard call order | replay block/render side work can run, then `FUN_00551A30`, then optional scenario launch side work, then `LogicClass::PerTickUpdate` | decompile `Main_Tick`; assembly `0x0055DBC3..0x0055DC9E` | Yes |
| replay playback liveness | playback render at `0x0055DBBE` is followed by `MOV ECX,0x8A0390; CALL 0x00551A30`, then can reach PerTick | assembly `0x0055DBB9..0x0055DC9E`; `DAT_00A8D5F8 & 2` branch in decompile | Conditional: replay playback |
| scenario intro/display-only early return | the earlier `ScenarioClass+0x62C` branch renders/waits/returns before this helper | `Main_Tick` decompile early return path | Conditional: scenario intro/display-only |
| `LogicClass` active vector | `LogicClass::PerTickUpdate` receives `ECX=0x87F778` later at `0x0055DC99` | assembly `0x0055DC99..0x0055DC9E` | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Delta for this slice |
|---|---|---|
| `src/sim/world/mod.rs` `live_object_order` / `live_object_order_snapshot` | owns a Rust surrogate for `LogicClass` active-object AI order and appends sorted fallback entity IDs | `FUN_00551A30` does not justify sorting/repairing this vector; it is a display Ground-layer prepass, not an AI active-vector prepass |
| `src/sim/world/mod.rs::advance_tick` | staged simulation phases start with movement/combat/etc.; no native display-layer prepass exists | no sim scheduler delta for `live_object_order`; render display-layer parity remains separate |
| `src/app_sim_tick.rs` | calls `sim.advance_tick`, then animation ticks and render-facing drains | no call equivalent to native `FUN_00551A30` before `PerTickUpdate`; if modeled, it belongs to display/render layer state, not sim AI order |
| `src/app_render/build_instances.rs` | rebuilds render instance vectors and fully sorts pages/lists each frame with `sort_by_depth_desc` | differs from native persistent Ground layer plus one-pass adjacent repair; may hide native transient one-pass ordering states |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00551A30` body | verified | decompile and capstone assembly `0x00551A30..0x00551A84` | none |
| call target vector | verified | caller assembly `0x0055DBC3..0x0055DBC8`; `LayerClass` report `0x008A0390` Ground | none |
| call order before PerTick | verified | `0x0055DBC8` before `0x0055DC99..0x0055DC9E` | none |
| replay playback reachability | verified for call context | `Main_Tick` decompile; `0x0055DBBE..0x0055DC9E` | full replay startup/restore out-of-scope |
| direct comparison to `FUN_0055BAA0` | verified enough for negative | decompile `0x0055BAA0`; prior helper report | no broader registration re-proof |
| direct comparison to `FUN_0055BAE0` | verified enough for negative | decompile `0x0055BAE0`; prior reswarm report | no broader unregister re-proof |
| `GetYSort` key identity | verified for base virtual | `ObjectClass::GetYSort @ 0x005F6BD0`; vtable docs | per-derived override census out-of-scope |
| Rust scheduler/render scan | touched-not-exhausted | `world/mod.rs`, `app_sim_tick.rs`, `build_instances.rs` static scan | implementation design |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-551A30-001 - Which vector does Main_Tick pass? -> Ground display LayerClass at 0x008A0390, not LogicClass at 0x0087F778.` (evidence: `0x0055DBC3..0x0055DBC8`; `0x0055DC99..0x0055DC9E`)
- `[RESOLVED] OQ-551A30-002 - Is it active in normal YR ticks? -> Yes when Main_Tick reaches the late side-work path before PerTick.` (evidence: `Main_Tick @ 0x0055D360`)
- `[RESOLVED] OQ-551A30-003 - Is it active in replay playback? -> Conditional yes; playback render path still falls through to this call.` (evidence: `DAT_00A8D5F8 & 2` decompile; `0x0055DBBE..0x0055DC9E`)
- `[RESOLVED] OQ-551A30-004 - Does it fully sort? -> No; one forward adjacent-swap pass only.` (evidence: `0x00551A41..0x00551A7D`)
- `[RESOLVED] OQ-551A30-005 - What is the bounds behavior for Count 0/1? -> returns before reading Items.` (evidence: `0x00551A37..0x00551A3D`)
- `[RESOLVED] OQ-551A30-006 - What is the comparison signedness? -> signed less-than on virtual int keys.` (evidence: `CMP` + `JGE` at `0x00551A63..0x00551A67`)
- `[RESOLVED] OQ-551A30-007 - Are equal keys stable? -> Yes for this pass; equal keys skip swap.` (evidence: `JGE 0x00551A76`)
- `[RESOLVED] OQ-551A30-008 - Does it compact or filter? -> No count decrement, no shift-left, no predicate except key compare.` (evidence: `0x00551A30..0x00551A84`)
- `[RESOLVED] OQ-551A30-009 - Does it read/write Object+0x98? -> No.` (evidence: negative instruction scan; `0x0055BAA0/0x0055BAE0` comparison)
- `[RESOLVED] OQ-551A30-010 - Is the loop count snapshotted? -> No, Count is reloaded at loop bottom.` (evidence: `0x00551A76`)
- `[RESOLVED] OQ-551A30-011 - Are object pointers guarded against null? -> No.` (evidence: `0x00551A4B..0x00551A5D`)
- `[RESOLVED] OQ-551A30-012 - Does it allocate or grow the vector? -> No; unlike SortedInsert, no resize path exists.` (evidence: `0x00551A30..0x00551A84`; `0x00551A90` comparison)
- `[RESOLVED] OQ-551A30-013 - What Rust active AI vector implication follows? -> Do not sort/repair live_object_order from this helper; it is display-layer state.` (evidence: binary vector address; Rust scan)
- `[DEFERRED] OQ-551A30-014 - Which derived classes override GetYSort?` (category: `out-of-scope`; reason: helper semantics only need the virtual return contract; next-step-if-pursued: vtable census for all drawable classes.)
- `[DEFERRED] OQ-551A30-015 - What concrete retail scene exposes a one-pass transient order difference?` (category: `needs-runtime-debugger`; reason: requires live scene capture or targeted fixture; next-step-if-pursued: build a two/three-object Ground-layer fixture with manually inverted order.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `FUN_00551A30` prepasses `g_DisplayLayers[2] @ 0x008A0390`, not `LogicClass @ 0x0087F778`. Active in YR: Yes. | `0x0055DBC3..0x0055DBC8`; `0x0055DC99..0x0055DC9E` | Rust `live_object_order` is a LogicClass surrogate; no display-layer vector equivalent | `src/sim/world/mod.rs`, future render display-layer state | Keep sim active-object order separate from render Ground-layer order; do not use this helper to justify sorting AI order. | Two AI objects with deliberately reversed `live_object_order` must remain in that AI order even if their render YSort keys are inverted. Proposed test: `fun_551a30_does_not_repair_logic_live_object_order` | Sorting `live_object_order` would create gameplay order drift. |
| The helper performs one stable adjacent-swap pass by signed `GetYSort`, with live count reload. Active in YR: Yes. | `0x00551A37..0x00551A7D` | Rust render fully sorts rebuilt instance lists every frame | `src/app_render/build_instances.rs`; future persistent `LayerClass` equivalent | If exact render-order parity is required, preserve persistent Ground-layer order and run a one-pass adjacent repair at native placement instead of full sort-to-fixed-point. | Start with Ground layer keys `[30, 10, 20]`; one native prepass yields `[10, 20, 30]`, but `[40, 30, 20, 10]` yields only one forward bubble pass, not arbitrary full sort. Proposed test: `ground_layer_prepass_single_adjacent_pass_not_full_sort` | A full sort can hide native transient ordering and tie-order behavior. |
| Equal YSort keys do not swap; insertion/re-submit order remains the tiebreaker. Active in YR: Yes. | `JGE` skip at `0x00551A67`; `SortedInsert @ 0x00551A90` prior docs | Rust sort stability depends on per-list sort and partitioning | render merge/sort surfaces | Preserve stable equal-key order within the native vector/list being modeled. | Three equal-key ground objects preserve prior relative order across prepass. Proposed test: `ground_layer_prepass_equal_y_sort_keeps_existing_order` | Do not add secondary stable ID/type/cell tiebreakers unless binary evidence proves one. |

## 10. Negative Facts / Do Not Do

- Do not describe `FUN_00551A30` as an active `LogicClass` object-vector sorter. Active in YR: No for that claim; evidence `ECX=0x8A0390`, while PerTick uses `ECX=0x87F778`.
- Do not sort, compact, dedupe, or filter `LogicClass`/Rust `live_object_order` because of this helper. Active in YR: No for this helper; evidence no count/field writes except adjacent `Items` swaps.
- Do not set or clear `ObjectClass+0x98` in a `FUN_00551A30` equivalent. Active in YR: No for this helper; evidence negative instruction scan and direct comparison to `0x0055BAA0/0x0055BAE0`.
- Do not implement the prepass as a full sort. Active in YR: No; evidence single loop `0x00551A41..0x00551A7D`.
- Do not add a stable-ID or class tiebreaker for equal `GetYSort`. Active in YR: No; equal keys skip swap.

## 11. Remaining Uncertainty

- Concrete derived-class `GetYSort` override census is deferred; base `ObjectClass::GetYSort` returns render-coordinate `X + Y`, but this slice only needed the virtual signed-int contract.
- A runtime visual fixture showing a multi-inversion one-pass transient was not captured. Static semantics are complete for the helper.

## 12. Stale Docs / Follow-up Docs

- `docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` lines 59-60 should replace "`side work including FUN_00551A30`" with: "`FUN_00551A30` is a one-pass adjacent repair over the Ground display `LayerClass` at `0x008A0390`, then `LogicClass::PerTickUpdate` runs later with `ECX=0x0087F778`; it is not an active-object AI vector prepass."
- `docs/research/MAIN_TICK_LATE_FRAME_INCREMENT_GATE_GLOBALS_RESWARM_20260528.md` replay clarification should replace "`pre-object order helper`" with: "`Ground display-layer one-pass YSort repair helper (`FUN_00551A30`, `ECX=0x008A0390`), not the `LogicClass` active-object vector (`ECX=0x0087F778`).`"
- `docs/research/LAYER_CLASS_GHIDRA_REPORT.md` should add after the sorted-insert discussion: "`Main_Tick` also calls `FUN_00551A30` once per reached tick on `g_DisplayLayers[2]`; this is a single adjacent-swap pass by `GetYSort`, not a full rebuild/sort, and it runs before `LogicClass::PerTickUpdate`."

## Sources

- Ghidra read-only decompile: `FUN_00551A30 @ 0x00551A30`.
- Capstone assembly from retail `gamemd.exe`: `0x00551A30..0x00551A84`, `0x0055DBC3..0x0055DC9E`, `0x00551A90..0x00551B19`, `0x005519B0..0x00551A29`, `0x005F6220..0x005F6242`.
- Ghidra caller query: `FUN_00551A30` caller is `Main_Tick @ 0x0055D360`.
- Ghidra decompile: `Main_Tick @ 0x0055D360`, `DynamicVector__SortedInsert @ 0x00551A90`, `DynamicVector__Insert @ 0x005519B0`, `ObjectClass__YSortComparator @ 0x005F6220`, `ObjectClass__GetYSort @ 0x005F6BD0`, `FUN_0055BAA0 @ 0x0055BAA0`, `FUN_0055BAE0 @ 0x0055BAE0`.
- Existing docs referenced: `LAYER_CLASS_GHIDRA_REPORT.md`, `MAIN_TICK_LATE_FRAME_INCREMENT_GATE_GLOBALS_RESWARM_20260528.md`, `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`, `ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`.
- Rust scan: `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, `src/app_render/build_instances.rs`.
