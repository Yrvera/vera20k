# FUN_006AF6C0 — Identity Correction and Full Decompilation Report

## CRITICAL FINDING: Address Mismatch

**0x006AF6C0 is NOT a refinery dock-queue processor.**

It is `SlaveManagerClass::AI_Update` — the per-frame state machine for slaves
owned by Yuri's Slave Miner unit. This was already flagged in
`BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` §"CRITICAL FINDING" (2026-05-11
audit), but the swarm task pre-brief was written before that correction
propagated. This report fully documents the function as it actually is.

Confidence: HIGH — verified from `get_function_by_address(0x6AF6C0)` →
`SlaveManagerClass__AI_Update`, function body 0x006AF6C0–0x006AFD3E, Ghidra
2026-05-19 session.

**2026-05-24 reswarm status:** This identity correction remains current. Do not
use this file as evidence for stock `CMIN/HARV -> GAREFN/NAREFN` refinery dock
admission, queue promotion, `0x15` handoff, or release timing. The current stock
refinery model is
`STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`.

---

## Part 1 — Address and Identity

| Item | Value | Evidence |
|------|-------|----------|
| Confirmed address | 0x006AF6C0 | `get_function_by_address` |
| Confirmed name | `SlaveManagerClass__AI_Update` | Ghidra RTTI label |
| Function size | 0x67E bytes (1662 bytes) | Body 006AF6C0–006AFD3E |
| Called by | `FUN_006AF5F0` (tick throttle) | `get_function_callers` |
| Tick throttle caller | `UnitClass__AI` (0x7360C0) via vtable+0x5C | disassembly verified |

Active in YR: **Yes** — called every time a Slave Miner unit ticks and has
live slaves. Slave Miners (e.g. YMSLA) are a standard YR Soviet/Yuri unit.

---

## Part 2 — Caller Chain

```
UnitClass__AI (0x7360C0)
  → if (param_1[0x9e] != 0)        // unit has SlaveManager
      (*vtable[0x5C])()             // dispatch = 0x7F31C8 + 0x5C = 0x7F3224
                                    // → FUN_006AF5F0 (tick throttle)
        → if (frame_timer elapsed)
            SlaveManagerClass__AI_Update (0x6AF6C0)
            UnitClass__Mission_Deploy()  // also called here
```

`FUN_006AF5F0` (0x006AF5F0) is a tick-rate throttle:
- Reads timer from `slaveManager+0x50` (start frame) and `+0x58` (interval = 10).
- Only calls AI_Update when `currentFrame - startFrame >= 10`.
- Resets `+0x50 = currentFrame`, `+0x58 = 10` each tick it fires.

Vtable dispatch verified: `SlaveManagerClass` primary vtable base = **0x7F31C8**
(from constructor disassembly `MOV dword ptr [ESI],0x7f31c8`). Slot 0x5C/4=23
= address 0x7F3224, bytes `F0 F5 6A 00` = 0x6AF5F0. ✓ HIGH.

---

## Part 3 — SlaveManagerClass Struct Layout

All offsets verified from constructor disassembly (0x6AF1A0 / 0x6AF4A0) and
AI_Update decompilation. param_1 type is `int *` — array indexing multiplies by 4.

| Offset | Field | Type | Notes |
|--------|-------|------|-------|
| +0x00 | vtable | ptr | = 0x7F31C8 (primary) |
| +0x04 | secondary_vtable_1 | ptr | = 0x7F31AC |
| +0x08 | secondary_vtable_2 | ptr | = 0x7F31A4 |
| +0x0C | secondary_vtable_3 | ptr | = 0x7F319C |
| +0x24 | master_ptr | int* | Slave miner (master unit) pointer |
| +0x28 | slave_type_ptr | int* | SlaveType object pointer |
| +0x2C | slave_count | int | Number of slaves to manage |
| +0x30 | master_deposit_delay | int | Timer duration after deposit (state 4→5) |
| +0x34 | respawn_delay | int | Timer duration for state 5 respawn wait |
| +0x38 | array_sub_obj | — | Dynamic array sub-object (20 bytes) |
| +0x3C | slot_array_ptr | int** | Pointer to array of slave-entry pointers |
| +0x40 | array_capacity | int | Allocated capacity |
| +0x44 | ? | | |
| +0x45 | can_grow | byte | Array can reallocate |
| +0x48 | active_slave_count | int | Number of active entries iterated |
| +0x4C | max_capacity | int | Default 10 |
| +0x50 | tick_timer_start | int | g_CurrentFrameCounter at last tick |
| +0x54 | tick_timer_unknown | int | CDTimer second field |
| +0x58 | tick_interval | int | Default 10 (fires every 10 frames) |
| +0x5C | 0 | int | Init zero |
| +0x60 | 0 | int | Init zero |

