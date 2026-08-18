# Mission_Deploy_Building Docked vs Undocked Branch - Ghidra Research Report

**Address(es):** `0x0073D630` primary, `0x004595C0` release helper  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the top-level branch in `UnitClass::Mission_Deploy_Building` on unit `+0x2E4` (`param_1[0xB9]`), whether the standard stock refinery unload can finish through the `+0x2E4 == 0` branch, and which branch calls `BuildingClass::ReleaseDockedHarvester`.  
**Non-Scope:** writer inventory for `+0x2E4`, full ore economics, full radio protocol, bunker enter state machine, and post-release pathfinding internals beyond identifying the release call.  
**Confidence:** High for the branch layout and call site; Medium for stock-refinery writer absence because that part cites the prior arrival-link report rather than redoing the writer inventory in this slot.  
**Active in YR:** Yes for `Mission_Deploy_Building` and standard GAREFN/NAREFN unload; the docked `+0x2E4 != 0` release branch is Conditional on an external dock pointer already being set.

## 1. Overview

`UnitClass::Mission_Deploy_Building` begins by testing unit field `+0x2E4` before any harvester unload-state dispatch. If that field is zero, the function runs the normal deploy/refinery FSM, including stock harvester unload states 3 and 4. If it is nonzero, the function immediately looks up a building from the unit's current cell and calls `BuildingClass::ReleaseDockedHarvester`.

The stock refinery unload FSM does not need the `+0x2E4 != 0` branch to complete. Its `+0x2E4 == 0` path can drain cargo, transition to state 4, wait for refinery door anim completion, clear `+0x6D1`, set mission Harvest (`0x0A`), optionally transmit radio clear `3`, and queue the next mission.

## 2. Key Offsets

| Offset / expression | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Unit `+0x2E4` / `param_1[0xB9]` | Top-level docked pointer tested before the FSM | `0x0073D63B CMP [ESI+0x2E4], EBX`; decompile `if (param_1[0xb9] == 0)` | Conditional |
| Unit `+0x6C4` / `param_1[0x1B1]` | UnitTypeClass pointer | `0x0073D672 MOV EAX,[ESI+0x6C4]` | Yes |
| UnitType `+0xE0E` | `Harvester=yes` gate | `0x0073D678 MOV CL,[EAX+0xE0E]`, `rulesmd.ini:[CMIN]/[HARV] Harvester=yes` | Yes |
| UnitType `+0xE0F` | `Weeder=yes` gate | `0x0073D686 MOV CL,[EAX+0xE0F]`; no stock HARV/CMIN weeder | No for stock miners |
| Unit `+0xBC` / `param_1[0x2F]` | Mission substate for unload FSM | state writes `0x0073E51C`, `0x0073E594`; state-4 branch at `0x0073E17F` | Yes |
| Unit byte `+0x6D1` | unload initialized flag; cleared on depart prep | `0x0073E1F6 MOV byte [ESI+0x6D1],0` | Yes |
| Building `+0x57C` | refinery door/anim busy guard before stock state-4 exit | `0x0073E1DF CMP [EAX+0x57C], EBX` then wait if nonzero | Yes |
| BuildingType `+0x16BB` | `Refinery=yes` guard for state-4 door wait | `0x0073E1D5 MOV DL,[ECX+0x16BB]`; GAREFN/NAREFN `Refinery=yes` | Yes |

## 3. Core Logic

### 3.1 Entry branch on unit `+0x2E4`

The exact entry sequence is:

```text
0x0073D638  XOR EBX, EBX
0x0073D63B  CMP dword ptr [ESI + 0x2E4], EBX
0x0073D641  JZ 0x0073D6E6
```

Therefore:

- `unit+0x2E4 == 0`: jump to the undocked/deploy-FSM dispatch.
- `unit+0x2E4 != 0`: fall through to the release lookup and `ReleaseDockedHarvester`.

**Active in YR: Conditional.** The branch is in the live mission handler. The nonzero path is active only if some earlier code has already populated unit `+0x2E4`.

### 3.2 The docked branch calls `ReleaseDockedHarvester`

The nonzero branch performs two building-cell lookups and then calls the release helper:

```text
0x0073D647  MOV EAX,[ESI]
0x0073D649  CALL [EAX + 0x1BC]
0x0073D651  CALL 0x0047C520
0x0073D656  TEST EAX,EAX
0x0073D658  JZ 0x0073D672
0x0073D65A  MOV EDX,[ESI]
0x0073D65E  CALL [EDX + 0x1BC]
0x0073D666  CALL 0x0047C520
0x0073D66B  MOV ECX,EAX
0x0073D66D  CALL 0x004595C0
0x0073D672  MOV EAX,[ESI + 0x6C4]
```

`get_function_xrefs 0x004595C0` returns one callsite: `0x0073D66D` in `UnitClass__Mission_Deploy_Building`.

