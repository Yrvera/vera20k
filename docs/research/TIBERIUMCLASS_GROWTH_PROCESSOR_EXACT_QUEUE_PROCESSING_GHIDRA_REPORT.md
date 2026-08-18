# TiberiumClass GrowthProcessor Exact Queue Processing - Ghidra Research Report

**Address(es):** `0x00722F00` primary; `0x00722C40`, `0x00483710`, `0x007235A0`, `0x00722AF0`, `0x007233A0`, `0x0055AFB0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact active standard-YR mechanics inside `TiberiumClass::GrowthProcessor @ 0x00722F00`: entry gates, batch count, heap pop order, per-entry processing, density mutation, reinsert, spread-queue feed, RNG consumption, rebuild trigger, and driver/timer interaction only as needed to prove liveness.
**Non-Scope:** save/load serialization, duplicate `AddToGrowthQueue` caller proof, full spread processor behavior, full `PlaceTiberium` placement effects, and Rust implementation.
**Confidence:** High for processor mechanics and Rust-facing deltas; Medium for the exact semantic name of the driver interval multiplier because this slot only rechecked the binary branch, not the writer/default chain for the multiplier flag.
**Active in YR:** Yes. `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0` calls `TiberiumClass::GrowthDriver_AllTypes @ 0x00722C40`; the driver calls `GrowthProcessor @ 0x00722F00` for each `TiberiumClass` when `ScenarioClass+0x34A6` is nonzero and its growth timer matures. Stock `rulesmd.ini` has `[Riparius] GrowthPercentage=.06`, `[Vinifera] GrowthPercentage=.06`, `[Aboreus] GrowthPercentage=.06`; stock `[Cruentus] GrowthPercentage=0`, so gems exit this processor by data.

## 0. Working Notes Gate

Target question: What exactly does active standard-YR `TiberiumClass::GrowthProcessor @ 0x00722F00` do to its growth heap, density byte, growth bitmap, spread queue, timers, and RNG stream?

Non-goals: Do not investigate save/load, duplicate caller proof, global spread processor, or already-settled TIBTRE source/midpoint/placement facts except where they touch the growth queue.

Evidence needed to mark COMPLETE: decompile plus assembly for `0x00722F00`; caller proof from `0x00722C40` and `0x0055AFB0`; callee proof for `CellClass::GrowTiberium`, `AddToSpreadQueue`, `RebuildGrowthQueue`, and `AddToGrowthQueue`; stock INI liveness; current Rust scan of `ore_growth.rs`/world tick/hash surfaces; no open questions in the claimed processor slice.

Stop conditions: Stop after the processor's entry gates, batch math, pop/reinsert/clear behavior, RNG consumption, and Rust handoff are proven; record save/load and duplicate caller proof as out-of-scope if encountered.

## 1. Overview

YR ore growth is queue-backed, not scan-backed. `GrowthProcessor` takes one `TiberiumClass`, chooses a random number of heap entries to pop, pops in heap priority order, calls `CellClass::GrowTiberium` only if the popped cell still maps to this same tiberium type, and either reinserts the cell into the growth queue plus feeds `AddToSpreadQueue`, or clears its growth-bitmap membership when the density is now full.

Two details matter for parity. First, the heap priority is only a sort key: the processor does not compare popped priority against `g_CurrentFrameCounter`. Second, the random batch is a pop-attempt budget. A stale/type-mismatch entry still consumes one attempt and is dropped from the heap by this function.

## 2. Class Layout / Key Offsets

| Owner | Offset | Type / shape | Processor use | Active in YR |
|---|---:|---|---|---|
| `TiberiumClass` | `+0x98` | int | Type index compared with `CellClass::GetTiberiumType`. | Yes; `0x0072300E`. |
| `TiberiumClass` | `+0xA8` | int | Growth interval source used by driver after the processor fires. | Yes; `0x00722CC6`. |
| `TiberiumClass` | `+0xB0` | double | `GrowthPercentage`; entry gate and batch fraction. | Yes; `0x00722F25`, `0x00722F40`. |
| `TiberiumClass` | `+0x10C` | int | Growth entry count in the entry array. | Yes; reinsert increments at `0x007230A4..0x007230A5`. |
| `TiberiumClass` | `+0x110` | heap object pointer | Growth min-heap; first int is heap count, second is capacity, third is pointer array. | Yes; entry/pop uses `0x00722F09..0x00722FE1`. |
| `TiberiumClass` | `+0x114` | byte array pointer | Growth membership bitmap, indexed by map-cell linear index. | Yes; set `1` at `0x00723080..0x0072308B`, clear `0` at `0x0072311E..0x0072312B`. |
| `TiberiumClass` | `+0x118` | array of 8-byte entries | Growth queue entries `{packed_coord:u32, priority:f32}`. | Yes; coord write `0x0072302E..0x0072303C`, priority write `0x00723078..0x0072307C`. |
| `TiberiumClass` | `+0x11C/+0x120/+0x124` | timer words | Driver last frame / cached word / interval. | Yes; `0x00722C76..0x00722CE3`. |
| `CellClass` | `+0x11E` | byte | Overlay data/density byte checked after `GrowTiberium`; `< 0x0B` reinserts, `>= 0x0B` clears growth bitmap. | Yes; `0x00723021..0x00723028`. |
| Global | `0x00A8ED84` | int | `g_CurrentFrameCounter`, added to reinsert priority. | Yes; `0x00723055..0x0072306C`; driver uses same global. |

## 3. Core Logic

### 3.1 Entry gates

`GrowthProcessor @ 0x00722F00` returns before consuming RNG unless all of these pass:

1. `TiberiumClass+0x110` heap pointer is non-null (`0x00722F09..0x00722F11`).
2. Heap count `*(heap+0)` is nonzero (`0x00722F17..0x00722F1F`).
3. `GrowthPercentage @ +0xB0` compares greater than `0.0` (`0x00722F25..0x00722F36`).

Active in YR: Yes. Stock `Riparius`, `Vinifera`, and `Aboreus` pass the percentage gate with `.06`; stock `Cruentus` does not pass because `[Cruentus] GrowthPercentage=0`.

### 3.2 Batch math and first RNG draw

The processor computes:

```text
batch_target = ftol(heap_count * GrowthPercentage)
batch_clamped = clamp(batch_target, 5, 50)
actual_attempts = signed_abs(Random::Next()) % batch_clamped + 1
```

Evidence:

- `FILD [heap_count]; FMUL [ESI+0xB0]; CALL Math__ftol` at `0x00722F3C..0x00722F46`.
- Clamp to minimum `5` and maximum `0x32` at `0x00722F4B..0x00722F68`.
- `Random__Next` from global RNG object at `0x00722F6A..0x00722F75`.
- Signed absolute before modulo at `0x00722F7A..0x00722F86`; `IDIV EDI`; `INC EDI` at `0x00722F86..0x00722F8D`.

`Math__ftol @ 0x007C5F00` uses `FISTP qword ptr [EAX]` after loading the engine control word when needed. The report does not rename that rounding mode beyond saying it is the binary's `ftol` conversion, not Rust `floor` by assumption.

Active in YR: Yes when the driver fires a positive-percentage type.

### 3.3 Capacity rebuild trigger

After choosing `actual_attempts`, the processor calls the map-cell-count helper `FUN_0042B1F0` and rebuilds if:

```text
initial_heap_count > map_cell_count - (actual_attempts * 2)
```

Evidence: `CALL 0x0042B1F0`, `LEA EDX,[EDI+EDI]`, `SUB EAX,EDX`, `CMP EBX,EAX`, `JLE no_rebuild`, `CALL 0x007233A0` at `0x00722F91..0x00722FA1`.

This is a pressure rebuild, not a per-entry condition. It happens before the first pop.

Active in YR: Yes, though normally only near map-cell saturation.

### 3.4 Heap pop and priority use

The processor pops the heap root before entering the main loop, then pops another root after each attempt until it reaches `actual_attempts` or the heap becomes empty.

The popped entry's priority is never compared against `g_CurrentFrameCounter`. Priority is used only by the heap ordering code (`0x007230C9..0x0072310D` for insert-up; `0x00723157..0x00723249` for pop-down). Therefore entries with priority greater than the current frame can still be popped if they are the heap minimum and the driver has fired.

Active in YR: Yes. This is a direct consequence of the assembly path from `0x00722FB4` to `0x00723249`.

### 3.5 Per-entry processing

For each popped entry pointer:

1. Null pointer returns immediately (`0x00722FF2..0x00722FF4`).
2. Packed coord is converted to a `CellClass*` through `MapClass::Get_CellClass @ 0x005657A0` (`0x00722FFA..0x00723005`).
3. `CellClass::GetTiberiumType @ 0x00485010` is called (`0x00723007..0x0072300E`).
4. Only if that type equals `this->ArrayIndex @ +0x98` does the processor call `CellClass::GrowTiberium @ 0x00483710` (`0x0072300E..0x0072301C`).

If the type does not match, the processor skips growth, does not reinsert the entry, does not clear the growth bitmap, increments the attempt counter, and pops the next entry if any (`0x0072312F..0x00723140`). That is a real stale-entry behavior, not a successful-growth loop.

Active in YR: Yes. Stale/type-mismatch entries are conditional on queue state diverging from cell contents, but the branch is live code in the standard processor.

### 3.6 Density mutation and reinsert/clear split

`CellClass::GrowTiberium @ 0x00483710` is called before the density test. That helper rechecks `ScenarioClass+0x34A6`, current overlay-to-tiberium mapping, flat slope, `OverlayData < MaxDensity - 1`, and `GrowthPercentage >= 0`, then calls `CellClass::PlaceTiberium(type, 1)`.

After the helper returns, `GrowthProcessor` reads the cell's `OverlayData` byte:

- If `OverlayData < 0x0B`, it appends a new growth entry for the same coord, computes a new priority, sets the growth bitmap byte to `1`, inserts the entry pointer into the min-heap, and calls `TiberiumClass::AddToSpreadQueue` for the same cell.
- If `OverlayData >= 0x0B`, it clears this tiberium type's growth bitmap byte for the cell to `0` and does not reinsert.

Evidence: call to `GrowTiberium` at `0x0072301C`; density compare `CMP byte ptr [EBP+0x11E],0x0B` and `JNC full` at `0x00723021..0x00723028`; reinsert path `0x0072302E..0x00723113`; full path clear `0x0072311E..0x0072312B`.

Active in YR: Yes. For stock Riparius, max density is `12`, so visible full density is data byte `11`.

### 3.7 Reinsert priority and spread feed RNG

Growth reinsert priority is:

```text
priority_f32 = float(g_CurrentFrameCounter + abs(signed(Random::Next() % 50)))
```

The assembly takes `Random__Next`, performs signed `IDIV 0x32`, takes the signed absolute value of the remainder, adds `g_CurrentFrameCounter`, then stores through `FILD/FSTP` into the entry's float priority field (`0x0072303F..0x0072307C`). This is equivalent to `signed_abs(raw) % 50` for normal signed integer inputs, but the binary order is remainder-first in this processor.

After inserting the still-growable cell into the growth heap, the processor calls `TiberiumClass::AddToSpreadQueue @ 0x00722AF0` with the same popped entry coord pointer (`0x00723110..0x00723113`). `AddToSpreadQueue` itself consumes an additional `Random::Next()` only if `CanSpreadTiberium` passes and this tiberium type's spread bitmap is currently `0`.

Active in YR: Yes.

### 3.8 RNG consumption summary

For a processor call that passes entry gates:

- Always consumes one `Random::Next()` for `actual_attempts`.
- Consumes one additional `Random::Next()` per popped matching-type cell whose post-growth `OverlayData < 11`, for the growth reinsert priority.
- May consume one additional `Random::Next()` inside `AddToSpreadQueue` for each still-growable cell, but only if `CanSpreadTiberium` passes and the spread bitmap did not already contain that cell.
- Consumes no processor RNG for full-density cells after growth.
- Consumes no reinsert/spread RNG for type-mismatch stale entries.

Active in YR: Yes.

## 4. INI Keys

| Key | Source | Stock YR value | Binary effect | Active in YR |
|---|---|---:|---|---|
| `[Tiberiums]` | `rulesmd.ini` | `0=Riparius`, `1=Cruentus`, `2=Vinifera`, `3=Aboreus` | Builds `g_TiberiumClass_Array`; class `+0x98` is compared to cell type. | Yes. |
| `[Riparius] Growth` | `rulesmd.ini` | `2200` | Driver interval source at `+0xA8`, scaled when driver reloads `+0x124`. | Yes. |
| `[Riparius] GrowthPercentage` | `rulesmd.ini` | `.06` | Positive entry gate and batch fraction. | Yes. |
| `[Cruentus] GrowthPercentage` | `rulesmd.ini` | `0` | Processor returns before batch/RNG for stock gems. | Yes as a zero-data gate; no stock gem growth. |
| `[Vinifera] GrowthPercentage` | `rulesmd.ini` | `.06` | Positive entry gate if such cells exist. | Conditional on map/rules content. |
| `[Aboreus] GrowthPercentage` | `rulesmd.ini` | `.06` | Positive entry gate if such cells exist. | Conditional on map/rules content. |
| `[General] TiberiumGrows` / map flags | `rulesmd.ini`, map INI | `yes` in stock rules | Feeds scenario/global gate represented by `ScenarioClass+0x34A6`. | Yes by default for standard skirmish unless map disables. |

## 5. Integration Points

| Point | Evidence | Active in YR |
|---|---|---|
| Logic tick calls growth before spread. | `0x0055AFB0` decompile: `TiberiumClass__GrowthDriver_AllTypes(); TiberiumClass__SpreadDriver_AllTypes();` | Yes. |
| Growth driver iterates every `g_TiberiumClass_Array` entry. | `0x00722C57..0x00722CEE`, count at `0x00B0F4F8`, array at `0x00B0F4EC`. | Yes. |
| Growth driver calls processor only when `ScenarioClass+0x34A6` and timer permit. | `0x00722C48..0x00722C99`. | Conditional; stock growth-enabled maps pass. |
| Growth driver reloads timer after processor. | `0x00722C9E..0x00722CE3`, writes `+0x11C/+0x120/+0x124`. | Yes. |
| Rebuild seeds only currently growable cells. | `0x007233A0` calls `GetTiberiumType` and `CanGrowTiberium @ 0x00483620`, then inserts priority `0.0`. | Yes. |
| Runtime placement can add growth queue entries. | `AddToGrowthQueue @ 0x007235A0`; xrefs from `Reduce_Tiberium`, `PlaceTiberium`, `VoxelAnimClass::AI`. | Yes. |

## 6. Current Rust Implementation Status

Current Rust does not implement this processor. `src/sim/ore_growth.rs` still owns a scan/reservoir model over `ResourceNode`s: it scans map chunks, reservoir-samples growth/spread candidates, mutates ore by `+120`, and spreads new level-1 ore. Recent TIBTRE work added native-shaped `OreGrowthQueueEntry` and a growth queue append path, but no native queue processor consumes those entries.

Rust deltas:

- `tick_ore_growth` does not use per-tiberium growth heaps, per-type percentages, or per-type timers.
- `OreGrowthState::growth_queue` is a vector, not the native min-heap plus bitmap membership model.
- No processor path implements `batch = clamp(ftol(heap_count * GrowthPercentage), 5, 50); actual = random % batch + 1`.
- No path pops priority-ordered growth entries, calls `GrowTiberium`, reinserts still-growable cells, and feeds `AddToSpreadQueue`.
- `world_hash.rs` now hashes current `ore_growth_state`, but the state being hashed is still not the native processor state.
- `world/mod.rs` ticks Rust ore growth before terrain spawners; GameMD logic tick also runs growth before spread, but terrain object AI belongs to object AI timing outside this processor slice.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes gate | verified | Section 0 | none |
| GrowthProcessor entry gates | verified | `0x00722F09..0x00722F36` | none |
| Batch math and clamp | verified | `0x00722F3C..0x00722F8D`, `0x007C5F00` | exact FPU control-word source not needed for this slice |
| Capacity rebuild trigger | verified | `0x00722F91..0x00722FA1` | none |
| Heap pop / priority ordering | verified | `0x00722FB4..0x00723249` | heap helper names not assigned |
| No current-frame priority gate | verified | full `0x00722F00` decompile/disassembly, no compare of popped priority to `0x00A8ED84` | none |
| Type-match gate and stale mismatch behavior | verified | `0x00722FFA..0x00723140` | runtime frequency of stale mismatch out-of-scope |
| `GrowTiberium` call and internal guards | verified | `0x0072301C`, `0x00483710` | none for processor handoff |
| Reinsert/clear split on `OverlayData < 11` | verified | `0x00723021..0x0072312B` | none |
| Spread-feed call | verified | `0x00723110..0x00723113`, `0x00722AF0` | full spread processor out-of-scope |
| Growth driver / timer integration | verified | `0x00722C40`, xref from `0x0055B4D7` | exact scenario flag writer deferred |
| Stock INI liveness | verified | `ini/rulesmd.ini:30372-30431` | none |
| Current Rust scan | verified-source-scan | `src/sim/ore_growth.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs` | implementation not performed |
| Save/load queue behavior | deferred | user non-goal | slot 3 of parent re-swarm |
| Duplicate `AddToGrowthQueue` caller proof | deferred | user non-goal | slot 2 of parent re-swarm |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this an exhaustive slice or coverage map? -> Exhaustive slice for `GrowthProcessor @ 0x00722F00` only.` (evidence: user slot scope; report header)
- `[RESOLVED] OQ-02 - Is the function active in standard YR? -> Yes through logic tick -> growth driver -> processor when growth gate/timer permits.` (evidence: `0x0055AFB0`, `0x00722C40`, `rulesmd.ini [Riparius]`)
- `[RESOLVED] OQ-03 - What are the entry gates? -> heap pointer, nonzero heap count, and `GrowthPercentage > 0.0`.` (evidence: `0x00722F09..0x00722F36`)
- `[RESOLVED] OQ-04 - How is batch target computed? -> `ftol(heap_count * GrowthPercentage)`, clamped to `[5,50]`.` (evidence: `0x00722F3C..0x00722F68`)
- `[RESOLVED] OQ-05 - How is actual pop count chosen? -> one RNG draw, signed absolute, modulo clamped batch, plus one.` (evidence: `0x00722F6A..0x00722F8D`)
- `[RESOLVED] OQ-06 - Is batch count successful growths or pop attempts? -> Pop attempts; mismatch entries increment the attempt counter.` (evidence: `0x0072312F..0x0072313A`)
- `[RESOLVED] OQ-07 - Does priority gate by current frame? -> No; priority only orders heap insert/pop.` (evidence: `0x00722F00` disassembly, absence of priority/current-frame compare after pop)
- `[RESOLVED] OQ-08 - What happens if popped cell type mismatches? -> No grow, no reinsert, no growth-bitmap clear, attempt consumed.` (evidence: `0x0072300E..0x00723140`)
- `[RESOLVED] OQ-09 - When is `GrowTiberium` called? -> Only after current cell type equals `this->ArrayIndex`.` (evidence: `0x00723007..0x0072301C`)
- `[RESOLVED] OQ-10 - Does processor check density before or after growth? -> After calling `GrowTiberium`.` (evidence: `0x0072301C..0x00723028`)
- `[RESOLVED] OQ-11 - What is the reinsertion condition? -> Post-growth `OverlayData < 11`.` (evidence: `0x00723021..0x00723028`)
- `[RESOLVED] OQ-12 - What is the full condition? -> Post-growth `OverlayData >= 11` clears growth bitmap and does not reinsert.` (evidence: `0x0072311E..0x0072312B`)
- `[RESOLVED] OQ-13 - Does growth feed spread? -> Yes, after still-growable reinsert, via `AddToSpreadQueue` for the same coord.` (evidence: `0x00723110..0x00723113`)
- `[RESOLVED] OQ-14 - How many RNG draws occur? -> One batch draw plus one growth-priority draw per still-growable reinsert, plus conditional spread-queue RNG inside `AddToSpreadQueue`.` (evidence: `0x00722F6A`, `0x0072303F`, `0x00722AF0`)
- `[RESOLVED] OQ-15 - Does stock Cruentus grow? -> No; class exists but `GrowthPercentage=0` exits before RNG.` (evidence: `rulesmd.ini [Cruentus]`, `0x00722F25..0x00722F36`)
- `[RESOLVED] OQ-16 - What current Rust surface is affected? -> `src/sim/ore_growth.rs`, `ProductionState`, and `world_hash.rs`.` (evidence: source scan)
- `[DEFERRED] OQ-17 - Exact native save/load behavior for queue fields.` (category: out-of-scope; reason: parent assigned to another slot; next-step-if-pursued: trace save/load or rebuild-on-load paths)
- `[DEFERRED] OQ-18 - Can `AddToGrowthQueue` duplicates occur across all callers?` (category: out-of-scope; reason: parent assigned to duplicate-semantics slot; next-step-if-pursued: xref each caller with bitmap state)
- `[DEFERRED] OQ-19 - Exact writer/default chain for `ScenarioClass+0x34A6`.` (category: requires-different-system-context; reason: this slice only needs driver gate and stock INI liveness; next-step-if-pursued: trace General/Basic/SpecialFlags reader writes)