---

## Part 4 — Slave Entry Layout

Each slave entry is a 20-byte heap allocation (`operator_new(0x14)` = 5 ints):

| Index | Byte Offset | Field | Notes |
|-------|-------------|-------|-------|
| [0] | +0x00 | slave_unit_ptr | Pointer to the slave InfantryClass/UnitClass |
| [1] | +0x04 | state | Current state (0–6) |
| [2] | +0x08 | timer_start | g_CurrentFrameCounter at timer start |
| [3] | +0x0C | timer_unknown | CDTimer second field (not used in comparisons) |
| [4] | +0x10 | timer_duration | Remaining frames (used for state 5/6 waits) |

The slot_array at `slaveManager+0x3C` is an array of `slave_entry*` pointers.
`piVar1 = *(int **)(*(int *)(param_1 + 0x3c) + local_7c * 4)` — each element
is a pointer-to-entry; the array is indexed by slave index 0..active_slave_count-1.

---

## Part 5 — State Machine (States 0–6)

### Null-check preamble (before switch)

```c
piVar2 = (int *)*piVar1;  // slave unit ptr
if ((piVar2 == NULL) && (piVar1[1] != 6)) {
    *piVar1 = 0;          // clear unit ptr (already null)
    piVar1[1] = 6;        // transition to cleanup state
    piVar1[2] = g_CurrentFrameCounter;
    piVar1[4] = *(int *)(param_1 + 0x30);  // deposit_delay as cleanup timer
}
```

If slave unit pointer is null and state is not already 6, force to state 6 (cleanup).

### State 0: Idle / Uninitialized

Not reached via the switch directly — the constructor sets entries to state 0
(entry[1]=0, entry[4]=0). State 0 is the starting state before the first
dispatch. The switch has no case 0, so it falls through on first tick. States
1 and 4 transition back to state 0; state 5 transitions to state 0 when timer
expires (slave respawn complete).

### State 1: Search for Ore Cell

Uses `g_RulesClass_Instance+0x1784` as scan radius (SlaveMinerSlaveScan INI key).
- Calls vtable+0x338 on the slave type to find nearest ore cell.
- If found cell matches `DAT_00b0b5b8/ba` (invalid cell sentinel): no ore cell
  was found by the scan — transition to **state 4** (return to master deposit).
  The slave's physical position is **not** checked here (2026-05-24 audit correction).
- Otherwise: sets ghost cell, issues Move command (`vtable+0x480` with cell,
  `vtable+0x1e8(2,0)`) → transition to **state 2**.

### State 2: Walking to Ore Cell

- Calls `vtable+0x1bc()` each tick (get current cell).
- Calls `FUN_00487DF0(piVar2)` — checks `slave+0xEC == 5` (mission == Harvest?).
  - If false and `slave+0x5A4 == 0` (no NavCom destination assigned): loop back to **state 1**.
  - If true: calls `FUN_00522D00` (sets mission=10 if not already) → **state 3**.

`FUN_00487DF0` verified: returns `(*(int*)(param_1 + 0xEC) == 5)`.
`FUN_00522D00` verified: `if (slave->mission != 10) slave->SetMission(10, 0)`.

### State 3: Harvesting at Ore Cell

- Calls `FUN_00522D30(piVar2)` — checks `slave->GetStorageLevel() >= 1.0`
  (slave cargo full check via vtable+0x2B4).
  - If **not full**: calls `vtable+0x1bc()`, calls `FUN_00487DF0`:
    - If not on Harvest mission and no path: calls `FUN_00522D20` (SetMission(5))
      → **state 1**.
    - Else: calls `FUN_00522FC0` — checks `slave->GetMission() == 10` (**Harvest**, not Guard; 2026-05-24 audit).
      If on Harvest: **state 1**.
  - If **full** (cargo complete): get slave's current cell, set ghost cell to
    master center, issue Move command → **state 4**.

