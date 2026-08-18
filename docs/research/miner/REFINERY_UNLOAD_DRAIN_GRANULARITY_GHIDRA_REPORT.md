# Refinery Unload Drain Granularity and Cadence - Ghidra Research Report

**Address(es):** `0x0073D630` (`UnitClass::Mission_Deploy_Building`), `0x006C9820` (`StorageClass::FindFirstNonEmptySlot`), `0x006C9680` (`StorageClass::GetAmount`), `0x006C96B0` (`StorageClass::RemoveAmount`), `0x00670CD4` (`RulesClass::ReadGeneral`, `HarvesterDumpRate` parse site)
**Investigation Mode:** exhaustive-slice (single question: drain quantum + cadence + terminating pulse)
**Claimed Scope:** what one refinery-unload pulse drains (bale vs. whole StorageClass slot vs. dump-attempt), the exact 14.4-frame gate formula, and whether a final empty/terminating pulse exists and what triggers state-3 -> state-4 exit, for stock `CMIN/HARV -> GAREFN/NAREFN` unloading.
**Non-Scope:** enter/radio handshake (mission-enter, radio `0x0E/0x15/0x18`), the reciprocal `+0x2E4`/`Force_Track(0x47)` conditional exit path, the harvest-at-ore-field loop (`HarvesterLoadRate`), two-miner queue handoff, modded `ProductionAnim` lifetime.
**Confidence:** High. Every claim below was re-verified this session via `disassemble_function(0x0073D630)` (full-function assembly) and `read_memory` on the two floating-point constants, not paraphrased from decompile alone.
**Active in YR:** Yes. Stock `[CMIN]`/`[HARV]` have `Harvester=yes` (`rulesmd.ini:7364`, `:8228`) and stock `[GAREFN]`/`[NAREFN]` have `DockUnload=yes`/`Refinery=yes` (`rulesmd.ini:11726-11727`, `:12519-12520`); this is the default unload path for every stock skirmish/campaign refinery.

## 1. Overview

**Verdict: one pulse drains one whole `StorageClass` resource-type slot, not one bale and not a fixed "dump attempt" amount.** `StorageClass::RemoveAmount` is called with the exact value just read by `StorageClass::GetAmount` for that slot, so the removal always empties the slot to zero in a single gate crossing, regardless of how large that slot's amount is (a pure-ore load of 20 bales drains in ONE pulse, not 20).

The gate cadence is `HarvesterDumpRate * 900.0 <= UnitClass+0xF8` (accumulator), independently re-verified: the `900.0` multiplier is confirmed by `read_memory(0x007e27f8)` = IEEE-754 double `900.0`, and `RulesClass+0x1528` is confirmed to be `HarvesterDumpRate` by the string xref from `RulesClass::ReadGeneral`. Stock `rulesmd.ini`/`rules.ini` do not set `[General] HarvesterDumpRate` (only `PurifierBonus` appears near that block), so the binary's pre-loaded default of `0.016` stands, giving **14.4 frames per gate**.

**A real terminating pulse exists and it is genuinely empty.** When `StorageClass::FindFirstNonEmptySlot` returns `-1` (no slot has `amount > 0`) on a gate crossing, the code does not drain anything or credit any amount — it writes mission substate `+0xBC = 4`, optionally requests `SetAnimSlotImage(8, ...)` (`ProductionAnim`, gated on `Refinery=yes`), clears slot 10 (`SpecialAnim`) if occupied, and direct-returns `1` from state 3. State 4 itself runs on the *next* mission call, not the same call. So a cargo of N populated slot-types costs exactly `N + 1` gate crossings: N real drains (one per non-empty slot, ascending index order) plus one terminating empty crossing that only flips the state.

## 2. Class Layout / Key Offsets