Adversarial checks answered from evidence:

- Empty heap: returns before RNG.
- Positive percentage but tiny heap: batch clamps to at least 5, but loop returns when heap empties.
- Popped stale cell: consumes an attempt and is not reinserted by this function.
- Full-density result: clears growth bitmap and does not call `AddToSpreadQueue`.
- Still-growable result: consumes reinsert RNG, sets growth bitmap, reinserts into heap, and then may enqueue spread.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Processor batch is `clamp(ftol(heap_count * GrowthPercentage), 5, 50)`, then `signed_abs(Random::Next()) % batch + 1`. | `0x00722F3C..0x00722F8D`, `0x007C5F00` | Missing; Rust scans/reservoir-samples candidates. | `src/sim/ore_growth.rs`, `SimRng` | Replace scan-cycle candidate selection with per-type heap pop-attempt budget and matching RNG consumption. | Seed 100 Riparius growth heap entries with `.06`; first processor call chooses `1..6` attempts from one RNG draw and never scans the map. Proposed test: `growth_processor_uses_clamped_percentage_pop_attempt_budget` | Do not use `[General] GrowthRate` scan cadence or reservoir sampling as a proxy. |
| Heap priority orders entries only; processor does not check priority against `g_CurrentFrameCounter`. | `0x00722F00` disassembly; no current-frame compare after pop | Missing/contradicted if future code treats priority as wake time. | `src/sim/ore_growth.rs` future queue processor | Pop by min-priority whenever the driver fires, regardless of whether priority is greater than current frame. | Entry with priority `current_frame+49` still pops when it is heap-minimum on a matured driver tick. Proposed test: `growth_processor_priority_orders_but_does_not_delay_pop` | Do not implement priority as a timer/wake-up gate. |
| A stale/type-mismatch popped entry consumes one attempt and is dropped without growth, reinsert, or growth-bitmap clear. | `0x0072300E..0x00723140` | Missing; no native heap/stale behavior exists. | `src/sim/ore_growth.rs`, queue bitmap state | Preserve pop-attempt semantics and stale-entry side effects when visible resource state changed after queue insertion. | Heap has one Riparius queue entry whose cell now maps to Cruentus; processor consumes one attempt, leaves cell unchanged, does not call spread feed. Proposed test: `growth_processor_type_mismatch_consumes_attempt_without_reinsert` | Do not filter stale entries before counting attempts unless a rebuild happened first. |
| Matching-type cell calls `GrowTiberium`, then post-growth `OverlayData < 11` reinserts with `currentFrame + abs(Random % 50)`, sets growth bitmap, and calls `AddToSpreadQueue`; `>=11` clears growth bitmap and stops. | `0x0072301C..0x0072312B`, `0x00483710`, `0x00722AF0` | Missing; Rust mutates stock directly and has no processor spread-feed. | `src/sim/ore_growth.rs`, overlay/resource mapping, future `CellClass::GrowTiberium` equivalent | Model density byte mutation through the same PlaceTiberium/grow rules, then branch on exact post-growth byte and update queue/bitmap/spread feed. | Density 9 grows to 10 and reinserts/feeds spread; density 10 grows to 11 and clears growth membership. Proposed test: `growth_processor_reinsert_until_overlay_data_ten_then_clear_at_eleven` | Do not branch on Rust stock amount before growth; GameMD branches on post-growth overlay data byte. |
| Driver runs growth before spread and reloads per-type growth timer after the processor fires. | `0x0055AFB0`, `0x00722C40` | Rust has one `tick_ore_growth` and terrain spawners after it; native per-type timers absent. | `src/sim/world/mod.rs`, `src/sim/ore_growth.rs`, `ProductionState` | Keep growth-driver-before-spread-driver ordering, with per-type last-frame/interval state. | On a tick where both queues mature, all type growth processors run before any spread processor. Proposed test: `ore_growth_driver_runs_all_growth_before_any_spread` | Do not combine growth and spread in one per-cell scan loop. |
| Growth queue state affects future deterministic output. | processor mechanics plus current `world_hash.rs` source scan | Partially represented but wrong model. | `src/sim/world/world_hash.rs`, snapshots | Hash and snapshot native queue timers, heap order, entries, and bitmaps once implemented. | Two sims with identical visible ore but different growth heap order produce different hashes. Proposed test: `growth_heap_order_changes_world_hash` | Do not hash only visible `resource_nodes`. |

