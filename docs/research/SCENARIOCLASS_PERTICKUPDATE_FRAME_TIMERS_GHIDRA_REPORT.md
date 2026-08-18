# ScenarioClass PerTickUpdate Frame-Timer Fields — Ghidra Research Report

**Date:** 2026-05-28  
**Target:** Five `ScenarioClass` frame-timer offsets accessed by `LogicClass::PerTickUpdate @ 0x0055AFB0`  
**Investigation Mode:** `/re-swarm` slot 4, read-only Ghidra  
**Active in YR:** Per-field (see table below)  
**Confidence:** HIGH for byte offsets, field roles, and YR-liveness of fields 1 and 5; HIGH for TS-dormant verdict on fields 3 and 4; MEDIUM for field 2 liveness pending `Rules+0x17F0` INI-key identity confirmation

---

## Investigation Contract

**Target question:** For each of five `ScenarioClass` offsets (`inst[0x47a]`, `inst[0x486]`, `inst[0x489]`, `inst[0x492]`, `inst[0xd4b]`) accessed in `LogicClass::PerTickUpdate @ 0x0055AFB0`: what field is it, what unit, what writes/reads it, what is its cadence and purpose, and is it Active in YR?

**Non-goals:** Do not re-derive lightning storm internals (covered by `LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md`); do not re-derive tiberium growth drivers (covered by `TIBERIUMCLASS_GROWTH_SPREAD_DRIVER_TIMERS_GHIDRA_REPORT.md`); do not implement Rust code; do not rename/create Ghidra annotations.

**Evidence needed to mark COMPLETE:** Live Ghidra decompile of `0x0055AFB0`; assembly context confirming byte offsets for each field; liveness ruling citing gating flag and its YR default; callee identity for each timer's action.

**Stop conditions:** Stop at one field-by-field table with evidence ranges and Active-in-YR verdicts. Do not exhaustively trace all callees unless identity is needed for liveness judgment.

---

## 0. PARAM-ARITHMETIC DETERMINATION

`g_ScenarioClass_Instance` is typed as `uint *` (pointer to `uint`) in Ghidra's decompile of `LogicClassPerTickUpdateLiveVector`. Therefore all `inst[N]` accesses mean **byte offset = N × 4**.

Verified: decompile shows `g_ScenarioClass_Instance[0x47a]` for the first timer; assembly at `0x0055B17D` shows `MOV EAX, dword ptr [EDI + 0x11e8]` where `0x11e8 = 0x47a × 4`. This confirms the `int *` interpretation.

**All byte offsets below are `index × 4`.**

---

## 1. Five-Field Summary Table

| Decompile index | Byte offset | Assembly citation | Field role | Unit | -1 sentinel | Active in YR | Gate |
|---|---|---|---|---|---|---|---|
| `inst[0x47a]` | `0x11E8` | `0x0055B17D`: `MOV EAX,[EDI+0x11E8]` | Scenario cell-action timer **start frame** | frames (`g_CurrentFrameCounter`) | Yes (`0xffffffff`) | **YES** | unconditional in cell-action loop |
| `inst[0x486]` | `0x1218` | `0x0055B234`: `MOV EDX,[EBP+0x1218]` | Timed-driver start frame for `FUN_004ACAC0` (vein/TS cell-update) | frames | Yes | **Conditional / likely TS-dormant** | `Rules+0x17F0 (bool) != 0` AND `Rules+0x1640 (double) != 0.0` |
| `inst[0x489]` | `0x1224` | `0x0055B2DF`: `MOV EDX,[EBP+0x1224]` | Timed-driver start frame for `FUN_004ACBC0` (fog/TS cell-update) | frames | Yes | **NO — TS-dormant** | `SpecialFlags bit 0x1000 != 0` AND `Rules+0x1648 (double) != 0.0` |
| `inst[0x492]` | `0x1248` | `0x0055B368`: `MOV ECX,[EBP+0x1248]` | Timed-driver start frame for `FUN_004AE4C0` (cell Z-recalc) | frames | Yes | **Conditional** (live with lightning storm) | `Scenario[0xd4b] != Scenario[0xd4c]` AND `Rules+0x1668 (double) != 0.0` |
| `inst[0xd4b]` | `0x352C` | `0x0055B343`: `MOV ECX,[EBP+0x352c]`; `0x0055B470`: `MOV [EDX+0x352c],ECX` | Ambient-transition "current step" counter | integer counter | No | **Conditional** (same as `inst[0x492]`) | compared with `inst[0xd4c]` (`0x3530`) |