| Field | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `UnitClass+0xBC` (`param_1[0x2F]`) | `0xBC` | Deploy-building mission substate (`3`=dump loop, `4`=finish) | `0x0073E51C` writes `4`; `0x0073E0A8..0x0073E0B2` dispatches on `3`/`4` | Yes |
| `UnitClass+0xF8` (`param_1[0x3E]`) | `0xF8` | Dump-rate accumulator (int32, read as float via `FILD`) | gate `0x0073E35B..0x0073E374`; reset `0x0073E493`, `0x0073E4D0` | Yes |
| `UnitClass+0x6D1` | `0x6D1` | Unload-active byte; set on first state-3 entry, cleared in state 4 | set `0x0073DFDA`; clear `0x0073E0D2` (Weeder), `0x0073E1F6` (non-Weeder) | Yes |
| `UnitClass+0x33C` | `0x33C` | Embedded harvester `StorageClass` (4 contiguous floats, one per Tiberium-type index) | `LEA ECX,[ESI+0x33C]` at `0x0073E3C5`/`0x0073E418`/`0x0073E457` (this-pointer to all three `StorageClass` calls) | Yes |
| `RulesClass+0x1528` | `0x1528` | `HarvesterDumpRate` (double) | `FLD double ptr [EDX+0x1528]` at `0x0073E361`; `FSTP double ptr [ESI+0x1528]` at `0x00670CE1` immediately after `PUSH 0x83be4c` ("HarvesterDumpRate") at `0x00670CD4` | Yes |
| `BuildingClass+0x57C` | `0x57C` | `Anims_0[8]` = `ProductionAnim` pointer | state-4 wait guard `0x0073E1DF..0x0073E1EA` | Conditional (stock GAREFN/NAREFN leave it null) |
| `BuildingClass+0x584` | `0x584` | `Anims_0[10]` = `SpecialAnim` pointer | cleared `0x0073E526..0x0073E534`, `0x0073E59E..0x0073E5AC` | Yes |
| Constant at `0x007E27F8` | — | IEEE-754 double `900.0` (frames per HarvesterDumpRate unit) | `read_memory(0x007e27f8,8)` = bytes `00 00 00 00 00 20 8c 40` = `0x408C200000000000` = `900.0` decoded and hand-checked this session | Yes |
| Constant at `0x007E1748` | — | IEEE-754 single `0.0f` (slot-occupied / removed-amount-positive threshold) | `read_memory(0x007e1748,4)` = `00 00 00 00` = `0.0f` | Yes |

## 3. Core Logic

### 3.1 Gate: 14.4-frame threshold, re-armed per real drain

Disassembly `0x0073E355..0x0073E374`:

```text
0073e355  MOV EDX,[0x8871e0]          ; g_RulesClass_Instance
0073e35b  FILD dword ptr [ESI+0xF8]   ; ST0 = (float)accumulator
0073e361  FLD double ptr [EDX+0x1528] ; ST0 = HarvesterDumpRate, ST1 = accumulator
0073e367  FMUL double ptr [0x7e27f8]  ; ST0 = HarvesterDumpRate * 900.0
0073e36d  FCOMPP                      ; compare threshold vs accumulator, pop both
0073e371  TEST AH,0x41
0073e374  JZ 0x0073e539               ; branch away (no drain this call) when threshold > accumulator
```

Default `HarvesterDumpRate=0.016` (absent from `rulesmd.ini`/`rules.ini`, confirmed by grep) gives `0.016 * 900.0 = 14.4` frames. This is the interval between gate *evaluations*, not a per-bale interval — the gate fires once per 14.4 frames regardless of how much or how little is drained on that crossing.

### 3.2 Real drain: whole slot, not a bale

`0x0073E3BF..0x0073E45C`:

```text
0073e3bf  LEA ECX,[ESI+0x33C]         ; this = harvester StorageClass
0073e3c5  CALL 0x006c9820             ; FindFirstNonEmptySlot() -> EBP (slot index or -1)
...
0073e40f  JZ 0x0073e423               ; EBP == -1: amount = 0.0, skip GetAmount
0073e412  LEA ECX,[ESI+0x33C]
0073e418  CALL 0x006c9680             ; GetAmount(EBP) -> amount, stored at [ESP+0x18]
...
0073e445  JZ 0x0073e4dc               ; EBP == -1 -> jump straight to the terminating block (3.3)
0073e44b  MOV ECX,[ESP+0x18]          ; ECX = amount just read
0073e450  PUSH ECX                    ; push amount
0073e451  LEA ECX,[ESI+0x33C]
0073e457  CALL 0x006c96b0             ; RemoveAmount(amount, EBP) -- requests removal of the FULL current amount
```

