# Shell First-Paint Slide — Tick/Schedule Formula Parity Trace

**Date:** 2026-05-30  
**Scope:** Schedule array build, total frame count, stagger (entry_tick per slot), Sleep cadence.  
**Binary:** FUN_006071E0 @ 0x006071E0 (gamemd.exe, YR)  
**Rust:** `src/app_shell_transition.rs` — `ShellFrameWave`  
**YR-active:** Yes. Confirmed via `FUN_00608260 → FUN_006071E0 @ 0x00608343` (DL=1 path) and `FUN_00622B50 → FUN_006071E0 @ 0x00622CAA` (DL=0 path), both reachable from every allow-listed shell dialog first paint. Allow-list includes 0xE2, 0x100, 0x102, 0x94, etc. (verified in SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md §3).

---

## Evidence Sources

- Live decompile: `decompile_function 0x006071E0` (this session)
- Live disassembly: `disassemble_function 0x006071E0` (this session)
- Supporting docs: `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md` (2026-05-19), `SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md` (2026-05-29), `SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md`

---

## 1. Schedule Array Construction (binary, verified)

**Inputs:**
- `iStack_168` = N = count of "group A" (active) controls enumerated by `FUN_0060A180` via `EnumChildWindows`
- `iVar8 = iStack_d0 = N + 1`
- Allocation: `operator_new((N+3)*4)` bytes → array has N+3 slots (indices [0..N+2])

**Fill loop (0x607672–0x60767F):**  
Writes values 1, 2, 3, ..., N+1 into indices [0..N]. Exit condition: `prev_value < N+1` → last written = N+1, loop exits with EAX = N+2.

**Post-fill assignments (0x607684–0x607690):**
```asm
00607684: INC EAX                              ; EAX = N+3
00607689: MOV [EDX + ESI*4 - 4], EBX          ; [N+2] = 0 (radar anchor)
0060768D: MOV [EDX + EDI*4], EAX              ; [N+1] = N+3 (SDWRNTMP/radar-open slot)
00607690: MOV [EDX + ESI*4 - 0xc], EBX        ; [N]   = 0 (SDMPBTN anchor, overwrites N+1)
```

**Resulting array (concrete for N=3):**

| Index | Value | Role |
|-------|-------|------|
| [0]   | 1     | slot 0 entry tick |
| [1]   | 2     | slot 1 entry tick |
| [2]   | 3     | slot 2 entry tick |
| [3]   | 0     | SDMPBTN anchor (overwritten) |
| [4]   | 6     | SDWRNTMP/radar-open: N+3 = 6 |
| [5]   | 0     | radar anchor |

**Resulting array (concrete for N=6):**

| Index | [0] | [1] | [2] | [3] | [4] | [5] | [6] | [7] | [8] |
|-------|-----|-----|-----|-----|-----|-----|-----|-----|-----|
| Value |  1  |  2  |  3  |  4  |  5  |  6  |  0  |  9  |  0  |

---

## 2. Max-Scan and Loop Bound (binary, verified)

**Max-scan (0x607694–0x6076A2):** Scans ESI = N+3 entries starting at index [0].  
For N=3: scans {1,2,3,0,6,0} → max = 6 = N+3.  
For N=4: scans {1,2,3,4,0,7,0} → max = 7 = N+3.  
For N=6: scans {1,2,3,4,5,6,0,9,0} → max = 9 = N+3.

**In all cases: max = N+3.**

**Loop bound (0x6076A4):**
```asm
006076a4: ADD ECX, 0x6       ; iStack_bc = max + 6 = N+3+6 = N+9
```

**Loop exit (0x607F22–0x607F29):**
```asm
00607f22: INC EAX            ; tick_after = tick_before + 1
00607f23: CMP EAX, ECX       ; compare tick_after vs iStack_bc
00607f29: JL  loop_start     ; continue while tick_after < N+9
```

Iteration count: ticks 0 through N+8 execute; loop runs when tick_after goes from 1 to N+8 (all < N+9). Tick N+9 would fail. **Total frames = N+9.**

**Concrete verification:**

| N (native) | max_entry | iStack_bc | Total frames (binary) |
|-----------|-----------|-----------|----------------------|
| 3         | 6         | 12        | 12                   |
| 4         | 7         | 13        | 13                   |
| 6         | 9         | 15        | 15                   |

---

## 3. Rust Formula

From `src/app_shell_transition.rs`:

```rust
pub(crate) const WAVE_TAIL_TICKS: u32 = 6;

fn total_ticks_for(slot_count: u32) -> u32 {
    let max_entry = slot_count + 2;     // Rust claims max = N+2
    max_entry + WAVE_TAIL_TICKS         // = N+2+6 = N+8
}

pub(crate) fn is_complete(&self) -> bool {
    self.tick >= self.total_ticks       // done when tick >= N+8
}

pub(crate) fn advance(&mut self, now: Instant) {
    // advances one tick per >=30ms, no catch-up
    if self.tick < self.total_ticks && now.duration_since(self.last_step_at) >= step {
        self.tick += 1;
        self.last_step_at += step;
    }
}
```