---

## 2. Field-by-Field Detail

### Field 1 — `inst[0x47a]` = byte offset `0x11E8`

**Role:** Scenario cell-action timer **start frame**. Holds the `g_CurrentFrameCounter` value at which the current cell-action timer began. Value `0xffffffff` = inactive/never started.

**Companion field:** `inst[0x47c]` = byte offset `0x11F0` = timer **duration** (frames remaining). Read in same timer-test block.

**Timer logic (decompile `0x0055AFB0`):**
```
elapsed = g_CurrentFrameCounter - inst[0x47a]
if elapsed < inst[0x47c]:  // timer still running
    remaining = inst[0x47c] - elapsed
else:
    ProcessCellAction(0x0e, ...)  // timer expired → fire action 0x0e
```

After the main loop, the same timer is re-checked outside the loop (addresses `0x0055B17D..0x0055B1D5`) for expiry handling: on expiry, `inst[0x47a]` is set to `0xffffffff` and `FUN_004F42F0(2)` is called.

**Unit:** frames (direct `g_CurrentFrameCounter` comparison, verified from decompile).

**Writes:** `PerTickUpdate` itself resets `inst[0x47a] = 0xffffffff` on expiry at `0x0055B1BF`. The start value is set by the cell-action dispatch callers (e.g., `TechnoClass__ProcessCellAction` / `FUN_006E53A0`) which set the timer when a timed action begins.

**Active in YR:** **YES** — this block runs unconditionally whenever `DAT_008B40D8 > 0` (scenario cell action count). Standard YR maps regularly set cell actions. Evidence: decompile of `0x0055AFB0`; assembly `0x0055B17D`.

---

### Field 2 — `inst[0x486]` = byte offset `0x1218`

**Role:** Timed-driver start frame for calling `FUN_004ACAC0`. Value `0xffffffff` = timer not yet started.

**Companion fields:** `inst[0x487]` = `0x121C` (middle-word/scratch); `inst[0x488]` = `0x1220` (timer duration/interval).

**Timer logic (addresses `0x0055B228..0x0055B28E`):**
```
if Rules+0x17F0 != 0 AND Rules+0x1640 != 0.0:
    elapsed = g_CurrentFrameCounter - inst[0x486]
    if elapsed < inst[0x488]:  // still running
        ...
    else:
        ftol(…)                 // reload interval from float
        inst[0x486] = g_CurrentFrameCounter  // reset start frame
        FUN_004ACAC0()
```

**What `FUN_004ACAC0` does:** Decompiled at `0x004ACAC0`. Full-map double-pass scan (512×512 cells): first pass flags cells where `CellFlags & 0x08 != 0 AND CellFlags & 0x10 == 0` (sets bit `0x20`); second pass fires `FUN_004ACDA0` on those cells and calls `FUN_004ADFF0(0,0)`. This is a Tiberian Sun **vein-growth/tiberium-flow precursor** cell update. The cell bits `0x08` and `0x10` are TS-era vein/tiberium overlay flags not used in standard YR maps.

**Gates:**
- `Rules+0x17F0` is a `bool` field at byte offset 0x17F0 in `RulesClass`. Not identified from ReadGeneral traversal in this session (ReadGeneral body spans `0x0066D530..0x00671E98` and the offset 0x17F0 is within that range but valid instruction boundaries not found at the specific scan points). Based on proximity to Lightning Storm fields (`0x17B0..0x17B4..0x17B8..0x17BC..0x17C0..0x17C4..0x17C8`) and TS context, this is likely a TS vein/weather enable flag with YR default **false**. **UNVERIFIED INI key identity — see Remaining Uncertainty.**
- `Rules+0x1640` is a `double` field at byte offset 0x1640. Not found as an FSTP target in ReadGeneral scan. May be from constructor default or a different reader. **UNVERIFIED identity — see Remaining Uncertainty.**