`FUN_00522D30` verified: returns `slave->vtable+0x2B4() >= 1.0f`.
`FUN_00522D20` verified: `slave->SetMission(5, 0)` (Mission::Guard = 5).
`FUN_00522FC0` verified: returns `slave->GetMission() == 10`.

### State 4: Returning to Master (Deposit)

Each tick:
1. Check if slave has arrived at master cell (within `g_RulesClass_Instance+0xDF8`
   distance, using `FUN_006B1A70` — cell Pythagorean distance).
2. Also checks `slave+0x5A4` (NavCom destination pointer; non-zero = slave currently has a movement target).
3. **Arrival + no NavCom destination** (`piVar2[0x169] == 0`):
   - Calls `BuildingClass__DepositOreFromStorage(*(param_1+0x24))`.
   - Calls `slave->vtable+0xD4()` (Limbo — put slave in limbo/hidden state).
   - Timer → **state 5** (`piVar1[1]=5`, `piVar1[2]=currentFrame`,
     `piVar1[4]=*(int*)(param_1+0x34)` = respawn delay).
4. **Arrival + still has NavCom destination** (`piVar2[0x169] != 0`):
   - If within distance → **state 4 continues** (waiting for destination to clear).
   - If far away → re-issue Move command to master center.
5. **Not at master cell + has NavCom destination**: re-issue Move command to master center.

