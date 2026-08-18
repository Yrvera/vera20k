# Shell First-Paint Slide — SDBTNANM Frame-Index Schedule Trace

**Date:** 2026-05-30
**Scope:** Verify Rust SDBTNANM per-frame constants in `src/app_shell_transition.rs` against
gamemd `FUN_006071E0 @ 0x006071E0`.
**Method:** Live Ghidra decompile of `0x006071E0` this session + prior research docs.
**Rust file under audit:** `src/app_shell_transition.rs`
**Active in YR:** YES — `FUN_00610CA0` (owner-draw subclass wndproc) fires the slide on the
first `WM_PAINT` of every allow-listed shell dialog (`0xE2`, `0x100`, `0x101`, `0x102`,
`0x94`, `0x129`, and ~45 others). Confirmed `SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md`.

---

## 1. Verification Source

Full decompile of `FUN_006071E0 @ 0x006071E0` obtained this session via Ghidra MCP
`decompile_function 0x006071E0`. Prior docs used as background only; all frame-constant
claims below are verified from the live decompile, not from prior docs.

---

## 2. Base-Frame Constants: gamemd vs Rust

### 2.1 gamemd — base-frame setup block (verified from decompile)

```c
iStack_13c = 5;   // slide-in; cVar14!=0 → 10   (Group A / active buttons)
iStack_114 = 0xb; // slide-in (=11); cVar14!=0 → 0x10 (=16)  (Group B / inactive buttons)
iStack_10c = 1;   // slide-in; cVar14!=0 → 6   (SDMPBTN base — not SDBTNANM)
local_118  = 0;   // slide-in; cVar14!=0 → 5   (SDMPBTN/SDWRNTMP ramp base)
iStack_110 = 0;   // slide-in; cVar14!=0 → 5   (Radar-open ramp base)
iStack_174 = (cVar14!=0) ? -1 : 1;             // direction multiplier
```

### 2.2 gamemd terminal frame computation — Group A

**Before-entry terminal** (`delta < 0`): `(-(uint)(cVar14 != 0) & 9) + 1`
- slide-in  (cVar14==0): `-(0) & 9 + 1 = 0 + 1 = 1`
- slide-out (cVar14!=0): `-(1) & 9 + 1 = 0xFFFFFFFF & 9 + 1 = 9 + 1 = 10`

**After-entry terminal** (`delta >= 6`): `(-(uint)(cVar14 != 0) & 0xFFFFFFF7) + 10`
- slide-in  (cVar14==0): `0 + 10 = 10`
- slide-out (cVar14!=0): `0xFFFFFFFF & 0xFFFFFFF7 + 10 = 0xFFFFFFF7 + 10 = -9 + 10 = 1`

**Ramp** (0 <= delta < 6): `delta * iStack_174 + iStack_13c`
- slide-in:  `delta + 5`  → frames 5, 6, 7, 8, 9, 10
- slide-out: `-delta + 10` → frames 10, 9, 8, 7, 6, 5

### 2.3 gamemd terminal frame computation — Group B

**Before-entry terminal** (`delta < 0`): `-(uint)(cVar14 != 0) & 10`
- slide-in  (cVar14==0): `0 & 10 = 0`
- slide-out (cVar14!=0): `0xFFFFFFFF & 10 = 10`

**After-entry terminal** (`delta >= 6`): `(-(uint)(cVar14 != 0) & 0xFFFFFFF6) + 10`
- slide-in  (cVar14==0): `0 + 10 = 10`
- slide-out (cVar14!=0): `0xFFFFFFFF & 0xFFFFFFF6 + 10 = 0xFFFFFFF6 + 10 = -10 + 10 = 0`

**Ramp** (0 <= delta < 6): `delta * iStack_174 + iStack_114`
- slide-in:  `delta + 11` → frames 11, 12, 13, 14, 15, 16
- slide-out: `-delta + 16` → frames 16, 15, 14, 13, 12, 11

### 2.4 Comparison table

| Constant           | gamemd (slide-in)         | Rust `GROUP_A_IN`           | Verdict    |
|--------------------|---------------------------|-----------------------------|------------|
| `before`           | 1                         | `before: 1`                 | **PASS**   |
| `base` (ramp base) | 5 (`iStack_13c`)          | `base: 5`                   | **PASS**   |
| `after`            | 10                        | `after: 10`                 | **PASS**   |

| Constant           | gamemd (slide-out)        | Rust `GROUP_A_OUT`          | Verdict    |
|--------------------|---------------------------|-----------------------------|------------|
| `before`           | 10                        | `before: 10`                | **PASS**   |
| `base` (ramp base) | 10 (`iStack_13c`)         | `base: 10`                  | **PASS**   |
| `after`            | 1                         | `after: 1`                  | **PASS**   |