**Active in YR:** **Conditional / likely TS-dormant.** Both gates must be non-zero, and standard YR maps do not have TS vein cells. Evidence: decompile `0x0055AFB0`; assembly `0x0055B20B` (`MOV AL,[EBX+0x17f0]`), `0x0055B215` (`FLD [EBX+0x1640]`), `0x0055B234` (`MOV EDX,[EBP+0x1218]`).

---

### Field 3 — `inst[0x489]` = byte offset `0x1224`

**Role:** Timed-driver start frame for calling `FUN_004ACBC0`. Value `0xffffffff` = timer not yet started.

**Companion fields:** `inst[0x48a]` = `0x1228` (middle-word); `inst[0x48b]` = `0x122C` (interval).

**Timer logic (addresses `0x0055B2C4..0x0055B33D`):**
```
if (ScenarioClass[0] & 0x1000) != 0 AND Rules+0x1648 != 0.0:
    elapsed = g_CurrentFrameCounter - inst[0x489]
    if elapsed < inst[0x48b]: ...
    else:
        ftol(...)
        inst[0x489] = g_CurrentFrameCounter
        FUN_004ACBC0()
```

**What `FUN_004ACBC0` does:** Decompiled at `0x004ACBC0`. Full-map cell iterator scan: flags cells where `CellFlags & 0x02 != 0 AND CellFlags & 0x01 == 0` (sets bit `0x40`); second pass fires `FUN_004ACC50` on those cells. This is a TS **fog-of-war / secondary tiberium** cell propagation update.

**Gates:**
- `ScenarioClass[0] & 0x1000` — the dword at `ScenarioClass+0` is `SpecialFlags`. Bit `0x1000` is the **Tiberian Sun fog-of-war flag**. Documented in `OBSERVER_SPECTATOR_FOG_GHIDRA_REPORT.md` and `CLAUDE.md`: *"standard YR fog-of-war is normally off unless this special flag is set"*. YR default = **false/0**. Assembly: `0x0055B2C4 MOV EAX,[EBP]` (reads `ScenarioClass[0]`), `0x0055B2C7 TEST AH,0x10` (checks bit 12 of the dword via AH = byte 1).
- `Rules+0x1648` is a `double`. YR identity unverified but irrelevant since the outer gate is already 0 in standard YR.

**Active in YR:** **NO — TS-dormant.** `SpecialFlags & 0x1000` defaults to 0 in all standard YR skirmish/MP maps. The entire block is unreachable in normal play. Evidence: decompile `0x0055AFB0`; assembly `0x0055B2C4..0x0055B2C7` (SpecialFlags bit check); `CLAUDE.md` documented TS fog gate.

---

### Field 4 — `inst[0x492]` = byte offset `0x1248`

**Role:** Timed-driver start frame for calling `FUN_004AE4C0` (full-map cell Z-recalculation). Value `0xffffffff` = timer not started.

**Companion fields:** `inst[0x493]` = `0x124C`; `inst[0x494]` = `0x1250` (interval).

**Timer logic (addresses `0x0055B33D..0x0055B4D7`):**
```
if Scenario[0xd4c] == Scenario[0xd4b]: skip entire block
if Rules+0x1668 == 0.0: skip entire block
// Timer gate:
elapsed = g_CurrentFrameCounter - inst[0x492]
if elapsed < inst[0x494]: still running
// else: query LS phase state helpers, update Scenario[0xd4b], call FUN_004AE4C0, FUN_004F42F0(1)
```

**LS phase state helpers queried before acting:**
- `FUN_0053A110` at `0x0053A110`: returns `DAT_00a9fabc == 1` — lightning storm ambient **darkening phase 1** active.
- `FUN_0053A120` at `0x0053A120`: returns `DAT_00a9fabc == 2` — ambient **brightening-back phase 2** active.
- `FUN_0053BAD0` at `0x0053BAD0`: returns `DAT_00a9fab0 != 0`.
- `FUN_0053B400` at `0x0053B400`: returns `DAT_00a9fac0 != 0`.