## 10. Negative Facts / Do Not Do

- Do not model growth priority as a wake-up timestamp. Active in YR: No such gate exists in `0x00722F00`; priority only orders heap pop/insert.
- Do not count only successful grows toward the random batch. Active in YR: pop attempts count; type mismatches increment the attempt counter at `0x0072312F..0x0072313A`.
- Do not prefilter all stale queue entries before consuming attempts. Active in YR: the function observes a popped stale/type-mismatch entry after the heap pop.
- Do not use `[General] GrowthRate` as the YR queue processor cadence. Active in YR: growth driver uses per-`TiberiumClass` `Growth @ +0xA8` and timer fields `+0x11C/+0x124`.
- Do not hardcode stock gems as absent from the TiberiumClass array. Active in YR: `Cruentus` exists, but `GrowthPercentage=0` makes the processor return before RNG.
- Do not branch on Rust resource stock level before calling the grow equivalent. Active in YR: `GrowTiberium` is called first, and then `OverlayData` byte is read for the reinsert/full split.
- Do not assume `AddToGrowthQueue` duplicate behavior from this processor. Active in YR: duplicate proof belongs to caller-specific paths; this report only proves processor pop/reinsert behavior.

## 11. Stale Docs / Follow-up Docs

- `docs/research/TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`: replace "Pops heap entries and processes until actual successful spreads reach the chosen count or heap empties" for the growth processor with: "Pops growth heap entries until the random pop-attempt budget is reached or the heap empties. A popped entry whose current cell no longer maps to this `TiberiumClass+0x98` still consumes one attempt and is dropped by this function without growth, reinsert, or growth-bitmap clear."
- `docs/research/traces/RIPARIUS_GROWTH_SPREAD_QUEUE_STANDARD_YR_TRACE.md`: replace "growth pops a Riparius growth queue entry, calls `CellClass::GrowTiberium`, then if the resulting density is still `< 11`..." with: "growth pops up to a random attempt budget from the per-type heap; each popped entry first revalidates current cell type. Only matching entries call `CellClass::GrowTiberium`; stale/type-mismatch entries consume an attempt and are not reinserted by this function."