| Constant           | gamemd (slide-in)         | Rust `GROUP_B_IN`           | Verdict    |
|--------------------|---------------------------|-----------------------------|------------|
| `before`           | 0                         | `before: 0`                 | **PASS**   |
| `base` (ramp base) | 11 (`iStack_114 = 0xb`)   | `base: 11`                  | **PASS**   |
| `after`            | 10                        | `after: 10`                 | **PASS**   |

| Constant           | gamemd (slide-out)        | Rust `GROUP_B_OUT`          | Verdict    |
|--------------------|---------------------------|-----------------------------|------------|
| `before`           | 10                        | `before: 10`                | **PASS**   |
| `base` (ramp base) | 16 (`iStack_114 = 0x10`)  | `base: 16`                  | **PASS**   |
| `after`            | 0                         | `after: 0`                  | **PASS**   |

---

## 3. Ramp Step Count (WAVE_RAMP_STEPS)

gamemd: `if (iVar8 < 6)` branch condition (present at multiple sites in the loop body,
corresponding to task-brief addresses `0x0060773F`, `0x00607B74`, `0x00607D7D`).
Delta range: 0, 1, 2, 3, 4, 5 → **6 steps**.

Rust: `WAVE_RAMP_STEPS = 6`; condition `delta < WAVE_RAMP_STEPS`.

**PASS** — exact match.

---

## 4. sdbtnanm_frame() Formula Structure

gamemd 4-case pattern for each element (Group A path shown):
```
if delta < 0                → before terminal
else if delta < 6           → ramp: delta * iStack_174 + iStack_13c
    (inner if delta==-1 → before, delta==-2 → after: dead code — negative
     delta already matched outer `< 0`; Ghidra decompiler artifact)
else                        → after terminal
```

Rust `sdbtnanm_frame()`:
```rust
if delta < 0         { f.before }
else if delta < 6    { f.base + delta * dir }
else                 { f.after }
```

The gamemd `delta == -1` / `delta == -2` sub-branches inside the `0 <= delta < 6` guard
are **dead code** (unreachable given the signed outer `< 0` check). The Rust 3-case
structure is therefore **equivalent** to gamemd's 4-case decompiler output.

**PASS** — formula structure matches.

---

## 5. Schedule Array and Total Tick Count

### 5.1 gamemd schedule array construction (verified from decompile)

```c
iVar8 = iStack_168 + 1;   // iStack_168 = button-column count N
// Fill loop: local_17c[0]=1, [1]=2, ..., [iStack_168]=N+1
// After loop: iVar7 = N+2
local_17c[N + 2] = 0;           // radar anchor = 0
local_17c[N + 1] = iVar7 + 1;   // successor slot = (N+2)+1 = N+3
local_17c[N]     = 0;           // SDMPBTN anchor = 0
```

Resulting array: `{1, 2, ..., N, 0, N+3, 0}` at indices `0..N+2`.

`max(schedule_array) = N+3`  
`iStack_bc = N+3+6 = N+9`

### 5.2 Rust total_ticks_for(N)

```rust
let max_entry = slot_count + 2;   // = N+2  ← WRONG
max_entry + WAVE_TAIL_TICKS       // = N+2+6 = N+8
```

**FAIL** — Rust computes `N+8`; gamemd computes `N+9`. Off by **1 tick (30 ms)**.

The Rust `max_entry` assumes the successor slot lands at `N+2`, but gamemd places it at
`N+3` (the fill loop writes `N+1` into `local_17c[N+1]`, then `local_17c[N+1] = iVar7+1`
overwrites it with `N+3`). The WAVE_TAIL_TICKS=6 value itself is correct.

**Observable effect:** The animation completes 30 ms early. The terminal held-frame
(all buttons at their `after` position) is displayed for one fewer tick before the wave
transitions to steady-state idle paint. Visible as a marginally shorter dwell at the
fully-slid-in state before the slide completes.

**Affected Rust:** `src/app_shell_transition.rs:128-133` (`total_ticks_for`)
**gamemd evidence:** decompile `0x006071E0`, schedule build block; `iStack_bc = iStack_bc + 6`.

### 5.3 WAVE_TAIL_TICKS = 6

gamemd: `iStack_bc = iStack_bc + 6` (verified from decompile).
Rust: `WAVE_TAIL_TICKS = 6`.
**PASS.**

---

## 6. Entry Tick per Slot