`StorageClass::FindFirstNonEmptySlot` (`0x006C9820`) scans exactly 4 float slots (`iVar1 < 4`), returning the first index whose value is `> 0.0f`, else `-1` — matching stock `[Tiberiums]` (`rulesmd.ini:30372-30376`: `0=Riparius`(Ore), `1=Cruentus`(Gems), `2=Vinifera`, `3=Aboreus`). `StorageClass::GetAmount` (`0x006C9680`) is a pure read: `return *(float*)(this + slot*4)`. `StorageClass::RemoveAmount` (`0x006C96B0`) is called with that same just-read value as the requested amount, so its "saturate if requested > available" branch never triggers here — the slot is driven to exactly `0` in one call, and the FPU-register return value (`ST0`, not popped by `FST`) equals the full amount removed. Only the **first** non-empty slot is touched per gate crossing; a slot with both Ore and Gems present drains Ore on one crossing and Gems on a later crossing, never both in the same pulse.

On a positive removal, the accumulator resets (`0x0073E493` Weeder path, `0x0073E4D0` non-Weeder path: `MOV dword ptr [ESI+0xF8],0x0`), credits are applied via `HouseClass__Add_Tiberium_Credits`/`Add_Tiberium_To_Storage`, and the function falls to `0x0073E539` — **state stays `3`**; it does not write `4` on a real drain.

### 3.3 Terminating pulse: empty gate writes state 4, credits nothing