`DAT_00a9fabc` is the **lightning storm ambient lighting transition phase counter** (0=idle, 1=darkening, 2=brightening), confirmed from `LightningStorm__Process @ 0x0053A6C0` decompile where it transitions 0→1→2→0.

**What `FUN_004AE4C0` does:** Decompiled at `0x004AE4C0`. Iterates all map cells via `MapClass__CellIterator_Next` and calls `Cell_ComputeZAdjust` on each. This recomputes each cell's visual Z-height/shadow offset — typically needed when ambient lighting or terrain transitions occur.

**Active in YR:** **Conditional.** The outer guard `Scenario[0xd4c] != Scenario[0xd4b]` gates on a transition being in progress. `Rules+0x1668` must be non-zero (likely the ambient speed/rate value; unverified INI key). The block is **reachable** during a lightning storm ambient transition (darkening/brightening phases 1 and 2 of `DAT_00a9fabc`). Since Lightning Storm IS a live YR superweapon, this block is live. Evidence: decompile `0x0055AFB0`; assembly `0x0055B33D MOV EAX,[EBP+0x3530]`, `0x0055B343 MOV ECX,[EBP+0x352c]`, `0x0055B368 MOV ECX,[EBP+0x1248]`; `LightningStorm__Process` decompile confirming `DAT_00a9fabc` lifecycle.

---

### Field 5 — `inst[0xd4b]` = byte offset `0x352C`

**Role:** Ambient-transition **current step counter** ("how far along" the current transition is). Compared with `inst[0xd4c]` = byte offset `0x3530` (the **target step counter**). When they match, the transition block skips entirely.

**Writes in PerTickUpdate:**
- Read: `0x0055B33D MOV EAX,[EBP+0x3530]` (reads `inst[0xd4c]`), `0x0055B343 MOV ECX,[EBP+0x352c]` (reads `inst[0xd4b]`). Compared at `0x0055B349 CMP EAX,ECX`. If equal → skip to `LAB_0055b4d7`.
- Write: `0x0055B43C MOV [ECX+0x3530],EAX` (writes `inst[0xd4c]`); `0x0055B470 MOV [EDX+0x352c],ECX` (writes `inst[0xd4b]`); `0x0055B490 MOV [EDX+0x352c],ECX`; `0x0055B4AE MOV [EDX+0x352c],EAX` (clamps to target). The block increments or decrements `inst[0xd4b]` toward `inst[0xd4c]` using `Math__ftol` of a rules-rate value, clamping at the target.

**Semantic meaning:** `inst[0xd4b]` = current ambient state value; `inst[0xd4c]` = target ambient state value. When a lightning storm starts/ends, `inst[0xd4c]` is updated to a new target, and each tick the `inst[0x492]` timer fires to step `inst[0xd4b]` toward `inst[0xd4c]` by `ftol(Rules+0x1668 rate)`. When they converge, the transition ends. This drives gradual ambient darkening/brightening of the map during lightning storms.

**Active in YR:** **Conditional** (same as `inst[0x492]`). Evidence: assembly `0x0055B33D..0x0055B4AE`; `LightningStorm::Process` decompile.

---

## 3. Assembly Evidence Table (inline citations)