Important negative finding: there is no cargo-empty, state-4, or `FindFirstNonEmptySlot == -1` check before this call. It is controlled by the top-level `unit+0x2E4 != 0` test plus building lookup success. If a stock refinery arrival set this field at the start of unloading, this function would call release before running the normal state-3 drain block.

**Active in YR: Conditional.** The code is live, and `ReleaseDockedHarvester` itself is live when the field exists, but this slice did not find a stock GAREFN/NAREFN writer. Prior dock-arrival report identifies the reciprocal `+0x2E4` writer as Bunker-gated, not standard refinery arrival.

### 3.3 Common continuation after release

After release or lookup failure, execution falls through at `0x0073D672` and tests the unit type:

```text
0x0073D672  MOV EAX,[ESI + 0x6C4]
0x0073D678  MOV CL,[EAX + 0xE0E]
0x0073D67E  TEST CL,CL
0x0073D680  JNZ 0x0073DEE0
0x0073D686  MOV CL,[EAX + 0xE0F]
0x0073D68C  TEST CL,CL
0x0073D68E  JNZ 0x0073DEE0
```

For HARV/CMIN (`Harvester=yes`), the current invocation can still enter the harvester block after the release call. However, `ReleaseDockedHarvester` has already cleared the unit link, set a movement destination, and set mission `MOVE=2`, so subsequent behavior is already release-shaped.

**Active in YR: Yes for the harvester type gate; Conditional for arriving here from the docked branch.**

### 3.4 Undocked branch can complete stock refinery unload

When `unit+0x2E4 == 0`, the function reaches the standard harvester unload path. The stock completion path is state 3 -> state 4:

1. State 3 finds the refinery from the unit cell plus `DAT_0089F6A0`, drains storage on dump-rate threshold, and when `StorageClass::FindFirstNonEmptySlot` returns `-1`, it sets close-door anim slot 8 and writes `unit+0xBC = 4`.
   - Evidence: `0x0073E517 CALL 0x00451750` with slot `8`; `0x0073E51C MOV [ESI+0xBC],0x4`.
2. State 4 re-finds the refinery, waits while `Refinery=yes` and `building+0x57C != 0`, then clears `unit+0x6D1`.
   - Evidence: `0x0073E1D5` reads `Type+0x16BB`; `0x0073E1DF` checks `building+0x57C`; `0x0073E1F6 MOV byte [ESI+0x6D1],0`.
3. Normal stock harvester exit sets mission Harvest and queues the mission.
   - Evidence: `0x0073E24F PUSH 0`, `0x0073E250 PUSH 0xA`, `0x0073E254 CALL [EDX+0x1E8]`; `0x0073E283 CALL [EAX+0x1EC]`.
4. If path/radio contact is present, it sends radio clear `3` before queueing.
   - Evidence: `0x0073E275 PUSH 0x3`, `0x0073E279 CALL [EDX+0x274]`.

This is a complete unload-completion path without `ReleaseDockedHarvester`.

**Active in YR: Yes.** HARV/CMIN have `Harvester=yes`; GAREFN/NAREFN have `DockUnload=yes` and `Refinery=yes`. This state-4 path is not gated by TS-only flags.

### 3.5 How to reconcile the prior conflict

The old conflict came from treating `ReleaseDockedHarvester` as the normal cargo-empty state-4 handoff. The branch evidence does not support that framing: the release call is at the top of the function and is only guarded by `unit+0x2E4 != 0` and building lookup. The cargo-empty transition writes state 4 inside the `+0x2E4 == 0` unload FSM, and state 4 has its own completion path.

Inference from this slice plus the prior arrival-link report: standard GAREFN/NAREFN unload should be modeled as the `unit+0x2E4 == 0` path unless the unresolved writer swarm slot proves a stock-refinery writer exists. If such a writer exists, it must occur only after the stock FSM has completed or under an interrupt/teardown condition; otherwise the top-level branch would eject the miner before cargo drain.

## 4. INI Keys