**Rust total frame count:**

| slot_count (Rust) | Rust max_entry | Rust total_ticks | Rust total frames |
|-------------------|----------------|------------------|-------------------|
| 3                 | 5              | 11               | 11                |
| 4                 | 6              | 12               | 12                |
| 6                 | 8              | 14               | 14                |

---

## 4. Parity Comparisons

### 4.1 Total Frame Count — FAIL

| N  | Binary total frames | Rust total frames | Delta |
|----|--------------------|--------------------|-------|
| 3  | 12                 | 11                 | **-1** |
| 4  | 13                 | 12                 | **-1** |
| 6  | 15                 | 14                 | **-1** |

**Root cause:** Rust computes `max_entry = N+2`. Binary max = N+3 (because the SDWRNTMP/radar-open slot at [N+1] is written N+3, not N+2). The doc comment in `total_ticks_for` says "anchor successor at (slot_count+1)+1" which would be N+3, but the code assigns `max_entry = slot_count + 2 = N+2`. Off by 1.

Binary: `local_17c[N+1] = iVar7 + 1` where `iVar7 = N+2` after the fill loop → value is N+3.  
Rust: doc comment says N+2, code uses N+2. **Both are wrong vs binary.**

**Verdict: FAIL. Binary total = N+9; Rust total = N+8 (1 frame short every time).**

### 4.2 Stagger (entry_tick per slot) — PASS

Binary: schedule array [slot] = slot+1 for slot 0..N-1.  
Entry tick for slot s = s+1. Verified from fill loop writing 1,2,...,N+1 starting at index 0.

Rust: `entry_tick(slot) = slot as i32 + 1`. Slot 0→tick 1, slot 1→tick 2, etc.

Both produce the same mapping. **PASS.**

### 4.3 Sleep duration (per-frame cadence) — PASS

Binary at 0x607F0F–0x607F11:
```asm
00607f0f: PUSH 0x1e
00607f11: CALL dword ptr [0x007e11f0]   ; Sleep(30 ms)
```
One Sleep(30) call per loop iteration, no time-accumulation logic in the binary.

Rust: `WAVE_TICK_MS = 30` and `advance()` steps at most one tick per >=30ms call, no catch-up:
```rust
self.last_step_at += step;  // pins next deadline, not wall-clock — faithful
```
**PASS.**

### 4.4 One-frame-per-sleep (no collapse) — PASS

Binary: straight `Sleep(N) → loop body → Sleep(N)` — no multi-step catch-up.  
Rust: `advance()` increments by at most 1 tick per call regardless of elapsed time. **PASS.**

---

## 5. Summary

| Check | Verdict | Detail |
|-------|---------|--------|
| Total frame count (N=3) | **FAIL** | Binary=12, Rust=11 |
| Total frame count (N=4) | **FAIL** | Binary=13, Rust=12 |
| Total frame count (N=6) | **FAIL** | Binary=15, Rust=14 |
| Stagger: entry_tick(slot) = slot+1 | PASS | Exact match |
| Sleep per tick = 30 ms | PASS | 0x1E = 30, exact match |
| One frame per sleep, no catch-up | PASS | No catch-up in binary or Rust |

**Root cause of FAIL:** Rust's `total_ticks_for` uses `max_entry = N+2` but the native max is N+3. The radar-open slot at array index [N+1] is written `N+3` (not N+2) because the fill loop leaves EAX=N+2 and then increments it once more before writing. The correct formula is:

```rust
fn total_ticks_for(slot_count: u32) -> u32 {
    let max_entry = slot_count + 3;   // was +2, must be +3
    max_entry + WAVE_TAIL_TICKS       // = N+3+6 = N+9
}
```

---

## 6. Player-Visible Effect

The animation terminates 1 frame (30 ms) early. All controls will still reach their fully-slid-in positions (the last ramp step and the held "after" terminal), but the game's post-animation logic (0x4EC or 0x4ED message send) fires 30 ms earlier than native. For slow hardware the difference is imperceptible; at 30 ms per frame it is a single-frame early completion visible only by frame-stepping capture or exact timing comparison.

---

## 7. Adjacent Findings (out of scope — not traced)

- **Frame INDEX values** (which SHP frame corresponds to which slot/tick): tracked in `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md` §5. Not traced here.
- **Control enumeration** (which controls count toward N for each dialog): `FUN_00608CD0` / `FUN_00609730` per `SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md` §3.3. Not traced.
- **Completion message** (`0x4EC` vs `0x4ED` based on DL mode): traced in prior docs. Not traced here.
- **Slide trigger** (first-paint `+0x1FC` state machine): covered in `SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md`. Not traced here.

---

*Evidence for all binary claims: `decompile_function 0x006071E0` and `disassemble_function 0x006071E0`, this session. No gamemd addresses or binary references in Rust code.*