| Claim | Ghidra MCP call / address | Evidence text |
|---|---|---|
| `g_ScenarioClass_Instance` pointer type is `uint *` (so index × 4 = byte offset) | `decompile_function 0x0055AFB0` | Decompile signature `uint *` for `g_ScenarioClass_Instance`; `inst[0x47a]` → asm `[EDI+0x11E8]` (0x47a × 4 = 0x11E8). |
| `inst[0x47a]` = byte offset `0x11E8`, frame timer start | `get_assembly_context 0x0055B17D` | `MOV EAX,dword ptr [EDI + 0x11e8]` |
| `inst[0x486]` = byte offset `0x1218` | `get_assembly_context 0x0055B234` | `MOV EDX,dword ptr [EBP + 0x1218]` |
| `inst[0x486]` gated by `Rules+0x17F0 bool` | `get_assembly_context 0x0055B20B` | `MOV AL,byte ptr [EBX + 0x17f0]; TEST AL,AL; JZ 0x0055b28e` |
| `inst[0x486]` gated by `Rules+0x1640 double != 0.0` | `get_assembly_context 0x0055B215` | `FLD double ptr [EBX + 0x1640]; FCOMP [0x007e2800]` |
| `inst[0x489]` = byte offset `0x1224` | `get_assembly_context 0x0055B2DF` | `MOV EDX,dword ptr [EBP + 0x1224]` |
| `inst[0x489]` gated by `SpecialFlags & 0x1000` | `get_assembly_context 0x0055B2BE` + decompile | `MOV EAX,[EBP]` (ScenarioClass[0]); `TEST AH,0x10` — checks bit 12 of SpecialFlags dword |
| `inst[0x492]` = byte offset `0x1248` | `get_assembly_context 0x0055B368` | `MOV ECX,dword ptr [EBP + 0x1248]` |
| `inst[0x492]` gated by `Scenario[0xd4b] != Scenario[0xd4c]` | `get_assembly_context 0x0055B33D` | `MOV EAX,[EBP+0x3530]; MOV ECX,[EBP+0x352c]; CMP EAX,ECX; JZ 0x0055b4d7` |
| `inst[0xd4b]` = byte offset `0x352C` (read) | `get_assembly_context 0x0055B343` | `MOV ECX,dword ptr [EBP + 0x352c]` |
| `inst[0xd4b]` = byte offset `0x352C` (write) | `get_assembly_context 0x0055B470` | `MOV dword ptr [EDX + 0x352c],ECX` |
| `SpecialFlags & 0x1000` = TS fog-of-war flag, YR default off | CLAUDE.md + `OBSERVER_SPECTATOR_FOG_GHIDRA_REPORT.md` | "standard YR fog-of-war is normally off unless this special flag is set" |
| `FUN_0053A110` checks `DAT_00a9fabc == 1` (LS ambient phase 1) | `decompile_function 0x0053A110` | `return DAT_00a9fabc == 1` |
| `FUN_0053A120` checks `DAT_00a9fabc == 2` (LS ambient phase 2) | `decompile_function 0x0053A120` | `return DAT_00a9fabc == 2` |
| `DAT_00a9fabc` is LS ambient lighting phase (0/1/2) | `decompile_function 0x0053A6C0` | `if (DAT_00a9fabc == 1) ... DAT_00a9fabc = 2 ... else if (DAT_00a9fabc == 2) ... DAT_00a9fabc = 0` |
| `FUN_004AE4C0` = full-map `Cell_ComputeZAdjust` | `decompile_function 0x004AE4C0` | Loop over `MapClass__CellIterator_Next` calling `Cell_ComputeZAdjust` |

---

## 4. Callee Identity Summary

| Timer fires → | Callee | Purpose |
|---|---|---|
| `inst[0x47a]` expiry | `TechnoClass__ProcessCellAction(0x0E, ...)` @ `0x006E53A0` | Cell-trigger timed action 0x0E (scenario event dispatch) |
| `inst[0x486]` expiry | `FUN_004ACAC0` | TS vein/tiberium-flow cell propagation (maps cells with bit 0x08 set) — TS era |
| `inst[0x489]` expiry | `FUN_004ACBC0` | TS fog/secondary-tiberium cell propagation (maps cells with bit 0x02 set) — TS era |
| `inst[0x492]` expiry | `FUN_004AE4C0` | Full-map `Cell_ComputeZAdjust` for terrain Z/ambient update |

---