| Key | Value | Effect | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[CMIN] Harvester` | `yes` | reaches harvester unload block at `0x0073DEE0` | Yes |
| `rulesmd.ini:[CMIN] Dock` | `NAREFN,GAREFN` | stock refinery candidate list | Yes |
| `rulesmd.ini:[HARV] Harvester` | `yes` | same Mission_Deploy_Building unload FSM | Yes |
| `rulesmd.ini:[GAREFN] DockUnload` | `yes` | stock Allied refinery admits dock unload | Yes |
| `rulesmd.ini:[GAREFN] Refinery` | `yes` | state-4 door guard and close-door anim branch | Yes |
| `rulesmd.ini:[NAREFN] DockUnload` | `yes` | stock Soviet refinery admits dock unload | Yes |
| `rulesmd.ini:[NAREFN] Refinery` | `yes` | same state-4 guard | Yes |

## 5. Integration Points

| Function | Role in this slice | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | mission handler containing both branches | full decompile, entry assembly context | Yes |
| `BuildingClass::ReleaseDockedHarvester @ 0x004595C0` | only called from the `+0x2E4 != 0` branch | xref `0x0073D66D`; release decompile | Conditional |
| `Look_up_building_in_cell @ 0x0047C520` | building lookup before release | calls at `0x0073D651`, `0x0073D666` | Yes |
| `BuildingClass::SetAnimSlotImage @ 0x00451750` | slot 8 on stock cargo-empty transition | call `0x0073E517` | Yes |
| unit vtable `+0x1E8` | state-4 normal exit sets mission Harvest | call `0x0073E254` | Yes |
| unit vtable `+0x274` | state-4 optional radio clear `3` | call `0x0073E279` | Yes |

## 6. Current Rust Implementation Status

Rust currently models a high-level refinery dock FSM in `src/sim/miner/miner_dock_sequence.rs` and `src/sim/miner/mod.rs`. It has explicit phases for `Approach`, `Linked`, `Pivoting`, `Unloading`, `DepositCooldown`, and `Departing`, and a Rust-side `RefineryDockContacts::on_pad` link in `src/sim/miner/miner_dock.rs`.

The comments currently describe `DepositCooldown` as matching a later `ReleaseDockedHarvester` transition, and `Departing` as the release moment. This report narrows the binary evidence: inside `Mission_Deploy_Building`, the stock `+0x2E4 == 0` unload FSM itself can complete by state 4 without calling `ReleaseDockedHarvester`; `ReleaseDockedHarvester` is reached only through the separate `+0x2E4 != 0` top branch.

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Entry `unit+0x2E4` branch | verified | `0x0073D63B`, `0x0073D641`; decompile | none |
| Docked branch release call | verified | `0x0073D647-0x0073D66D`; xref to `0x004595C0` | none |
| Release call guards | verified | branch assembly shows only nonzero field + building lookup before call | none |
| Common harvester continuation after release | verified | `0x0073D672-0x0073D68E` | exact same-tick side effects after release are not expanded |
| Stock state-3 empty transition | verified | `0x0073E517`, `0x0073E51C` | ore economics out of scope |
| Stock state-4 completion through undocked branch | verified | `0x0073E1D5-0x0073E283` | none for branch outcome |
| Stock refinery writer absence for `+0x2E4` | touched-not-exhausted | prior `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md` | assigned to other swarm slots |
| Full `ReleaseDockedHarvester` pathfinding and visuals | touched-not-exhausted | `0x004595C0` decompile read only for branch role | covered by post-unload exit report |

## 8. Open Questions - Final State

[RESOLVED] OQ1 - Which branch calls `ReleaseDockedHarvester`? The `unit+0x2E4 != 0` top branch calls it at `0x0073D66D`; no other xrefs were reported.

[RESOLVED] OQ2 - Is `ReleaseDockedHarvester` called by cargo-empty state 4 inside the stock unload FSM? No. The only call is before the harvester FSM dispatch and is gated by `unit+0x2E4 != 0`, not by state 4 or cargo-empty.

[RESOLVED] OQ3 - Can the stock `unit+0x2E4 == 0` path complete unload? Yes. State 3 sets state 4 when storage is empty; state 4 waits for door anim, clears `+0x6D1`, sets mission Harvest, optionally sends radio clear `3`, and queues the mission.

[RESOLVED] OQ4 - What happens if `unit+0x2E4 != 0` is set too early for a stock refinery miner? The top branch would call release before reaching the state-3 drain block. Therefore a stock-refinery writer, if proven later, cannot be an arrival-time prerequisite for ordinary draining without additional timing not visible in this function.

[RESOLVED] OQ5 - Is the docked release branch YR-active? Conditional. The code is live in YR, and the helper exists; this branch runs only when an external path sets unit `+0x2E4`. Prior arrival-link evidence says standard GAREFN/NAREFN arrival does not set it.

## Sources

- Ghidra decompiled: `UnitClass::Mission_Deploy_Building @ 0x0073D630`, `BuildingClass::ReleaseDockedHarvester @ 0x004595C0`.
- Ghidra assembly context: `0x0073D63B`, `0x0073D66D`, `0x0073E1F6`, `0x0073E254`, `0x0073E27F`, `0x0073E517`, `0x0073E594`.
- Ghidra xrefs: `get_function_xrefs 0x004595C0` -> `0x0073D66D` only.
- Prior docs read: `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`, `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`, `CHRONO_MINER_POST_UNLOAD_EXIT_ANCHOR_GHIDRA_REPORT.md`, `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`, plus base `ini/rules.ini` / `ini/art.ini` for corresponding stock entries.
- Rust scan: `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_dock_sequence.rs`.