`BuildingClass__DepositOreFromStorage` at 0x00522D50 — confirmed callee.
`piVar2[0x169]` = `slave+0x5A4` — the **NavCom destination target pointer**, written by `FootClass::Set_Destination_Internal @ 0x004D94B0`. Verified via `decompile_function 0x004D94B0` (Ghidra's own comment: "Sets FootClass+0x5A4 (NavCom destination target)") plus `decompile_function 0x006AF6C0` showing every state-2/state-4 read of `piVar2[0x169]` gates on whether the slave currently has a destination — not on cargo or path-steps (2026-05-24 audit resolution). Consistent with `RADIO_SYSTEM_MODEL_SYNTHESIS.md` and `miner/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` framing.
`g_RulesClass_Instance+0xDF8` — distance threshold (verified reference in
AI_Update at `if (*(int *)(g_RulesClass_Instance + 0xdf8) < (int)sVar4)`).

### State 5: Post-Deposit Respawn Delay

CDTimer countdown using `piVar1[2]` (start), `piVar1[4]` (duration):
```c
remaining = piVar1[4];
if (piVar1[2] != -1) {
    elapsed = g_CurrentFrameCounter - piVar1[2];
    if (elapsed < remaining) { remaining -= elapsed; break; }  // still waiting
}
if (remaining != 0) break;  // timer still active
// timer expired: respawn slave
piVar2[0x1b] = *(int *)(piVar2[0x1b0] + 0xa0);  // restore health from Type
piVar2[0x1c] = *(int *)(piVar2[0x1b0] + 0xa0);  // same
piVar1[1] = 0;  // reset to state 0
```

When timer expires, slave health fields are reset from `slave->Type+0xA0` and
state returns to **0** (restart the search cycle).

### State 6: Cleanup (Dead/Null Slave)

Identical CDTimer countdown to state 5. When timer expires:
```c
FUN_006AF650(piVar1);  // respawn or remove slave entry
```

`FUN_006AF650` (0x006AF650): Gets slave type's TypeClass vtable, calls
`vtable+0x8C` (spawn/create new slave unit), links new unit to master via
`unit+0x2DC = master_ptr`, resets entry to state 0 with timer 0.

---

## Part 6 — State Transition Summary

```
NULL ptr detected → 6
0  → 1 (first tick, no switch case)
1  → 2 (ore found, issue move)
1  → 4 (already at master center, no ore)
2  → 1 (arrived but no Harvest mission, no path)
2  → 3 (Harvest mission active = FUN_00487DF0 true)
3  → 1 (not full, not on Harvest)
3  → 4 (slave cargo full, return to master)
4  → 5 (arrived at master, cargo empty, DepositOreFromStorage called)
4  → 4 (re-issue move if not yet arrived)
5  → 0 (respawn delay expired, slave health reset)
6  → 0 (dead-slave cleanup timer expired, FUN_006AF650 spawns new slave)
```

---

## Part 7 — What the Swarm Brief Asked About (Resolution)

**(a) Confirm 0x6AF6C0 is the target:**
Confirmed present — but **identity is wrong**. It is `SlaveManagerClass__AI_Update`,
not a refinery dock-queue processor.

**(b) Caller chain:**
`UnitClass__AI` (0x7360C0) → via vtable dispatch at `slaveManager_ptr + 0x5C`
→ `FUN_006AF5F0` (throttle, every 10 frames) → `SlaveManagerClass__AI_Update`.

**(c) DockManager field on BuildingClass:**
Does not apply. The SlaveManager is on the **slave miner unit** at `unit+0x278`
(= `param_1[0x9E]` in UnitClass::AI). `BuildingClass+0x2E4` is the standard
single-slot dock reservation used by harvesters — unrelated.

**(d) State values:**
Valid states: **0, 1, 2, 3, 4, 5, 6**. Full meanings documented above.
State 4→5 calls `DepositOreFromStorage` — this matches the brief's claim, but
the "deposit" is slave ore into the slave miner master, not harvester → refinery.

**(e) Per-entry layout:**
entry[0]=slave_ptr, entry[1]=state, entry[2]=timer_start,
entry[3]=timer_unknown, entry[4]=timer_duration. Total 20 bytes.

**(f) Chrono miner branch:**
No chrono miner branch exists here. Slave Miners are not Teleporter units.
The entire function is Slave Miner only.

**(g) Timer/duration values:**
State 4→5 timer: `*(int*)(param_1+0x34)` — from SlaveManagerClass instance
(set at construction from INI). State 5/6 timers: same field.
`g_RulesClass_Instance+0xDF8` — approach proximity threshold.
Tick rate: every 10 frames (`+0x58 = 10`).

---

## Part 8 — What IS the Refinery Dock Processor?

There is no separate refinery dock-queue object. The correct pipeline (from
BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md §CRITICAL FINDING, verified):

1. `UnitClass::Mission_Enter` (0x739EC0) — drives harvester to refinery.
2. `UnitClass::Mission_Deploy_Building` (0x73D630) — per-bale dump loop.
3. `FUN_004595C0` (0x4595C0) — refinery per-frame visual update + harvester
   eject helper, called from Mission_Deploy_Building.
4. `BuildingClass::UndockUnit` (0x4593A0) — destruction-time eject only.
5. `BuildingClass::DepositOreFromStorage` (0x522D50) — also called here, for
   same-deposit-call-site reason that caused the original misattribution.

---

## Key Verified Facts

1. **0x6AF6C0 = SlaveManagerClass::AI_Update** (not refinery dock queue).
   Evidence: `get_function_by_address(0x6AF6C0)` → label `SlaveManagerClass__AI_Update`.

2. **Vtable base = 0x7F31C8**, slot 0x5C = 0x7F3224 = FUN_006AF5F0 (tick throttle).
   Evidence: constructor disassembly `MOV [ESI],0x7f31c8`; `read_memory(0x7F3224)` → `F0 F5 6A 00`.

3. **Slave entry layout: 20 bytes, 5 ints** — entry[0]=unit_ptr, entry[1]=state (0–6),
   entry[2]=timer_start, entry[3]=timer_unknown, entry[4]=timer_duration.
   Evidence: `operator_new(0x14)` in constructor; field access pattern in AI_Update.

4. **State 4→5 calls `BuildingClass__DepositOreFromStorage` (0x522D50)** — but it
   deposits slave ore into the slave miner master, not into a refinery.
   Evidence: `get_function_callees(0x6AF6C0)` → `BuildingClass__DepositOreFromStorage @ 00522d50`.

5. **SlaveManager on unit at offset 0x278** (`param_1[0x9E]` in UnitClass::AI), tick fires
   every 10 frames via `slaveManager+0x58` interval field.
   Evidence: `UnitClass__AI` decompilation: `if (param_1[0x9e] != 0) { (**(code **)(*(int *)param_1[0x9e] + 0x5c))(); }`.

---

## Status

**COMPLETE** — with scope correction. The function at 0x6AF6C0 has been fully
decompiled and documented. All seven questions from the swarm brief are answered.
The underlying investigation target (refinery dock queue state machine) does not
exist as a standalone function; the correct refinery dock pipeline is documented
in BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md.