## 5. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required effect | Acceptance scenario | Risk |
|---|---|---|---|---|---|---|
| `inst[0x47a]` / `0x11E8` is the scenario cell-action timer start frame; expires via `g_CurrentFrameCounter - start >= duration`; resets to `0xffffffff` on expiry; calls `TechnoClass__ProcessCellAction(0x0E)` and `FUN_004F42F0(2)`. | Decompile `0x0055AFB0`; asm `0x0055B17D..0x0055B1D5` | No matching `ScenarioClass` timer field exists in Rust; trigger/cell-action system is absent or split. | Future cell-action / trigger implementation in `sim/` | `ScenarioClass` needs a timer field at byte `+0x11E8` tracking last-start frame; expiry triggers cell-action dispatch. | Map with a timed cell-action trigger: action fires at correct frame after start. | Proposed test: `test_cell_action_timer_expires_at_correct_frame`. |
| `inst[0x489]` / `0x1224` block is **TS-dormant**: gated by `SpecialFlags & 0x1000` (fog-of-war), always 0 in standard YR. | Decompile `0x0055AFB0`; asm `0x0055B2C4..0x0055B2C7`; CLAUDE.md TS fog note | Do not implement | N/A | **Do not implement** `FUN_004ACBC0` block or `inst[0x489]` update logic for standard YR | Any standard YR skirmish: `FUN_004ACBC0` must never be called | `test_fog_cell_block_never_fires_in_standard_yr` |
| `inst[0x492]` / `0x1248` and `inst[0xd4b]` / `0x352C` drive the ambient lighting transition (darkening/brightening) during a lightning storm, stepping `Scenario[0xd4b]` toward `Scenario[0xd4c]` at rate `ftol(Rules+0x1668)` and triggering full-map `Cell_ComputeZAdjust` each step. | Decompile `0x0055AFB0`; `0x0053A6C0`; asm `0x0055B33D..0x0055B4AE` | Rust `lightning_storm.rs` has no ambient step counter or `Cell_ComputeZAdjust` trigger. | `src/sim/superweapon/lightning_storm.rs` | Add `Scenario.ambient_current` and `Scenario.ambient_target` counters; step toward target per-tick using `Rules.lightning_ambient_rate`; call map Z-recalc when stepping. | Fire a lightning storm → map ambient gradually darkens over N frames (Z-recalc fires each step). | `test_lightning_storm_ambient_transition_steps`; Risk: `Rules+0x1668` INI key identity unverified. |

---

## 6. Negative Facts / Do Not Do

1. **Do not implement `inst[0x489]`/`FUN_004ACBC0` for standard YR.** The `SpecialFlags & 0x1000` gate (fog-of-war) is off in all standard YR skirmish and MP maps. The entire block is TS-dormant. Evidence: `0x0055B2C4..0x0055B2C7`; CLAUDE.md.

2. **Do not implement `inst[0x486]`/`FUN_004ACAC0` unless TS vein cells are in scope.** Both `Rules+0x17F0 != 0` and `Rules+0x1640 != 0.0` must hold. `FUN_004ACAC0` scans for TS vein-cell bits (`0x08`/`0x10`) absent in standard YR maps. Evidence: `decompile_function 0x004ACAC0`.

3. **Do not conflate `inst[0x47a]` (scenario cell-action timer at `+0x11E8`) with the tiberium growth driver timers at `TiberiumClass+0x100/+0x11C`.** These are distinct timer structs in different objects. Evidence: `TIBERIUMCLASS_GROWTH_SPREAD_DRIVER_TIMERS_GHIDRA_REPORT.md`.

4. **Do not implement `ScenarioClass[0xd4b/0xd4c]` ambient counters outside the lightning storm context.** They are only written/read by the `inst[0x492]` block which queries lightning storm phase state. Evidence: decompile `0x0055AFB0`; `FUN_0053A110`/`FUN_0053A120` checking `DAT_00a9fabc`.

5. **Do not treat `Rules+0x1668` double as an AI difficulty float** (as suggested by `AI_DIFFICULTY_SYSTEM.md` context for trigger actions 0x47/0x48 writing `Rules+0x1668/0x1670`). In PerTickUpdate the same address is read as the ambient-transition rate, checked as `!= 0.0` to enable the transition block. These uses may coexist. Do not assume they conflict without checking the INI key identity.

---

## 7. Remaining Uncertainty