## 12. Remaining Uncertainty

- Exact native save/load behavior for growth queue entries, heap, bitmap, and timers is intentionally out-of-scope for this slot.
- Global duplicate-entry proof for `AddToGrowthQueue` is intentionally out-of-scope for this slot.
- The exact writer/default chain for `ScenarioClass+0x34A6` was not re-traced here; stock INI liveness and driver gate were sufficient for this processor slice.
- The semantic name of the driver multiplier flag selected at `0x00722CA4` remains inherited from prior timing research; this report only confirms the branch exists and affects interval reload.

## Sources

- Ghidra decompile/disassembly: `TiberiumClass::GrowthProcessor @ 0x00722F00`.
- Ghidra decompile/disassembly: `TiberiumClass::GrowthDriver_AllTypes @ 0x00722C40`.
- Ghidra decompile/disassembly: `CellClass::GrowTiberium @ 0x00483710`.
- Ghidra decompile/disassembly: `CellClass::CanGrowTiberium @ 0x00483620`.
- Ghidra decompile/disassembly: `TiberiumClass::AddToGrowthQueue @ 0x007235A0`.
- Ghidra decompile: `TiberiumClass::AddToSpreadQueue @ 0x00722AF0`.
- Ghidra decompile/disassembly: `TiberiumClass::RebuildGrowthQueue @ 0x007233A0`.
- Ghidra decompile: `TiberiumClass::InitGrowthQueues_All @ 0x00722D00`.
- Ghidra decompile: `LogicClassPerTickUpdateLiveVector @ 0x0055AFB0`.
- Xrefs: `0x00722F00` called from `0x00722C99`; `0x00722C40` called from `0x0055B4D7`; `0x00722AF0` called from `0x00723113`; `0x00483710` called from `0x0072301C`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Prior docs referenced: `TIBERIUMCLASS_GROWTH_SPREAD_QUEUE_STATE_AND_SERIALIZATION_GHIDRA_REPORT.md`, `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md`, `TIBERIUM_VOXEL_VEIN_DIRTY_PRODUCER_STOCK_LIVENESS_GHIDRA_REPORT.md`, `traces/RIPARIUS_GROWTH_SPREAD_QUEUE_STANDARD_YR_TRACE.md`.
- Rust scanned: `src/sim/ore_growth.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs`, `src/sim/production/production_types.rs`, `src/app_init.rs`.
