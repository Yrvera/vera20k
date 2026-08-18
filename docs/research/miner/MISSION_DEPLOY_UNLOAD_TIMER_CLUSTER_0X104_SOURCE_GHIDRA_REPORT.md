# Mission Deploy Unload Timer Cluster +0x104 Source - Ghidra Research Report

**Address(es):** `0x0073D630` (`UnitClass::Mission_Deploy_Building`), `0x006F9E50` (`TechnoClass::AI_Update`), `0x00737BA0` (`UnitClass::Unlimbo`), `0x0073D450` (`UnitClass::Harvest_Ore_Tick`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact source value written to `UnitClass+0x104` by the stock refinery unload-start block, and the local periodic-accumulator consumer contract for `+0xF8..+0x110`.
**Non-Scope:** dock radio `0x16` RateTimer semantics, PathType polarity, stock zero-link state 4, cargo-credit arithmetic, global xref proof that no other `+0x104` consumers exist anywhere in gamemd.
**Confidence:** High for the unload-start source and `TechnoClass::AI_Update` cluster contract; Medium for field naming/meaning because several older docs use stale "Z" wording.
**Active in YR:** Yes. The path is active for standard YR harvester/refinery unload once `Mission_Deploy_Building` reaches the stock refinery unload-start gate.

## 1. Overview

The `UnitClass+0x104` value written at stock refinery unload-start is not read from the unit position, not read from the building, and not returned by the preceding RateTimer/facing code. The live binary writes a stack local/scratch dword (`iStack_8`, storage equivalent to `[ESP+0x78]` at `0x0073DFF5`) into `UnitClass+0x104`.

Within the `+0xF8..+0x110` cluster, `+0x104` is not part of the elapsed-counter math. The cadence consumer in `TechnoClass::AI_Update` checks `+0x100`, `+0x108`, and `+0x10C`, increments `+0xF8` by `+0x110`, then overwrites `+0x104` with its own stack scratch dword.

## 2. Key Offsets

| Offset | Role in this slice | Active in YR | Evidence |
|---|---|---|---|
| `Unit+0xF8` | elapsed dump accumulator | Yes | `0x0073DFD0`, `0x006FABF7..0x006FAC06`, `0x0073E355..0x0073E374` |
| `Unit+0xFC` | timer-fired flag set by `AI_Update` on expiry | Yes | `0x006FABFF` |
| `Unit+0x100` | start frame for periodic accumulator | Yes | `0x0073DFF3`, `0x006FAC16` |
| `Unit+0x104` | scratch/secondary dword copied from stack in seed sites; not used by local cadence math | Yes | `0x0073DFF5..0x0073DFF9`, `0x006FAC12..0x006FAC1C` |
| `Unit+0x108` | current interval/duration | Yes | `0x0073DFFC`, `0x006FABCA`, `0x006FAC22` |
| `Unit+0x10C` | reload interval / active flag | Yes | `0x0073DFED`, `0x006FABE7` |
| `Unit+0x110` | increment step added to `+0xF8` | Yes | `0x006FABF1..0x006FAC06`; default/writer proof belongs to swarm slot 1 |

## 3. Core Findings

### Finding 1 - Mission_Deploy unload-start copies a stack scratch dword into `+0x104`

Active in YR: Yes.

At the first-entry unload-start block in `UnitClass::Mission_Deploy_Building`, the write order is:

1. `Unit+0xF8 = 0`
2. `Unit+0x6D1 = 1`
3. `EAX = g_CurrentFrameCounter`
4. `EDX = Unit+0x100`
5. `ECX = 1`
6. `Unit+0x10C = 1`
7. `[Unit+0x100] = g_CurrentFrameCounter`
8. `EAX = [ESP+0x78]`
9. `[Unit+0x104] = EAX`
10. `[Unit+0x108] = 1`
11. optional refinery slot 7 setup
12. `Unit+0xBC = 3`

Assembly evidence:

```asm
0073dfbd  MOV AL,byte ptr [ESI + 0x6d1]
0073dfc5  JNZ 0x0073e0a2
0073dfcb  MOV EBX,0x1
0073dfd0  MOV dword ptr [ESI + 0xf8],0x0
0073dfda  MOV byte ptr [ESI + 0x6d1],BL
0073dfe0  MOV EAX,[0x00a8ed84]
0073dfe5  LEA EDX,[ESI + 0x100]
0073dfeb  MOV ECX,EBX
0073dfed  MOV dword ptr [ESI + 0x10c],EBX
0073dff3  MOV dword ptr [EDX],EAX
0073dff5  MOV EAX,dword ptr [ESP + 0x78]
0073dff9  MOV dword ptr [EDX + 0x4],EAX
0073dffc  MOV dword ptr [EDX + 0x8],ECX
0073e093  MOV dword ptr [ESI + 0xbc],0x3
```

Decompiler evidence:

```c
param_1[0x3e] = 0;
*(undefined1 *)((int)param_1 + 0x6d1) = 1;
iVar3 = g_CurrentFrameCounter;
param_1[0x43] = 1;
param_1[0x40] = iVar3;
param_1[0x41] = iStack_8;
param_1[0x42] = 1;
param_1[0x2f] = 3;
```

The important correction is that the decompiler's `iStack_8` is a stack local/scratch value. It is not a current-coordinate read in this unload-start block.

### Finding 2 - Dataflow does not find a real producer for `iStack_8` inside the live unload-start path

Active in YR: Yes for the path; exact runtime stack contents are runtime-state dependent.

Ghidra `analyze_dataflow` at the `0x0073DFF9` store reports `iStack_8` as the store input. The backward chain reaches only indirect call effects and a control-flow phi, not an explicit assignment:

```text
0x0073df61 CALL 0x004c93d0  -> INDIRECT on iStack_8
0x0073dee2 CALL 0x0065ae30  -> INDIRECT on iStack_8
0x0073d672 MULTIEQUAL       -> control-flow merge
```

The two nearby calls on the live gate path are `PathType::Has_Valid_Steps` at `0x0065AE30` and `RateTimer::Current` at `0x004C93D0`. The RateTimer call receives an output pointer at `[ESP+0x4C]`, not `[ESP+0x78]`. The facing-window block writes and reads `[ESP+0x34]` for `0x4000`, also not `[ESP+0x78]`.

Implementation consequence: do not invent a semantic coordinate source for `Mission_Deploy_Building` unload-start `+0x104`. If modeling the byte field, the closest verified description is "copied stack scratch dword" for this write.

### Finding 3 - `TechnoClass::AI_Update` uses the cluster as a periodic accumulator and overwrites `+0x104`

Active in YR: Yes.

The cadence consumer is `TechnoClass::AI_Update @ 0x006F9E50`, after `MissionClass::Mission_Dispatch`. It skips buildings (`vtable+0x2C == 6`), then evaluates the timer:

```asm
006fabb8  CALL dword ptr [EAX + 0x2c]
006fabbf  CMP EAX,0x6
006fabc2  JZ 0x006fac31
006fabc4  MOV EDX,dword ptr [ESI + 0x100]
006fabca  MOV EAX,dword ptr [ESI + 0x108]
006fabd0  CMP EDX,-0x1
006fabd5  MOV ECX,dword ptr [0x00a8ed84]
006fabdb  SUB ECX,EDX
006fabdd  CMP ECX,EAX
006fabdf  JGE 0x006fabe7
006fabe1  SUB EAX,ECX
006fabe3  CMP EAX,EBP
006fabe5  JNZ 0x006fac2a
006fabe7  MOV EAX,dword ptr [ESI + 0x10c]
006fabed  CMP EAX,EBP
006fabef  JZ 0x006fac2a
006fabf1  MOV ECX,dword ptr [ESI + 0x110]
006fabf7  MOV EDX,dword ptr [ESI + 0xf8]
006fabfd  ADD EDX,ECX
006fabff  MOV byte ptr [ESI + 0xfc],0x1
006fac06  MOV dword ptr [ESI + 0xf8],EDX
006fac0c  MOV ECX,dword ptr [0x00a8ed84]
006fac12  MOV EDX,dword ptr [ESP + 0x2c]
006fac16  MOV dword ptr [ESI + 0x100],ECX
006fac1c  MOV dword ptr [ESI + 0x104],EDX
006fac22  MOV dword ptr [ESI + 0x108],EAX
```

This block proves the local consumer contract:

- if `Unit+0x100 != -1`, expiration is `g_CurrentFrameCounter - start >= Unit+0x108`;
- if `Unit+0x100 == -1`, the timer fires only when `Unit+0x108 == 0`;
- if expired and `Unit+0x10C != 0`, set `Unit+0xFC = 1`;
- add `Unit+0x110` to `Unit+0xF8`;
- set `Unit+0x100 = g_CurrentFrameCounter`;
- overwrite `Unit+0x104` from AI_Update stack scratch `[ESP+0x2C]`;
- set `Unit+0x108 = Unit+0x10C`;
- otherwise set `Unit+0xFC = 0`.

No branch in this local update reads old `Unit+0x104` to decide cadence or bale deposit timing.

### Finding 4 - Other seed sites show `+0x104` is generic timer scratch, not specifically unload-Z

Active in YR: Yes for harvester-type units.

`UnitClass::Unlimbo @ 0x00737BA0` seeds the same cluster. In both non-harvester and harvester/weeder branches it copies a stack local into `+0x104`:

```asm
00737c24  MOV dword ptr [EDX],EAX        ; +0x100 = current frame
00737c26  MOV EAX,dword ptr [ESP + 0x4]
00737c2a  MOV dword ptr [EDX + 0x4],EAX  ; +0x104
00737c2f  MOV dword ptr [EDX + 0x8],ECX  ; +0x108 = 0

00737c69  MOV dword ptr [EDX],EAX        ; +0x100 = current frame
00737c6b  MOV EAX,dword ptr [ESP + 0x8]
00737c70  MOV dword ptr [EDX + 0x4],EAX  ; +0x104
00737c75  MOV dword ptr [EDX + 0x8],ECX  ; +0x108 = 1
```

`UnitClass::Harvest_Ore_Tick @ 0x0073D450` writes `+0x104` from the unit's current object-coordinate Y local (`local_8` in the decompile), while `+0x100/+0x108/+0x10C` are used to arm the periodic extraction timer:

```c
local_c = param_1[0x27];  // Location_X
local_8 = param_1[0x28];  // Location_Y
local_4 = param_1[0x29];  // Location_Z
...
param_1[0x40] = g_CurrentFrameCounter;
param_1[0x41] = local_8;
param_1[0x42] = iVar1;
param_1[0x43] = iVar1;
```

This conflicts with older wording that called `+0x104` "Z-coord" without qualification. At least one active writer (`Harvest_Ore_Tick`) writes Y, and the unload-start writer copies stack scratch rather than any coordinate read.

### Finding 5 - The first dump cadence does not depend on `+0x104`

Active in YR: Yes.

The dump gate in state 3 compares:

```c
RulesClass+0x1528 HarvesterDumpRate * 900.0 <= (double)Unit+0xF8
```

The gate does not read `+0x104`. `+0x104` can drift or be overwritten without changing the local unload cadence, provided `+0xF8`, `+0x100`, `+0x108`, `+0x10C`, and `+0x110` match.

Evidence: `Mission_Deploy_Building @ 0x0073E355..0x0073E374` for the gate; `TechnoClass::AI_Update @ 0x006FABE7..0x006FAC22` for the periodic increment.

## 4. INI Keys

No INI key writes or directly gates `Unit+0x104`.

| INI key | Role | Active in YR | Evidence |
|---|---|---|---|
| `[General] HarvesterDumpRate` | state-3 dump threshold via `RulesClass+0x1528 * 900.0` | Yes | `0x0073E355..0x0073E374`; prior `UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md` |

## 5. Integration Points

- `Mission_Deploy_Building @ 0x0073D630` initializes the unload periodic accumulator when `Unit+0x6D1 == 0` and the RateTimer facing window accepts.
- `TechnoClass::AI_Update @ 0x006F9E50` runs the periodic accumulator update after mission dispatch in the same broad object AI function.
- `Mission_Deploy_Building` state 3 later reads `+0xF8` for the dump gate and resets `+0xF8` after a successful drain. It does not read `+0x104` in the stock dump gate.

## 6. Current Rust Implementation Status

Rust currently uses a higher-level miner unload timer rather than a byte-field `+0xF8..+0x110` accumulator:

- `src/sim/miner/miner_dock_sequence.rs` has `phase_unloading`, `snap.miner.unload_timer`, and state transitions into `RefineryDockPhase::Unloading`.
- `src/sim/miner/mod.rs` documents `unload_timer` in tenths-of-a-tick and `HarvesterDumpRate * 900`.
- No Rust field corresponding to `Unit+0x104` is present in the miner component.

Current Rust delta for this slot: `+0x104` is absent, which is acceptable for a bridge implementation if no Rust behavior claims byte-field parity. For Plan C byte-field parity, the field should exist only as a scratch/preserved dword; it must not be repurposed as semantic facing, target cell, or unload progress.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Mission_Deploy_Building` unload-start `+0x104` write | verified | decompile and assembly `0x0073DFBD..0x0073E09D` | none for source instruction |
| dataflow producer for unload-start `iStack_8` | verified-local | `analyze_dataflow 0x0073DFF9 iStack_8` | runtime stack contents can only be observed with debugger watch/trace |
| `TechnoClass::AI_Update` periodic accumulator block | verified | decompile and assembly `0x006FABB8..0x006FAC31` | none for local contract |
| `Harvest_Ore_Tick` same-cluster writer | verified | decompile and assembly `0x0073D450` | none for active writer identity |
| `UnitClass::Unlimbo` same-cluster writer | verified | decompile and assembly `0x00737BA0` | exact meaning of its stack scratch value is not needed for unload-start |
| global xrefs/readers of `Unit+0x104` outside this cluster | deferred | no full instruction-index sweep performed | separate global field-xref audit if Plan C wants save/load/global side effects |
| `Unit+0x110` default/writer | deferred | non-scope slot 1 | swarm slot 1 should own this |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What exact value does unload-start copy into Unit+0x104? -> The dword loaded from stack scratch/local `iStack_8`, assembly `[ESP+0x78]` at `0x0073DFF5`, is stored to `[Unit+0x104]` at `0x0073DFF9`.` (evidence: `0x0073DFF5..0x0073DFF9`)
- `[RESOLVED] OQ-02 - Is the source a coordinate read from the unit or refinery? -> No direct coordinate/object/building read feeds this store in the live unload-start block.` (evidence: `0x0073DFBD..0x0073E09D`; `analyze_dataflow 0x0073DFF9 iStack_8`)
- `[RESOLVED] OQ-03 - Does local unload cadence read old Unit+0x104? -> No. `AI_Update` uses `+0x100/+0x108/+0x10C/+0x110` for cadence and overwrites `+0x104`; `Mission_Deploy` state 3 gates on `+0xF8`.` (evidence: `0x006FABE7..0x006FAC22`, `0x0073E355..0x0073E374`)
- `[RESOLVED] OQ-04 - Is the path active in standard YR? -> Yes, this is the stock harvester/refinery deploy-unload mission path after path and facing gates pass.` (evidence: `0x0073D630` decompile; stock miner/refinery docs listed in Sources)
- `[RESOLVED] OQ-05 - Is older "Z-coord" wording reliable? -> No. `Harvest_Ore_Tick` writes current object-coordinate Y (`param_1[0x28]`) to `+0x104`, while unload-start writes stack scratch.` (evidence: `0x0073D450` decompile/assembly)
- `[DEFERRED] OQ-06 - Are there global consumers of Unit+0x104 outside the periodic accumulator?` (category: bounded-cost-too-high; reason: this slot was scoped to unload-start source and local consumer contract, not a whole-program field-xref audit; next-step-if-pursued: run global instruction/dataflow search for `Unit+0x104` with struct-aware field access tooling or targeted save/load decompilation)
- `[DEFERRED] OQ-07 - What exact runtime stack value sits in `iStack_8` in a live stock unload trace?` (category: needs-runtime-debugger; reason: static evidence proves source location but not runtime contents; next-step-if-pursued: watch `Unit+0x104` at `0x0073DFF9` and record `[ESP+0x78]` on a HARV/CMIN dock unload)
- `[DEFERRED] OQ-08 - What initializes `Unit+0x110` for stock harvesters?` (category: out-of-scope; reason: assigned to swarm slot 1; next-step-if-pursued: writer/default audit for `Unit+0x110`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Unload-start writes `+0x104` from stack scratch, not a semantic coordinate | `0x0073DFF5..0x0073DFF9`, dataflow at `0x0073DFF9` | missing byte-field; bridge does not model it | future miner byte-field state near `src/sim/miner/mod.rs` / dock FSM | If Plan C models the cluster, store an opaque scratch/preserved dword for `+0x104`; do not use it to drive unload behavior | `mission_deploy_unload_start_writes_opaque_timer_scratch_0x104` | Do not set `+0x104` to facing, pad cell, refinery id, or guaranteed Z |
| Local periodic accumulator cadence ignores old `+0x104` | `0x006FABE7..0x006FAC22`, `0x0073E355..0x0073E374` | Rust uses `unload_timer`, no byte-field accumulator | `src/sim/miner/miner_dock_sequence.rs::phase_unloading`; future shared timer helper | Cadence should depend on `+0x100/+0x108/+0x10C/+0x110` and `+0xF8`, not `+0x104` | `dock_unload_cadence_ignores_timer_cluster_0x104` | Do not block dump because `+0x104` is zero/stale/uninitialized |
| `AI_Update` overwrites `+0x104` whenever the periodic accumulator fires | `0x006FAC12..0x006FAC1C` | absent | future `TechnoClass::AI_Update` equivalent or sim tick timing layer | On timer expiry, after adding step to `+0xF8`, write current frame to `+0x100`, write scratch to `+0x104`, reload `+0x108=+0x10C` | `periodic_accumulator_expiry_overwrites_0x104_after_increment` | Do not treat the unload-start `+0x104` value as stable across the unload |

## 10. Negative Facts / Do Not Do

- Do not model unload-start `+0x104` as East facing, dock pad cell, refinery coordinate, target object id, or a hardcoded zero.
- Do not call `+0x104` "Z-coordinate" without qualification. The active `Harvest_Ore_Tick` writer stores current Y, and the active unload-start writer stores stack scratch.
- Do not use `+0x104` to decide when the first bale deposits. The local cadence path uses `+0xF8`, `+0x100`, `+0x108`, `+0x10C`, and `+0x110`.
- Do not claim whole-program "write-only" status for `+0x104` from this report. This report proves only the local unload-start source and local periodic-accumulator contract.

## 11. Remaining Uncertainty

- Runtime contents of `[ESP+0x78]` at `0x0073DFF5` were not sampled. Static evidence proves the source location, but a runtime watchpoint would be needed to say what value stock HARV/CMIN actually writes in one scenario.
- Global consumers/save-load treatment of `Unit+0x104` were not exhaustively audited.
- `Unit+0x110` initializer/default remains owned by the separate swarm slot.

## 12. Stale Docs / Follow-up Wording

Replace wording like:

> `Unit+0x104` is Z-coord/secondary storage.

with:

> `Unit+0x104` is a secondary scratch dword in the `+0xF8..+0x110` periodic accumulator. In `Mission_Deploy_Building` unload-start it is copied from a stack scratch dword (`iStack_8` / `[ESP+0x78]`), not from a current coordinate. In `Harvest_Ore_Tick` it is seeded from the current object-coordinate Y local. The local unload cadence does not read `+0x104`.

## Sources

- Live Ghidra MCP, project `testProsjekt:/gamemd.exe`
- `decompile_function 0x0073D630`
- `disassemble_function 0x0073D630`
- `analyze_dataflow 0x0073DFF9 variable=iStack_8`
- `decompile_function 0x006F9E50`
- `disassemble_function 0x006F9E50`
- `decompile_function 0x00737BA0`
- `disassemble_function 0x00737BA0`
- `decompile_function 0x0073D450`
- `disassemble_function 0x0073D450`
- `docs/research/UNIT_0x3E_BALE_CADENCE_TIMER_GHIDRA_REPORT.md`
- `docs/research/miner/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md`
- `docs/research/miner/DOCK_0X16_RATETIMER_MISSION_DEPLOY_PROOF_GATES_REINVESTIGATION_20260526.md`

**Status:** COMPLETE for the assigned slot scope.