1. **`Rules+0x17F0` INI key identity not confirmed.** The field is a `bool` (byte) at `Rules+0x17F0`. ReadGeneral scanned but valid instruction addresses could not be found at exact boundaries containing this offset. Likely a TS vein/weather flag with YR default `false`, but identity is UNCHECKED.

2. **`Rules+0x1640` and `Rules+0x1648` double identity not confirmed.** Neither offset was found as an FSTP target in ReadGeneral. They may be set by the constructor (default 0.0) and never written by ReadGeneral, which would make their effective YR default `0.0` — meaning both `inst[0x486]` and `inst[0x489]` blocks would never fire even if the outer gating flags were set. UNCHECKED.

3. **`Rules+0x1668` double identity.** Context suggests it is the ambient-transition step rate. `AI_DIFFICULTY_SYSTEM.md` cites trigger actions 0x47/0x48 writing `RulesClass+0x1670` and `+0x1668` at runtime (possibly for a different use). Whether these are the same field or a field overlap is UNCHECKED.

4. **`DAT_00a9fab0` and `DAT_00a9fac0` identity** queried by `FUN_0053BAD0` / `FUN_0053B400`. These are adjacent to lightning storm globals but not identified in the lightning storm report. Their role in the `inst[0x492]` timer logic is inferred but not fully traced.

5. **What writes `ScenarioClass[0xd4c]` (`inst[0xd4c]`)** to set the ambient target. The PerTickUpdate block writes `inst[0xd4c]` (via `MOV [ECX+0x3530],EAX` at `0x0055B43C`), suggesting it is set within the same block. The initial setter during Lightning Storm start was not traced. UNCHECKED.

---

## 8. YELLOW — Unverified Claims

The following claims are UNVERIFIED and must not be used as implementation facts:

- `Rules+0x17F0` INI key identity and YR default value.
- `Rules+0x1640`, `Rules+0x1648` INI key identity and whether default is 0.0 (which would make the `inst[0x486]`/`inst[0x489]` blocks permanently inactive in YR even without the outer flag gates).
- `Rules+0x1668` identity and whether AI difficulty trigger writes and ambient-rate use are the same field.

---

## 9. Stale-Doc Replacement Wording

No existing doc covers these five specific ScenarioClass offsets in detail. The nearest existing docs (`PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md`, `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md`) mention the scenario timer block and `+0x47A/+0x47C` but do not give byte offsets for all five fields or per-field Active-in-YR verdicts.

If either doc is updated to add field details, replace the general reference to "scenario timer fields `+0x47A/+0x47C`" with the exact byte offsets and verdicts from Section 1 of this report.

---

## Sources

- `decompile_function 0x0055AFB0` (`LogicClassPerTickUpdateLiveVector`).
- `get_assembly_context` at: `0x0055B17D`, `0x0055B183`, `0x0055B1D8`, `0x0055B20B`, `0x0055B215`, `0x0055B228`, `0x0055B234`, `0x0055B2BE`, `0x0055B2DF`, `0x0055B33D`, `0x0055B343`, `0x0055B368`, `0x0055B470`.
- `decompile_function 0x004ACAC0`, `0x004ACBC0`, `0x004AE4C0`, `0x0053A110`, `0x0053A120`, `0x0053BAD0`, `0x0053B400`, `0x0053A6C0`, `0x00539EB0`.
- Prior docs: `PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md`, `LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md`, `LIGHTNING_STORM_SUPERWEAPON_GHIDRA_REPORT.md`, `TIBERIUMCLASS_GROWTH_SPREAD_DRIVER_TIMERS_GHIDRA_REPORT.md`, `SCENARIO_INIT_DEEP_DIVE.md`, `OBSERVER_SPECTATOR_FOG_GHIDRA_REPORT.md`.
- `CLAUDE.md` (TS fog-of-war gating documentation).

## Status

**COMPLETE** for per-field byte offsets, units, roles, and Active-in-YR verdicts with assembly evidence. **PARTIAL** for INI key identity of `Rules+0x17F0`, `0x1640`, `0x1648`, `0x1668` (see Remaining Uncertainty §7).