Reached from `0x0073E445` (`FindFirstNonEmptySlot == -1`) or `0x0073E46B` (`RemoveAmount` result `<= 0.0f`, an unreachable-in-practice edge given 3.2's exact-amount call). Block `0x0073E4DC..0x0073E539`:

```text
0073e4dc  MOV EDX,[EDI+0x520]         ; refinery Type ptr
0073e4e2  MOV AL,[EDX+0x16BB]         ; Refinery=yes ?
0073e4ea  JZ  0x0073e51c              ; skip slot-8 request if not a Refinery
0073e4ec..0073e517                    ; SetAnimSlotImage(8, damaged, 0) -- ProductionAnim one-shot request
0073e51c  MOV dword ptr [ESI+0xBC],0x4   ; <-- STATE 3 -> STATE 4 WRITE
0073e526  MOV EAX,[EDI+0x584]         ; SpecialAnim (slot 10) pointer
0073e52e  JZ  0x0073e539              ; skip if not occupied
0073e530..0073e534                    ; ClearAnimSlot(10)
0073e539  ...                          ; shared post-check, eventual RET 1 at 0x0073e5b4
```

No `HouseClass__Add_Tiberium_Credits`/`Add_Tiberium_To_Storage` call and no `unit+0xF8` reset occur on this path — the pulse is genuinely empty. State `3`'s switch case then falls through to a direct `return 1` at `0x0073E5B4`; state `4` executes on the **next** `Mission_Deploy_Building` call, not inside this one (confirmed: no fallthrough/jump from the `0x0073E51C` block back into the state-4 dispatch arm at `0x0073E0A8`).

### 3.4 State 4: `+0x6D1` clears only after this next call

`0x0073DFDA` sets `+0x6D1=1` on first entry to state 3 (`MOV byte ptr [ESI+0x6D1],BL` with `BL=1`, gated by `if (*(char*)(ESI+0x6D1)==0)` at `0x0073DFBD..0x0073DFC5`). Clearing happens only in state 4, on the mission call *after* the terminating pulse:

- Weeder branch (`UnitType+0xE0F != 0`): unconditional clear at `0x0073E0D2`.
- Non-Weeder branch (stock `HARV`/`CMIN`): rediscovers the refinery (`0x0073E17F..0x0073E1C6`), and if `Refinery=yes` **and** `building+0x57C` (slot-8 `ProductionAnim` pointer) is still non-null, direct-returns `1` at `0x0073E1EA` **without** clearing `+0x6D1` (i.e., waits another mission call). Otherwise clears at `0x0073E1F6`. For stock `GAREFN`/`NAREFN`, `+0x57C` never gets populated (no active stock `ProductionAnim`, `artmd.ini:1749` commented for NAREFN, absent for GAREFN), so the wait is normally a no-op and the clear fires on the very next call after the terminating pulse.

## 4. INI Keys

| Key | Stock value | Effect | Active in YR |
|---|---|---|---|
| `[General] HarvesterDumpRate` | absent; binary pre-loaded default `0.016` | `0.016 * 900.0 = 14.4` frames per gate evaluation | Yes |
| `[Tiberiums] 0..3` | `Riparius, Cruentus, Vinifera, Aboreus` (`rulesmd.ini:30372-30376`) | matches the 4-slot `StorageClass` array bound (`iVar1 < 4`) that `FindFirstNonEmptySlot` scans | Yes |
| `[Tiberium] Value` per type | credit value "per 'bail'" (comment, `rulesmd.ini:30380`) | this is the credit-per-bale **conversion rate** used for display/credit math, a different concept from the runtime drain quantum this report covers | Yes |
| `[CMIN]/[HARV] Storage` | `20`/`40` | total cargo bale capacity (unrelated to per-pulse quantum) | Yes |
| `[CMIN]/[HARV] Harvester` | `yes`/`yes` | routes to this unload FSM via `UnitType+0xE0E` | Yes |
| `[GAREFN]/[NAREFN] Refinery` | `yes`/`yes` | gates the slot-8 `ProductionAnim` request and the state-4 wait guard | Yes |
| `[GAREFN]/[NAREFN] ProductionAnim` | absent/commented | keeps `building+0x57C` null, so the state-4 wait is normally a no-op | Conditional |

## 5. Integration Points

- `UnitClass::Mission_Deploy_Building @ 0x0073D630` owns the entire state-3/state-4 dump loop; this is the sole owner of drain-quantum and cadence behavior.
- `StorageClass::FindFirstNonEmptySlot/GetAmount/RemoveAmount @ 0x006C9820/0x006C9680/0x006C96B0` are the only storage-mutating calls in this slice; there is no separate "remove one bale" helper anywhere in this path.
- `RulesClass::ReadGeneral @ 0x00670CD4` is the sole INI-to-`+0x1528` writer for `HarvesterDumpRate`.
- Confirms (does not contradict) prior sibling reports `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_STATE3_STATE4_TIMING_GHIDRA_REPORT.md` and `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`, both of which independently reached the same whole-slot/14.4-frame/terminating-pulse conclusions from decompile; this report adds direct assembly/immediate-value re-verification (`disassemble_function`, `read_memory`) rather than relying on their decompile citations.

## 6. Current Rust Implementation Status

`src/sim/miner/miner_dock_sequence.rs::phase_unloading` (read this session, lines 1165-1294) already implements whole-slot drain per gate: it groups cargo by `ResourceType` (`SLOT_ORDER = [Ore, Gem]`), drains one resource type's full amount per threshold crossing, resets `unload_accumulator` only on a real drain (line 1282), and on the first empty-slot crossing transitions directly to `RefineryDockPhase::Departing` (line 1293) without seeding an extra cooldown — this matches the binary's "terminating pulse writes state 4 directly" behavior. `phase_deposit_cooldown` (line 1299) is a retired pass-through kept only for legacy saves.

One scope note, not a demonstrated bug: Rust's `SLOT_ORDER` covers 2 of the engine's 4 `StorageClass` indices (Ore, Gem). Stock YR ore-field growth/spawn rules do not populate Vinifera/Aboreus (TS-legacy Tiberium types), so this is not a currently-reachable parity gap in stock skirmish/campaign play; flagged here only for completeness since the binary's loop bound is 4, not 2.

## 7. Coverage Ledger

| Area / claim | Status | Evidence | What remains |
|---|---|---|---|
| Drain quantum = whole `StorageClass` slot, not bale | verified | `disassemble_function(0x0073D630)` `0x0073E3BF..0x0073E457`; `decompile_function(0x006C9820/0x006C9680/0x006C96B0)` | none |
| Gate cadence = `HarvesterDumpRate * 900.0` frames | verified | `disassemble_function(0x0073D630)` `0x0073E355..0x0073E374`; `read_memory(0x007e27f8)` = `900.0` | none |
| `HarvesterDumpRate` = `RulesClass+0x1528` | verified | `search_strings("HarvesterDumpRate")` -> `0x0083BE4C`; `get_xrefs_to` -> `0x00670CD4`; `get_assembly_context(0x00670cd4)` shows `FSTP double ptr [ESI+0x1528]` at `0x00670CE1` | none |
| Stock default `0.016` (14.4 frames) | verified for INI-absence; default value not traced to its constructor-site literal this session | grep `ini/rulesmd.ini`/`ini/rules.ini` for `HarvesterDumpRate` = no hits; `ReadGeneral` pre-loads `[ESI+0x1528]`/`[ESI+0x152C]` before the INI-override call (`0x00670CC0..0x00670CC6`), consistent with a hardcoded binary default | literal `0.016` immediate not located at its initializer this session (peripheral to scope) |
| Terminating pulse exists, credits nothing, writes state 4 | verified | `0x0073E4DC..0x0073E539`; no `Add_Tiberium_Credits`/`Add_Tiberium_To_Storage` call in that block | none |
| State 4 does not execute in the same call as the terminating write | verified | state-3 case direct-returns `1` at `0x0073E5B4`; state-4 dispatch arm (`0x0073E0A8`) is a separate switch case reached only on the *next* call | none |
| `+0x6D1` set/clear timing | verified | set `0x0073DFDA`; clear `0x0073E0D2`/`0x0073E1F6`; wait guard `0x0073E1DF..0x0073E1EA` | none |
| Only first non-empty slot drains per crossing (never two in one pulse) | verified | `FindFirstNonEmptySlot` returns on first `>0.0f` match, `0x006C9820` | none |
| Current Rust matches this model | verified by source read | `src/sim/miner/miner_dock_sequence.rs:1165-1294` | none for this slice |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Bale, whole-slot, or dump-attempt granularity? -> Whole StorageClass slot; RemoveAmount is always called with the exact value GetAmount just read for that slot.` (evidence: `0x0073E3BF..0x0073E457`)
- `[RESOLVED] OQ-2 - Exact cadence? -> HarvesterDumpRate(0.016 default) * 900.0 = 14.4 frames per gate evaluation, re-armed to 0 only after a real drain.` (evidence: `0x0073E355..0x0073E374`, `read_memory(0x007e27f8)`)
- `[RESOLVED] OQ-3 - Is there a terminating empty pulse? -> Yes; the first gate crossing with no non-empty slot credits nothing and writes state 4; state 4 itself runs on the next mission call.` (evidence: `0x0073E4DC..0x0073E539`, state-4 dispatch arm at `0x0073E0A8`)
- `[RESOLVED] OQ-4 - Does one pulse ever drain more than one resource type? -> No; FindFirstNonEmptySlot stops at the first non-empty index.` (evidence: `0x006C9820` decompile)
- `[DEFERRED] OQ-5 - Literal 0.016 default constructor site for RulesClass+0x1528. (category: out-of-scope; reason: peripheral to the drain-quantum/cadence/terminating-pulse question, already load-bearing-confirmed via INI absence + pre-load-then-override pattern; next-step-if-pursued: locate RulesClass constructor/default-field-initializer table.)`

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| One 14.4-frame gate crossing drains one whole resource-type slot to zero, not a bale-sized increment | `0x0073E3BF..0x0073E457` | none: already implemented | `src/sim/miner/miner_dock_sequence.rs::phase_unloading` | keep full-slot-per-crossing drain; do not switch to per-bale decrement | `cmin_pure_ore_load_drains_in_one_gate_crossing`: 20-bale pure-ore CMIN empties cargo on the first due gate, not over 20 gates | Do not reintroduce a fixed bale-per-tick decrement anywhere in this phase |
| The first empty-cargo gate crossing is a real, separately-timed pulse that credits nothing and transitions state | `0x0073E4DC..0x0073E539` | none: already implemented (`deposit_cooldown_ticks`/legacy phase retired) | `phase_unloading`, `phase_departing` | keep the empty crossing as its own 14.4-frame-gated event, not a same-tick fallthrough after the last real drain | `cmin_last_real_slot_drain_requires_one_more_empty_gate_before_departing`: full-ore CMIN stays `Unloading` for one more 14.4-frame gate after its single real drain before reaching `Departing` | Do not collapse the terminating pulse onto the same tick as the last real drain |

## 10. Negative Facts / Do Not Do

- Do NOT model refinery unload drain as "one bale per pulse." `RemoveAmount` always receives the full amount `GetAmount` just read for that slot; a 20-bale pure-ore load drains in exactly one 14.4-frame pulse, not 20.
- Do NOT treat "per-bale" wording in INI comments (`rulesmd.ini:30380`, "Value = credit value per 'bail'") as a runtime drain-quantum claim — that is a credit-conversion constant for a different calculation, not the `StorageClass` removal granularity.
- Do NOT credit anything on the terminating empty-slot gate crossing. No `HouseClass__Add_Tiberium_Credits`/`Add_Tiberium_To_Storage` call exists in the `0x0073E4DC..0x0073E539` block.
- Do NOT execute state-4 cleanup logic on the same `Mission_Deploy_Building` call that writes `+0xBC=4`. The write is followed by a direct `return 1`; state 4 only runs on the next call.
- Do NOT drain two resource-type slots in a single gate crossing, even when cargo has both Ore and Gems. `FindFirstNonEmptySlot` returns only the first non-empty index per call.

## 11. Remaining Uncertainty

- The literal `0.016` default value for `HarvesterDumpRate` was not traced to its exact constructor/initializer instruction this session (the `ReadGeneral` call site only shows a pre-load-then-INI-override pattern, consistent with but not a direct read of the hardcoded default). Peripheral to this report's scope; the effective value is not in doubt (INI absence confirmed by grep, and the 14.4-frame figure is independently corroborated by two prior sibling Ghidra reports).
- The per-frame increment mechanism that advances `UnitClass+0xF8` toward the 14.4-frame threshold was not traced to its source this session — it is not incremented anywhere inside `Mission_Deploy_Building` itself (only reset-to-zero and read-for-compare appear in this function), so it must be driven by a generic per-object periodic-timer tick outside this function's boundary. This does not affect the drain-quantum or terminating-pulse findings, which are independent of exactly how the accumulator advances.

## 12. Stale Docs / Follow-up Docs

- `docs/research/miner/HARVESTER_DOCK_UNLOAD.md`: lines ~157-169 still frame the dump loop as "minutes per bale" / "14.4 frames per bale" / "subtract one bale from cargo" / "When all bales dumped." Replace with: "HarvesterDumpRate is frames per whole-StorageClass-slot dump gate (14.4 frames stock default); each gate drains one entire resource-type slot to zero via `RemoveAmount(GetAmount(slot), slot)`, not a fixed bale amount; the terminating gate (FindFirstNonEmptySlot == -1) credits nothing and writes mission substate 4."
- `docs/research/miner/HARVESTER_DOCK_UNLOAD_SEQUENCE.md`: line 807 table entry `"0x1528 | HarvesterDumpRate | Minutes per bale during unloading (double)"` is stale. Replace with: "Frames per whole-slot dump gate (0.016 default x 900.0 = 14.4 frames); drains an entire StorageClass resource-type slot per crossing, not a bale."
- Not stale (already corrected 2026-07-10, consistent with this report): `docs/research/miner/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md` (line 123, 304), `docs/research/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md` (lines 34, 518, 755), `docs/research/miner/traces/CHRONO_MINER_PAD_PIVOT_TO_EAST_TRACE.md` ("Correction 2026-07-12" section) — all already state whole-slot-per-crossing draining; this report independently re-confirms their conclusion via live assembly rather than superseding it.
- Sibling reports `MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_STATE3_STATE4_TIMING_GHIDRA_REPORT.md` and `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md` are corroborated, not contradicted, by this report.

## Sources

- Ghidra `decompile_function(0x0073D630)` - `UnitClass::Mission_Deploy_Building`, this session.
- Ghidra `disassemble_function(0x0073D630)` - full-function assembly, this session (all cited addresses read directly from this output).
- Ghidra `decompile_function(0x006C9820)` - `StorageClass::FindFirstNonEmptySlot`.
- Ghidra `decompile_function(0x006C9680)` - `StorageClass::GetAmount`.
- Ghidra `decompile_function(0x006C96B0)` - `StorageClass::RemoveAmount`.
- Ghidra `read_memory(0x007e27f8, 8)` - confirmed IEEE-754 double `900.0`.
- Ghidra `read_memory(0x007e1748, 4)` - confirmed IEEE-754 single `0.0f`.
- Ghidra `search_strings("HarvesterDumpRate")` -> `0x0083BE4C`; `get_xrefs_to(0x0083be4c)` -> caller `0x00670CD4`; `get_assembly_context(xref_sources=0x00670cd4)` - confirms `FSTP double ptr [ESI+0x1528]` at `0x00670CE1`.
- INI: `ini/rulesmd.ini` (grep for `HarvesterDumpRate`, `PurifierBonus`, `[Tiberiums]`, `[CMIN]`, `[HARV]`, `[GAREFN]`, `[NAREFN]`), `ini/rules.ini`, `ini/artmd.ini`.
- Rust: `src/sim/miner/miner_dock_sequence.rs:1165-1304` (read this session).
- Prior docs read/reconciled: `docs/research/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_STATE3_STATE4_TIMING_GHIDRA_REPORT.md`, `docs/research/miner/STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`, `docs/research/miner/STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`, `docs/research/miner/traces/CHRONO_MINER_PAD_PIVOT_TO_EAST_TRACE.md`, `docs/research/miner/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md`, `docs/research/miner/MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md`, `docs/research/miner/HARVESTER_DOCK_UNLOAD.md`, `docs/research/miner/HARVESTER_DOCK_UNLOAD_SEQUENCE.md`.
