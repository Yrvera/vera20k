# Unit +0x110 Unload Accumulator Step Writers - Ghidra Research Report

**Address(es):** `0x006F2B40` (`TechnoClass::Constructor`), `0x006F9E50` (`TechnoClass::AI_Update`), `0x0073D630` (`UnitClass::Mission_Deploy_Building`), `0x00737BA0` (`UnitClass::Unlimbo`), `0x00737180` (`UnitClass::HarvestBrain_Idle`), `0x0073D450` (`UnitClass::Harvest_Ore_Tick`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `TechnoClass/UnitClass +0x110` as the periodic accumulator step used by stock HARV/CMIN refinery unload accumulation.
**Non-Scope:** dock `0x16` RateTimer semantics, East direction, PathType polarity, stock zero-link exit, cargo credit arithmetic, full save/load field serialization.
**Confidence:** High for constructor default, AI increment use, Mission_Deploy/Unlimbo/harvest non-writers in this slice; Medium for "all writers" because inline Ghidra scripts were disabled, so broad write inventory used byte-pattern searches plus required anchor decompiles instead of a custom operand-class script.
**Active in YR:** Yes. `TechnoClass::AI_Update` is the normal per-techno AI tick and `UnitClass::Mission_Deploy_Building` is the active mission `0x10` DockUnload handler for standard HARV/CMIN refinery unloading.

## 1. Overview

The `+0xF8..+0x110` cluster is a TechnoClass periodic accumulator, not a plain CDTimer. The load-bearing `+0x110` field is initialized by `TechnoClass::Constructor` to `1`, and `TechnoClass::AI_Update` adds that value into `+0xF8` each time the cluster's periodic timer fires.

For stock HARV/CMIN refinery unload state 3, `UnitClass::Mission_Deploy_Building` does not write `+0x110`; it starts the timer by writing `+0x10C = 1`, `+0x100 = g_CurrentFrameCounter`, `+0x104 = stack value`, and `+0x108 = 1`. Therefore the unload counter increments by exactly `1` per eligible AI_Update timer fire because the constructor already guaranteed `+0x110 = 1`.

## 2. Key Offsets

| Offset | Decompiler alias | Meaning in this slice | Writer/use evidence | Active in YR |
|---|---:|---|---|---|
| `+0xF8` | `param[0x3E]` | Accumulator value tested by harvest/unload code | incremented in `TechnoClass::AI_Update` at `0x006FAC06`; cleared by unload start at `0x0073DFD0` | Yes |
| `+0xFC` | byte | "changed this tick" marker for the cluster | set `1` at `0x006FABFF`, else cleared at `0x006FAC2A` | Yes |
| `+0x100` | `param[0x40]` | timer start frame | written from `g_CurrentFrameCounter` at `0x0073DFF3`; refreshed by AI_Update at `0x006FAC16` | Yes |
| `+0x104` | `param[0x41]` | secondary/Z stack value copied with timer state | unload start copies `[ESP+0x78]` through `EDX+4` at `0x0073DFF5..0x0073DFF9`; AI_Update copies stack local at `0x006FAC1C` | Yes |
| `+0x108` | `param[0x42]` | current duration/remaining interval | unload start writes `1` via `EDX+8` at `0x0073DFFC`; AI_Update reloads it from `+0x10C` at `0x006FAC22` | Yes |
| `+0x10C` | `param[0x43]` | repeat interval / active value | unload start writes `1` at `0x0073DFED`; zero disables the accumulator in AI_Update | Yes |
| `+0x110` | `param[0x44]` | per-fire increment step added to `+0xF8` | constructor writes `1` at `0x006F2B81`; AI_Update reads at `0x006FABF1` | Yes |

## 3. Core Logic

### 3.1 Constructor default proves `+0x110 == 1`

`TechnoClass::Constructor @ 0x006F2B40` initializes the accumulator cluster before UnitClass-specific construction. The relevant assembly has `EDI = 1` and writes that value to `+0x110`:

```text
006f2b46  CALL RadioClass__Constructor
006f2b4b  XOR EBX,EBX
006f2b4d  MOV EDI,0x1
006f2b5e  MOV dword ptr [ESI + 0xf8],EBX
006f2b6f  MOV dword ptr [ESI + 0x108],EBX
006f2b75  MOV dword ptr [ESI + 0x100],EAX
006f2b7b  MOV dword ptr [ESI + 0x10c],EBX
006f2b81  MOV dword ptr [ESI + 0x110],EDI
```

**Active in YR:** Yes. Unit construction at `UnitClass::Constructor @ 0x007353C0` calls the base constructor chain, and standard HARV/CMIN are UnitClass objects.

### 3.2 AI_Update adds `+0x110` to `+0xF8`

`TechnoClass::AI_Update @ 0x006F9E50` runs the periodic cluster after mission dispatch. It first checks whether the timer has expired using `+0x100` and `+0x108`; when expired and `+0x10C != 0`, it reads `+0x110`, adds it to `+0xF8`, marks `+0xFC = 1`, writes the new accumulator, refreshes start frame, copies the current stack Z/secondary value to `+0x104`, and reloads `+0x108` from `+0x10C`.

```text
006fabe7  MOV EAX,dword ptr [ESI + 0x10c]
006fabed  CMP EAX,EBP
006fabef  JZ 006fac2a
006fabf1  MOV ECX,dword ptr [ESI + 0x110]
006fabf7  MOV EDX,dword ptr [ESI + 0xf8]
006fabfd  ADD EDX,ECX
006fabff  MOV byte ptr [ESI + 0xfc],0x1
006fac06  MOV dword ptr [ESI + 0xf8],EDX
006fac0c  MOV ECX,dword ptr [g_CurrentFrameCounter]
006fac12  MOV EDX,dword ptr [ESP + 0x2c]
006fac16  MOV dword ptr [ESI + 0x100],ECX
006fac1c  MOV dword ptr [ESI + 0x104],EDX
006fac22  MOV dword ptr [ESI + 0x108],EAX
006fac2a  MOV byte ptr [ESI + 0xfc],0x0
```

**Active in YR:** Yes. This is the normal TechnoClass AI tick for units; it is not a TS-only branch.

### 3.3 Mission_Deploy_Building starts unload without writing `+0x110`

In `UnitClass::Mission_Deploy_Building @ 0x0073D630`, once path and facing gates accept and `+0x6D1 == 0`, unload-start writes the cluster as follows:

```text
0073dfbd  MOV AL,byte ptr [ESI + 0x6d1]
0073dfc5  JNZ 0073e0a2
0073dfcb  MOV EBX,0x1
0073dfd0  MOV dword ptr [ESI + 0xf8],0x0
0073dfda  MOV byte ptr [ESI + 0x6d1],BL
0073dfe0  MOV EAX,[g_CurrentFrameCounter]
0073dfe5  LEA EDX,[ESI + 0x100]
0073dfed  MOV dword ptr [ESI + 0x10c],EBX
0073dff3  MOV dword ptr [EDX],EAX        ; +0x100
0073dff5  MOV EAX,dword ptr [ESP + 0x78]
0073dff9  MOV dword ptr [EDX + 0x4],EAX  ; +0x104
0073dffc  MOV dword ptr [EDX + 0x8],ECX  ; +0x108, ECX == 1
0073e093  MOV dword ptr [ESI + 0xbc],0x3
```

There is no `+0x110` write in this unload-start block. The unload path relies on the already-initialized constructor value.

**Active in YR:** Yes for stock HARV/CMIN DockUnload refineries after the Mission_Deploy path/facing gates accept.

### 3.4 Required anchor non-writers

`UnitClass::Unlimbo @ 0x00737BA0` writes the same cluster for spawn/placement paths:

- non-harvester/weeder branch: `+0xF8=0`, `+0x10C=0`, `+0x100=current frame`, `+0x104=local_8`, `+0x108=0`;
- harvester/weeder branch: `+0xF8=RandomRanged(0,0x1D)`, `+0x10C=1`, `+0x100=current frame`, `+0x104=local_8`, `+0x108=1`.

It does not write `+0x110`. **Active in YR:** Yes for UnitClass unlimbo/spawn, but not an unload-state writer.

`UnitClass::HarvestBrain_Idle @ 0x00737180` writes `+0x100/+0x104/+0x108/+0x10C` while managing idle harvest cadence and rediscovery, and can randomize `+0xF8`; it does not write `+0x110`. **Active in YR:** Yes for harvesters, but not an unload-state writer.

`UnitClass::Harvest_Ore_Tick @ 0x0073D450` writes `+0xF8=0`, `+0x100=current frame`, `+0x104=current Y/stack cell value`, and sets `+0x108/+0x10C` from `HarvesterLoadRate` (or `HarvesterLoadRate*3` for weeders). It does not write `+0x110`. **Active in YR:** Yes for harvesters, but harvest/load cadence, not refinery unload deposit cadence.

## 4. INI Keys

| INI key | Default / stock relevance | Direct effect on `+0x110`? | Evidence |
|---|---|---:|---|
| `[General] HarvesterDumpRate=` | `0.016`; state-3 dump gate compares `HarvesterDumpRate * 900.0 <= +0xF8` | No | existing `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`, this report confirms `+0xF8` step source |
| `[General] HarvesterLoadRate=` | load/ore-harvest cluster duration source | No | `UnitClass::Harvest_Ore_Tick @ 0x0073D450` |
| `[UnitType] Harvester=yes` | reaches harvester branches in Mission_Harvest/Mission_Deploy | No | stock HARV/CMIN rules and `UnitClass::Mission_Deploy_Building` |
| `[UnitType] Weeder=yes` | alternative harvester-like branch | No | `Mission_Deploy_Building` branches on `type+0xE0F` |

No INI reader was found or needed for `+0x110`: the active default is a constructor constant.

## 5. Integration Points

- `UnitClass::Mission_Deploy_Building` initializes unload-active accumulation when the stock path/facing gates accept.
- `TechnoClass::AI_Update` then increments `+0xF8` outside the mission handler when the timer fires.
- The next `Mission_Deploy_Building` state-3 dispatch compares `RulesClass+0x1528 HarvesterDumpRate * 900.0` against `+0xF8` before draining one storage slot.
- The `+0x110` step is not CMIN-specific. It is TechnoClass state and applies equally to HARV and CMIN while they use the same UnitClass unload FSM.

## 6. Current Rust Implementation Status

Current Rust does not model this byte-field cluster directly. Relevant surfaces:

- `src/sim/miner/mod.rs:263-267`: `Miner::unload_timer` stores a local countdown in tenths-of-a-tick.
- `src/sim/miner/miner_dock_sequence.rs:805-835`: `start_unload_deploy` enters the unload phase, currently snaps facing and seeds `unload_timer = unload_tick_interval - 10`.
- `src/sim/miner/miner_dock_sequence.rs:854-935`: `phase_unloading` decrements the local countdown by `10`, drains one resource slot at a threshold, and then adds `unload_tick_interval`.
- `src/sim/world/mod.rs:1114`: `binary_frame` is derived at the start of Rust tick processing, while gamemd's global frame counter is known from prior reports to increment near the end of the frame.

The current model can match coarse deposit cadence but is not byte-field equivalent to the constructor/AI_Update accumulator.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::Constructor @ 0x006F2B40` `+0x110` default | verified | decompile plus assembly `0x006F2B4B..0x006F2B81` | none |
| `TechnoClass::AI_Update @ 0x006F9E50` increment use | verified | decompile plus assembly `0x006FABE7..0x006FAC22` | none |
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` unload start | verified | decompile plus assembly context `0x0073DFBD..0x0073E093` | exact meaning/source of the `+0x104` stack value belongs to slot 2 |
| `UnitClass::Unlimbo @ 0x00737BA0` | verified-for-this-field | decompile shows cluster writes but no `+0x110` write | full unlimbo semantics out-of-scope |
| `UnitClass::HarvestBrain_Idle @ 0x00737180` | verified-for-this-field | decompile shows `+0x100/+0x104/+0x108/+0x10C` writes but no `+0x110` write | full harvest idle semantics out-of-scope |
| `UnitClass::Harvest_Ore_Tick @ 0x0073D450` | verified-for-this-field | decompile shows load timer writes but no `+0x110` write | full harvest load semantics out-of-scope |
| Whole-program direct writer inventory | touched-not-exhausted | byte-pattern searches found the constructor direct write and many unrelated `+0x110` displacements from other classes; inline operand script disabled | custom Ghidra script could produce a cleaner global writer table |
| Save/load restore of `+0x110` | deferred | not required to answer live stock default | if save/load byte parity is implemented, verify serialization separately |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-110-001 - What value is live in `+0x110` before stock HARV/CMIN enters unload state 3? -> `1`, written by `TechnoClass::Constructor` at `0x006F2B81`.` (evidence: `0x006F2B40` decompile and assembly)
- `[RESOLVED] OQ-110-002 - Does Mission_Deploy_Building write `+0x110` during unload-start? -> No; the unload-start block writes `+0xF8`, `+0x6D1`, `+0x10C`, `+0x100`, `+0x104`, `+0x108`, and `+0xBC`, but no `+0x110`.` (evidence: `0x0073DFBD..0x0073E093`)
- `[RESOLVED] OQ-110-003 - What consumes `+0x110` for the accumulator? -> `TechnoClass::AI_Update` reads `+0x110` and adds it to `+0xF8` when `+0x10C != 0` and the `+0x100/+0x108` timer has expired.` (evidence: `0x006FABE7..0x006FAC22`)
- `[RESOLVED] OQ-110-004 - Does Unlimbo override the step? -> No; it initializes the active/duration/start/value cluster, but not `+0x110`.` (evidence: `0x00737BA0` decompile)
- `[RESOLVED] OQ-110-005 - Do HarvestBrain_Idle or Harvest_Ore_Tick override the step? -> No; both write timer/accumulator fields for harvest cadence, but not the step field.` (evidence: `0x00737180`, `0x0073D450` decompiles)
- `[DEFERRED] OQ-110-006 - Is every whole-program writer to any class's byte offset `+0x110` classified?` (category: `bounded-cost-too-high`; reason: many unrelated classes use offset `+0x110`, and scripts are disabled; next-step-if-pursued: run a read-only operand-class Ghidra script with function/class filtering)
- `[DEFERRED] OQ-110-007 - How is `+0x110` serialized and restored?` (category: `out-of-scope`; reason: the target question is live stock default before unload, not save/load; next-step-if-pursued: inspect TechnoClass save/load serializers)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `+0x110` is constructor-defaulted to `1` for TechnoClass-derived units | `0x006F2B81` | missing byte-field | `GameEntity` / miner timing state if Plan C is implemented | initialize accumulator step to exactly `1` for units that own the Techno accumulator | `unload_accumulator_step_defaults_to_one_for_harv_and_cmin` | Do not derive this from INI or harvester type |
| `AI_Update` increments `+0xF8` by `+0x110` when `+0x10C != 0` and timer expires | `0x006FABE7..0x006FAC22` | Rust uses local countdown | `src/sim/miner/miner_dock_sequence.rs`, possible shared Techno timer helper | model an accumulator that increases `+0xF8` by step `1`, reloads `+0x108` from `+0x10C`, refreshes `+0x100`, and updates `+0x104` | `techno_ai_update_unload_cluster_increments_f8_by_step_one` | Do not call the current countdown equivalent to byte-field parity |
| Unload-start does not write `+0x110`; it writes `+0x10C=1` and `+0x108=1` | `0x0073DFD0..0x0073DFFC` | current Rust seeds countdown directly | `start_unload_deploy`, future Mission_Deploy bridge | start the periodic accumulator without mutating the step | `mission_deploy_unload_start_preserves_accumulator_step_one` | Do not reset step on every dock cycle unless save/load/constructor semantics require it |
| HARV and CMIN share this path | same UnitClass mission and TechnoClass AI tick | Rust already shares miner dock sequence | `MinerKind` branches should not special-case accumulator step | use identical step/default for both | `cmin_and_harv_share_unload_accumulator_step` | Do not make Chrono-specific timing here |

Proposed Rust test names:

- `unload_accumulator_step_defaults_to_one_for_harv_and_cmin`
- `techno_ai_update_unload_cluster_increments_f8_by_step_one`
- `mission_deploy_unload_start_preserves_accumulator_step_one`
- `cmin_and_harv_share_unload_accumulator_step`

## 10. Negative Facts / Do Not Do

- Do not set `+0x110` in `Mission_Deploy_Building`; gamemd does not.
- Do not derive the step from `HarvesterDumpRate`; that rate is the threshold, while `+0x110` is the per-fire accumulator increment.
- Do not treat `+0x108` as the step. In unload start, `+0x108 = 1` is the timer duration/current interval; `+0x110 = 1` is the increment step.
- Do not special-case Chrono Miners. The same TechnoClass accumulator and UnitClass unload mission serve CMIN and HARV.
- Do not use the many unrelated `+0x110` hits from bullets, building animation, UI, or weapon types as evidence about Unit/Techno accumulator state.

## 11. Remaining Uncertainty

- Slot 2 still owns the exact source/meaning of the `+0x104` stack value copied at unload start (`0x0073DFF5..0x0073DFF9`) and refreshed by AI_Update (`0x006FAC1C`).
- A future save/load parity pass should verify whether TechnoClass serialization preserves `+0x110`, but that does not block live stock construction/unload behavior.
- Whole-program writer inventory would be cleaner with a custom read-only operand script; scripts were disabled in this Ghidra MCP session.

## Stale Docs / Follow-up Wording

- In `docs/research/UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md`, replace: "`field_0x110 = 1` (default, not yet verified)" with: "`field_0x110 = 1` is constructor-verified in `TechnoClass::Constructor @ 0x006F2B81`; `Mission_Deploy_Building` does not rewrite it during unload-start."
- In `docs/research/miner/DOCK_0X16_RATETIMER_MISSION_DEPLOY_PROOF_GATES_REINVESTIGATION_20260526.md`, replace the `+0x110` blocker wording with: "`+0x110` default/writer is resolved by `UNIT_0X110_UNLOAD_ACCUMULATOR_STEP_WRITERS_GHIDRA_REPORT.md`; remaining byte-field blocker is the exact `+0x104` stack/Z source and any save/load serialization details if required."

## Sources

- Ghidra read-only decompile/disassembly: `TechnoClass::Constructor @ 0x006F2B40`
- Ghidra read-only decompile/disassembly: `TechnoClass::AI_Update @ 0x006F9E50`
- Ghidra read-only decompile/assembly context: `UnitClass::Mission_Deploy_Building @ 0x0073D630`
- Ghidra read-only decompile: `UnitClass::Unlimbo @ 0x00737BA0`
- Ghidra read-only decompile: `UnitClass::HarvestBrain_Idle @ 0x00737180`
- Ghidra read-only decompile: `UnitClass::Harvest_Ore_Tick @ 0x0073D450`
- Existing docs: `docs/research/UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md`
- Existing docs: `docs/research/miner/DOCK_0X16_RATETIMER_MISSION_DEPLOY_PROOF_GATES_REINVESTIGATION_20260526.md`
- Rust scan: `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/world/mod.rs`