gamemd: `local_17c[slot] = slot + 1` for slots 0..N-1 (fill loop, 1-indexed).  
Rust: `entry_tick(slot) = slot as i32 + 1`.  
**PASS** — exact match.

---

## 7. Direction Multiplier

gamemd: `iStack_174 = (cVar14!=0) ? -1 : 1`.  
Rust: `WaveDirection::SlideIn => 1`, `WaveDirection::SlideOut => -1`.  
**PASS.**

---

## 8. Decompiler Dead-Code Note

The inner `if (iVar8 == -1) goto LAB_...` and `if (iVar8 == -2) goto LAB_...` guards
inside the `0 <= delta < 6` branch in each draw group are **unreachable**. A delta of
-1 or -2 is negative, so the outer `if (delta < 0)` catches it first. These are Ghidra
decompiler artifacts of the SBB/NEG/AND pattern used for branchless terminal computation.
The Rust 3-case structure is the correct clean translation.

---

## 9. Adjacent Findings (out-of-scope for this trace)

These are observed incidentally; the specific mechanics are NOT traced here:

1. **Group assignment for skirmish back-button:** gamemd draws the back button (`cStack_175`
   path) using `iStack_13c` (Group A constants). Rust assigns all skirmish right-panel
   buttons to `ButtonGroup::A`. Appears consistent, but the exact slot-to-group assignment
   for the mixed-type dialog 0x102 (active vs. inactive buttons) has not been verified
   per-control-ID in this trace.

2. **SDMPBTN/SDWRNTMP frame schedule:** These use `local_118` (base 0 / 5) and `iStack_10c`
   (base 1 / 6) with the same 6-step ramp. Rust does not appear to implement
   SDMPBTN/SDWRNTMP transition drawing. This is adjacent; not traced here.

3. **Signal after slide completion:** gamemd sends `0x4ED` (slide-in) or `0x4EC` (slide-out
   with text-reveal broadcast). Rust clears `shell_first_paint_slide = None` on completion.
   The downstream `0x4EC → 0x4EE` text-reveal chain is not implemented. Out of scope.

4. **Control enumeration and group classification:** `FUN_0060A180` / `FUN_0060A250` filter
   which buttons participate and which are Group A vs. back-button. Per-dialog enumeration
   has not been traced per-slot in this report.

---

## 10. Stage Summary

| Stage                              | Rust value / formula                     | gamemd value / formula           | Verdict      |
|------------------------------------|------------------------------------------|----------------------------------|--------------|
| GROUP_A_IN.before                  | 1                                        | 1                                | PASS         |
| GROUP_A_IN.base                    | 5                                        | 5 (`iStack_13c`)                 | PASS         |
| GROUP_A_IN.after                   | 10                                       | 10                               | PASS         |
| GROUP_A_OUT.before                 | 10                                       | 10                               | PASS         |
| GROUP_A_OUT.base                   | 10                                       | 10                               | PASS         |
| GROUP_A_OUT.after                  | 1                                        | 1                                | PASS         |
| GROUP_B_IN.before                  | 0                                        | 0                                | PASS         |
| GROUP_B_IN.base                    | 11                                       | 11 (`iStack_114 = 0xb`)          | PASS         |
| GROUP_B_IN.after                   | 10                                       | 10                               | PASS         |
| GROUP_B_OUT.before                 | 10                                       | 10                               | PASS         |
| GROUP_B_OUT.base                   | 16                                       | 16 (`iStack_114 = 0x10`)         | PASS         |
| GROUP_B_OUT.after                  | 0                                        | 0                                | PASS         |
| WAVE_RAMP_STEPS                    | 6                                        | 6 (`delta < 6`)                  | PASS         |
| WAVE_TAIL_TICKS                    | 6                                        | 6 (`iStack_bc + 6`)              | PASS         |
| entry_tick formula                 | slot + 1                                 | slot + 1                         | PASS         |
| direction multiplier               | +1 / -1                                  | +1 / -1                          | PASS         |
| sdbtnanm_frame() 4-case structure  | 3-case (dead branches omitted)           | 4-case decompiler (1 dead arm)   | PASS         |
| total_ticks_for(N)                 | N+8 (`max_entry=N+2`)                    | N+9 (`max_entry=N+3`)            | **FAIL**     |

---

## 11. Confidence Assessment

- **Content** (formula correctness): HIGH — all values read directly from live decompile.
- **Identity** (correct function): HIGH — `0x006071E0` is the slide-in/out animator per
  multiple independent prior docs and confirmed caller chain.
- **Binding** (active in YR): HIGH — confirmed via `SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md` that this runs on first paint of every allow-listed shell dialog in standard YR skirmish/menu.
