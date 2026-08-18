# TechnoClass AI Update Unload Accumulator Ordering - Ghidra Research Report

**Address(es):** `0x006F9E50` `TechnoClass::AI_Update`, `0x004DA530` `FootClass::AI`, `0x007360C0` `UnitClass::AI`, `0x005B3060` `MissionClass::Mission_Dispatch`, `0x0073D630` `UnitClass::Mission_Deploy_Building`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** stock `HARV`/`CMIN` refinery unload ordering between `TechnoClass::AI_Update` timer-cluster increment of `Unit+0xF8` and `UnitClass::Mission_Deploy_Building` state-3 `HarvesterDumpRate` gate.
**Non-Scope:** RateTimer `Set/Current`, `PathType::Has_Valid_Steps` polarity, cargo credit formulas, exact `+0x104` meaning, and runtime replay capture.
**Confidence:** High
**Active in YR:** Yes. Stock `[HARV]` and `[CMIN]` have `Harvester=yes`; stock `[GAREFN]`/`[NAREFN]` have `DockUnload=yes`/`Refinery=yes`; mission `0x10` dispatches to `UnitClass::Mission_Deploy_Building`.

## 1. Overview

`TechnoClass::AI_Update` calls `MissionClass::Mission_Dispatch` before it runs the generic `+0xF8/+0x100/+0x108/+0x10C/+0x110` timer-cluster accumulator block. Therefore `Mission_Deploy_Building` state 3 samples the dump accumulator before the current tick's Techno accumulator update.

On the first accepted unload-start dispatch, `Mission_Deploy_Building` initializes the timer cluster with `+0xF8=0`, `+0x100=g_CurrentFrameCounter`, `+0x108=1`, and `+0x10C=1`. The accumulator block later in that same `TechnoClass::AI_Update` pass does not increment `+0xF8`, because elapsed frames are still `0` and the one-frame duration has not expired.

## 2. Key Offsets

| Owner | Offset | Meaning in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| MissionClass/TechnoClass | `+0xC8` (`param[0x32]`) | mission timer start frame | `Mission_Dispatch @ 0x005B3060` | Yes |
| MissionClass/TechnoClass | `+0xD0` (`param[0x34]`) | mission delay/duration returned by handler | `0x005B3060` stores handler return here | Yes |
| UnitClass/TechnoClass | `+0xF8` | unload dump accumulator; compared in deploy state 3 | increment block `0x006FABC4..0x006FAC2A`; gate `0x0073E35B..0x0073E374` | Yes |
| UnitClass/TechnoClass | `+0xFC` | timer active-this-frame flag written by accumulator block | `0x006FABFF` set true, `0x006FAC2A` set false | Yes |
| UnitClass/TechnoClass | `+0x100` | timer-cluster start frame | unload-start write `0x0073DFF3`; accumulator read `0x006FABC4` | Yes |
| UnitClass/TechnoClass | `+0x104` | timer-cluster secondary/Z value copied from stack | unload-start write `0x0073DFF5..0x0073DFF9`; accumulator write `0x006FAC1C` | Yes |
| UnitClass/TechnoClass | `+0x108` | timer-cluster duration | unload-start write `0x0073DFFC = 1`; accumulator read `0x006FABCA` | Yes |
| UnitClass/TechnoClass | `+0x10C` | timer-cluster active/repeat value | unload-start write `0x0073DFED = 1`; accumulator gate `0x006FABE7..0x006FABEF` | Yes |
| UnitClass/TechnoClass | `+0x110` | accumulator increment step | accumulator adds it into `+0xF8` at `0x006FABF1..0x006FAC06` | Yes; exact initializer is slot-1 scope |

## 3. Core Logic

### 3.1 Live unit AI order

For stock units, the static call path is:

1. `UnitClass::AI @ 0x007360C0` calls `FootClass::AI @ 0x004DA530` at `0x0073647B`.
2. `FootClass::AI` starts with `TechnoClass::AI_Update @ 0x006F9E50` at `0x004DA539`.
3. `TechnoClass::AI_Update` increments `+0xC4`, then calls `MissionClass::Mission_Dispatch @ 0x005B3060` at `0x006FA655`.
4. The `+0xF8` accumulator block runs later in the same `TechnoClass::AI_Update`, beginning with the `+0x100/+0x108` timer check at `0x006FABC4`.
5. `FootClass::AI` locomotor process runs still later at `0x004DA87A`.

Active in YR: Yes. `get_function_xrefs` shows `FootClass::AI -> TechnoClass::AI_Update` at `0x004DA539`, `TechnoClass::AI_Update -> Mission_Dispatch` at `0x006FA655`, and `UnitClass::AI -> FootClass::AI` at `0x0073647B`.

### 3.2 Mission dispatch storage order

`MissionClass::Mission_Dispatch @ 0x005B3060` first runs `ObjectClass::AI`, then checks mission timer fields. If the mission is not due, it returns before any handler call. If due, mission `0x10` calls vtable slot `+0x23C`, then stores:

- `+0xC8 = g_CurrentFrameCounter`
- `+0xCC = stack value` (decompiler's `iStack_8`)
- `+0xD0 = handler return delay`

Active in YR: Yes. Mission `0x10` is the stock queued unload mission from refinery radio `0x15`; existing radio docs and `Mission_Dispatch @ 0x005B3060` confirm the live dispatch slot.

### 3.3 Unload-start initialization runs before the accumulator block

When mission `0x10` reaches the accepted path/facing branch, `UnitClass::Mission_Deploy_Building` writes:

1. `0x0073DFD0`: `Unit+0xF8 = 0`
2. `0x0073DFDA`: `Unit+0x6D1 = 1`
3. `0x0073DFE0`: reads `g_CurrentFrameCounter`
4. `0x0073DFED`: `Unit+0x10C = 1`
5. `0x0073DFF3`: `Unit+0x100 = current frame`
6. `0x0073DFF5..0x0073DFF9`: `Unit+0x104 = stack value`
7. `0x0073DFFC`: `Unit+0x108 = 1`
8. `0x0073E093`: `Unit+0xBC = 3`
9. `0x0073E09D -> 0x0073E289`: returns through the mission timer epilogue

Because this is inside `Mission_Dispatch`, all of these writes happen before the later `TechnoClass::AI_Update` accumulator block in that same unit AI pass.

Active in YR: Yes. This branch is reached by stock `HARV/CMIN` after `PathType` and facing gates pass.

### 3.4 Same-tick first increment is impossible

The accumulator block at `0x006FABC4..0x006FAC2A` performs this ordering:

1. Read `start = Unit+0x100` and `duration = Unit+0x108`.
2. If `start != -1`, compute `elapsed = g_CurrentFrameCounter - start`.
3. If `elapsed < duration`, compute remaining `duration - elapsed`.
4. If remaining is nonzero, skip increment and write `Unit+0xFC = 0`.
5. Only if remaining is zero and `Unit+0x10C != 0`, write:
   - `Unit+0xFC = 1`
   - `Unit+0xF8 += Unit+0x110`
   - `Unit+0x100 = g_CurrentFrameCounter`
   - `Unit+0x104 = stack value`
   - `Unit+0x108 = Unit+0x10C`

On the first accepted unload-start pass, mission code has just written `+0x100 = g_CurrentFrameCounter` and `+0x108 = 1`. The later accumulator block sees `elapsed = 0`, `duration = 1`, remaining `1`, and therefore skips the increment. The first possible `+0xF8 += +0x110` occurs on a later `TechnoClass::AI_Update` pass where elapsed is at least one frame.

Active in YR: Yes. This is the live non-building `TechnoClass::AI_Update` accumulator path; the block explicitly skips buildings by jumping to `0x006FAC31` when `WhatAmI()==6`.

### 3.5 State-3 dump gate sees previous accumulator state

State 3 in `UnitClass::Mission_Deploy_Building` compares:

`(double)Unit+0xF8 >= RulesClass+0x1528 HarvesterDumpRate * 900.0`

Evidence range: `0x0073E355..0x0073E374`.

Since `Mission_Dispatch` runs before the accumulator block every `TechnoClass::AI_Update` pass, this gate can see only the accumulator value produced by earlier passes. It cannot see the increment that may occur later in the same `TechnoClass::AI_Update` pass.

Active in YR: Yes. This is the stock state-3 refinery unload gate for `HARV/CMIN`.

## 4. INI Keys

| Key | Stock value / source | Effect | Active in YR |
|---|---|---|---|
| `[General] HarvesterDumpRate` | default `0.016` | state-3 threshold `0.016 * 900 = 14.4` | Yes |
| `[CMIN] Harvester` | `yes`, `rulesmd.ini` | reaches harvester branch | Yes |
| `[HARV] Harvester` | `yes`, `rulesmd.ini` | reaches harvester branch | Yes |
| `[GAREFN]/[NAREFN] DockUnload` | `yes`, `rulesmd.ini` | radio `0x15` queues mission `0x10` | Yes |
| `[GAREFN]/[NAREFN] Refinery` | `yes`, `rulesmd.ini` | state-3/state-4 refinery unload path | Yes |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::AI @ 0x007360C0` | stock unit AI owner | calls `FootClass::AI` at `0x0073647B` | Yes |
| `FootClass::AI @ 0x004DA530` | mobile-unit tick shell | starts with `TechnoClass::AI_Update`; locomotor process later at `0x004DA87A` | Yes |
| `TechnoClass::AI_Update @ 0x006F9E50` | mission dispatch and timer-cluster owner | mission call `0x006FA655`; accumulator `0x006FABC4..0x006FAC2A` | Yes |
| `MissionClass::Mission_Dispatch @ 0x005B3060` | mission timer gate and return-delay storage | mission `0x10` slot `+0x23C`; stores return delay in `+0xD0` | Yes |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | unload-start and state-3 dump gate | init `0x0073DFD0..0x0073E093`; gate `0x0073E355..0x0073E374` | Yes |

## 6. Current Rust Implementation Status

Current Rust does not model the gamemd timer cluster directly:

- `src/sim/miner/miner_dock_sequence.rs:805..830` starts unload with Rust-side pad bookkeeping, display override, forced facing, a sound event, and `unload_timer = interval - 10`.
- `src/sim/miner/miner_dock_sequence.rs:854..875` drains using a local `unload_timer` countdown rather than `TechnoClass::AI_Update` incrementing a shared `+0xF8` accumulator after mission dispatch.
- `src/sim/miner/mod.rs:263..267` documents `unload_timer` as the Rust countdown.
- `src/sim/world/mod.rs:1113..1114` updates `binary_frame` at the beginning of Rust tick execution, whereas existing gamemd timing docs place `g_CurrentFrameCounter` increment near the end of the main tick.

Current Rust delta: mechanism drift for Plan C. Rust can be player-close, but full coupled byte-field parity needs a mission-dispatch-before-accumulator order and a one-frame no-increment edge on unload-start.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::AI -> FootClass::AI` | verified | decompile `0x007360C0`; xref `0x0073647B` | none |
| `FootClass::AI -> TechnoClass::AI_Update` | verified | decompile `0x004DA530`; xref `0x004DA539` | none |
| `TechnoClass::AI_Update -> Mission_Dispatch` placement | verified | decompile/disassembly `0x006FA646..0x006FA655` | none |
| `TechnoClass::AI_Update` accumulator placement | verified | disassembly `0x006FABC4..0x006FAC2A`, after mission dispatch | none |
| unload-start timer-cluster init | verified | disassembly `0x0073DFD0..0x0073E093` | exact `+0x104` meaning belongs to slot 2 |
| same-tick first increment edge | verified | `+0x100=current`, `+0x108=1`; accumulator remaining branch `0x006FABD0..0x006FAC2A` | none |
| state-3 gate reads pre-current-pass accumulator | verified | mission dispatch before accumulator plus gate `0x0073E355..0x0073E374` | none |
| exact `+0x110` initializer/default | deferred | slot 1 target | separate writer/default investigation |
| runtime first deposit frame | deferred | static order only | runtime trace if exact replay frame needed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does stock UnitClass AI reach TechnoClass::AI_Update? -> Yes, `UnitClass::AI` calls `FootClass::AI`, which calls `TechnoClass::AI_Update`.` (evidence: `0x0073647B`, `0x004DA539`)
- `[RESOLVED] OQ-02 - Does Mission_Dispatch run before the unload accumulator block? -> Yes. `Mission_Dispatch` call is at `0x006FA655`; accumulator timer check starts at `0x006FABC4`.` (evidence: `0x006F9E50` disassembly)
- `[RESOLVED] OQ-03 - Can accepted unload-start initialize the timer cluster before that same pass's accumulator block? -> Yes; unload-start writes are inside mission `0x10` dispatch before returning to `TechnoClass::AI_Update`.` (evidence: `0x005B3060`, `0x0073DFD0..0x0073E093`, `0x006FA655`)
- `[RESOLVED] OQ-04 - Does the same pass increment `+0xF8` immediately after unload-start? -> No. `+0x100=current frame` and `+0x108=1` make remaining time nonzero, so the block skips increment and clears `+0xFC`.` (evidence: `0x0073DFF3`, `0x0073DFFC`, `0x006FABD0..0x006FAC2A`)
- `[RESOLVED] OQ-05 - Can state-3 dump gate see an accumulator increment from later in the same AI pass? -> No. The gate is inside mission dispatch, and the increment block is later.` (evidence: `0x006FA655`, `0x006FABC4`, `0x0073E355..0x0073E374`)
- `[RESOLVED] OQ-06 - Does accumulator continue while mission dispatch is not due? -> Yes; `Mission_Dispatch` may return early, but `TechnoClass::AI_Update` continues to the later accumulator block.` (evidence: `0x005B3060` early return; caller continues at `0x006FA65A` and later `0x006FABC4`)
- `[DEFERRED] OQ-07 - Where is `+0x110` initialized to the stock unload increment step?` (category: `requires-different-system-context`; reason: assigned to swarm slot 1; next-step-if-pursued: audit constructors/save-load/harvest/unload-start writers for `Unit+0x110`)
- `[DEFERRED] OQ-08 - What exact runtime frame is the first stock deposit after accepted unload-start?` (category: `needs-runtime-debugger`; reason: this report proves static order, not mission-table delay plus frame-counter replay timing; next-step-if-pursued: runtime trace `+0xF8/+0x100/+0x108/+0x10C/+0x110` and mission timer fields)
- `[DEFERRED] OQ-09 - What is the exact source/meaning of the stack value copied to `+0x104`?` (category: `requires-different-system-context`; reason: assigned to swarm slot 2; next-step-if-pursued: trace dataflow around `0x0073DFF5` and `0x006FAC1C`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Mission `0x10` state-3 gate runs before the current pass's accumulator increment | `0x006FA655` before `0x006FABC4`; gate `0x0073E355..0x0073E374` | Rust `phase_unloading` decrements local countdown in the unload phase, not after a mission dispatch pass | `src/sim/miner/miner_dock_sequence.rs::phase_unloading`; future mission scheduler | read/check dump threshold before applying the current tick's accumulator increment | `mission_deploy_state3_reads_accumulator_before_current_ai_increment` | Do not decrement/increment the unload timer before evaluating the mission state-3 gate |
| Accepted unload-start does not cause a same-pass first increment | writes `+0x100=current`, `+0x108=1`; skip branch `0x006FABD0..0x006FAC2A` | Rust seeds `unload_timer = interval - 10`, which compresses the first wait by one local decrement step | `start_unload_deploy`, future timer-cluster model | after accepted unload-start, leave `+0xF8=0` through the rest of that same unit AI pass | `unload_start_same_ai_pass_does_not_increment_dump_accumulator` | Do not compensate by pre-decrementing the first unload interval unless proven byte-equivalent |
| The accumulator can advance on passes where mission dispatch is not due | `Mission_Dispatch` early return at `0x005B3060`; caller continues to `0x006FABC4` | Rust unload countdown is bound to the dock FSM phase, not a TechnoClass-level timer cluster | `Miner` state plus possible Techno timer component | keep the unload accumulator independent from whether mission `0x10` actually dispatched this tick | `unload_accumulator_advances_between_due_mission_dispatches` | Do not update accumulator only when `phase_unloading` dispatch logic runs |

Proposed Rust test names:

- `mission_deploy_state3_reads_accumulator_before_current_ai_increment`
- `unload_start_same_ai_pass_does_not_increment_dump_accumulator`
- `unload_accumulator_advances_between_due_mission_dispatches`

## 10. Negative Facts / Do Not Do

- Do not model the dump gate as seeing an increment performed later in the same `TechnoClass::AI_Update` pass.
- Do not increment `+0xF8` on the same pass that accepted unload-start initializes `+0x100=current frame` and `+0x108=1`.
- Do not tie accumulator advancement only to mission handler execution; gamemd advances the timer cluster later in `TechnoClass::AI_Update`, even when mission dispatch itself returns early due to mission delay.
- Do not use Rust's current `unload_timer = interval - 10` as Plan C byte-field parity. It is a bridge approximation, not the gamemd ordering.
- Do not use this report to claim `+0x110` initialization or `+0x104` meaning; those remain separate swarm slots.

## 11. Remaining Uncertainty

- Exact `Unit+0x110` default/writer is not resolved here.
- Exact `Unit+0x104` stack value source is not resolved here.
- Exact runtime first deposit frame requires combining this ordering with mission-table delay, frame-counter increment placement, and a runtime trace.

## 12. Stale Docs / Follow-up Docs

Suggested replacement wording for docs that imply `unload_timer`/`+0xF8` increments before the state-3 gate:

> `UnitClass::Mission_Deploy_Building` state 3 samples `Unit+0xF8` during `MissionClass::Mission_Dispatch`. The generic `TechnoClass::AI_Update` timer-cluster block that may increment `+0xF8` runs later in the same unit AI pass, so the gate sees only increments from earlier passes.

Suggested replacement wording for docs that imply first unload-start preloads a one-frame-decremented timer:

> Accepted unload-start writes `+0xF8=0`, `+0x100=g_CurrentFrameCounter`, `+0x108=1`, and `+0x10C=1`. The later same-pass accumulator block sees one frame remaining and does not increment `+0xF8`; the first increment is possible only on a later `TechnoClass::AI_Update` pass.

## Sources

- Ghidra read-only decompile/disassembly: `TechnoClass::AI_Update @ 0x006F9E50`
- Ghidra read-only decompile: `FootClass::AI @ 0x004DA530`
- Ghidra read-only decompile: `UnitClass::AI @ 0x007360C0`
- Ghidra read-only decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`
- Ghidra read-only decompile/disassembly: `UnitClass::Mission_Deploy_Building @ 0x0073D630`
- Ghidra xrefs: `0x004DA539`, `0x006FA655`, `0x0073647B`, `0x007F5EAC`
- Existing docs read/reconciled: `docs/research/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_STATE3_STATE4_TIMING_GHIDRA_REPORT.md`, `docs/research/UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`, `docs/research/STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`, `docs/research/UNIT_MISSION_DEPLOY_BUILDING_UNLOAD_START_IMPLEMENTATION_VERIFICATION_GHIDRA_REPORT.md`
- Rust scan only: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`, `src/sim/world/mod.rs`
