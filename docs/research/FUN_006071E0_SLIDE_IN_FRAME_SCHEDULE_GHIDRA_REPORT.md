# FUN_006071E0 — Main-Menu Button-Click Slide-In Transition Frame Schedule

**Date:** 2026-05-19
**Scope:** Per-iteration frame-index selection formula for SDMPBTN.SHP and SDWRNTMP.SHP
inside `FUN_006071E0 @ 0x006071E0`; flag-byte origins for `cVar15` and `cVar16`.
**Active in YR:** Yes in shell transition paths. Confirmed via
`FUN_00608260 -> FUN_006071E0` at `0x00608343` and common shell paint
`FUN_00622B50 -> FUN_006071E0` at `0x00622CAA`. 2026-05-27 clarification:
do not read this as proof that main-menu button `0x683` directly opens offline
Skirmish `0x102` through this helper. `SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md`
verifies that `0x683` first returns code `1`, `Main_Game` calls
`FUN_0060D380(1)`, and offline Skirmish is reached later through return code
`0x0B` / `g_GameMode = 5` / `FUN_006AE2C0`.

No Rust code was modified.

---

## 1. Background

`SDMPBTN_SDWRNTMP_RECT_CONSUMERS_GHIDRA_REPORT.md §3c` identified `FUN_006071E0`
as the one-shot slide-in/slide-out transition animator. That report left the
frame-index selection formula and the `cVar15`/`cVar16` flag bytes undecoded
(Open Question Q2). This report resolves both.

---

## 2. Function Signature and Calling Convention

```
void FUN_006071e0(void)    // no explicit stack params
```

Called via `__fastcall` with `ECX` = dialog HWND (the main-menu window handle).
`FUN_00608260` passes the HWND implicitly through ECX at `0x00608343`.
`FUN_00607FD0` also calls it via `__fastcall`, using the HWND found in the
dialog-record hash map.

The function itself extracts three flag bytes from the dialog record (see §3)
by doing a hash-map lookup on the HWND stored in ECX at entry.

**Evidence:** decompile `0x006071E0`; disassemble `0x006071E0`; decompile
`0x00608260`; `get_xrefs_to 0x006071E0`.

---

## 3. Flag Bytes: cVar15, cVar16, and cVar14 — Dialog Record Offsets

The decompiler labels `cVar14`, `cVar15`, `cVar16` and describes them as
`(char)((uint)unaff_EBX >> N)`. This notation is Ghidra's artifact for values
read from stack slots that were populated before EBX was zeroed (at
`0x006071EE: XOR EBX, EBX`). The actual values come from dialog-record fields
looked up via HWND.

### 3.1 Hash-map structure

Three sequential HWND-record hash-map lookups occur at the start of
`FUN_006071E0`. Each walk the linked list keyed by HWND and, when found, dereference
the record pointer with `ADD EAX, 0x4` (skipping the vtable/handle word at offset 0)
before reading the flag byte.

### 3.2 Offset table (verified from disassembly)

| Decompiler name | Stack slot | Assembly read | Record byte offset (base+4+raw) | Meaning |
|---|---|---|---|---|
| `cVar14` | `[ESP+0x11]` | `[EAX+0xD7]` at `0x0060727D` | base **+0xDB** | Slide direction / closing flag |
| `cVar15` | `[ESP+0x12]` | `[EAX+0xD6]` at `0x00607294` | base **+0xDA** | SDMPBTN enable flag |
| `cVar16` | `[ESP+0x13]` | `[EAX+0xD5]` at `0x006076DE` | base **+0xD9** | SDWRNTMP enable flag |

Notes:
- All three reads apply the `ADD EAX, 0x4` offset before the `[EAX+Dxx]` dereference
  (verified in disassembly at `0x00607276`, `0x0060728D`, `0x006076D3`).
- `cVar14` comes from a **different hash-map lookup** than `cVar15`/`cVar16`:
  `cVar14` is read from the first lookup (record identified by ESI = original HWND)
  at offset +0xDB; `cVar15` is from the second lookup (+0xDA); `cVar16` from the
  third lookup at a later block (+0xD9).
- BSS-initialized records zero all flag bytes. Whether +0xDA/+0xD9/+0xDB are set
  depends on the dialog's init path (not investigated this run — see §7 Open
  Questions).

**Evidence:** disassemble `0x006071E0`, instructions at
`0x00607276`/`0x0060727D`/`0x0060728D`/`0x00607294`/`0x006076D3`/`0x006076DE`.

---

## 4. Total Tick Count (iStack_bc / the Loop Bound)

### 4.1 Loop structure

The draw loop uses `uStack_184` as the iteration counter, initialized to 0 and
incremented by 1 per tick. The bound is `iStack_bc` (named `ECX` in the
decompiled max-of-array pass). The loop exits when `uStack_184 >= iStack_bc`.

### 4.2 iStack_bc derivation

```
iStack_d0 = iStack_168 + 2          // number of "child button" slots + 2
local_17c  = operator_new((iStack_168 + 3) * 4)  // schedule array
```

The schedule array `local_17c` is filled:
- `local_17c[0..iStack_d0-1]` = sequential integers starting at 1 (entry ticks
  for each button slot)
- `local_17c[iStack_d0]` = `iStack_d0 + 1 + 1` = last entry's successor
- `local_17c[iStack_168]` = 0  (SDMPBTN anchor index)
- `local_17c[iStack_168 + 2]` = 0  (radar-open anchor)

`iStack_bc` = max value in that array + 6. The "+6" extends the loop by 6 extra
ticks past the last button-entry tick to let the slide complete.

**Key result:** Total tick count = `max_schedule_entry + 6`. For a standard
main-menu with N buttons: `max_schedule_entry ≈ N + 2`, so total ticks ≈ **N + 8**.

### 4.3 Sleep per tick

`Sleep(0x1E)` = **30 ms** per iteration.
Total animation wall time ≈ `(N + 8) × 30 ms`.

**Evidence:** decompile `0x006071E0`, the `operator_new` block and the
`max + 6` computation.

---

## 5. Per-Tick Frame-Index Selection Formula

Each element drawn per tick follows the same 4-case pattern keyed on:
- `delta = uStack_184 - local_17c[slot_index]`  (current tick − schedule entry for this slot)
- `cVar14` (direction flag: 0 = slide-in, non-zero = slide-out/close)
- `iStack_174` = `cVar14 ? -1 : 1`  (direction multiplier; see §5.1)
- `iStack_13c`, `iStack_114`, `iStack_10c`, `local_118`, `iStack_110` = base-frame
  offsets per asset type (see §5.2)

### 5.1 Direction multiplier

```
iStack_174 = (-(uint)(cVar14 != 0) & 0xFFFFFFFE) + 1
           = cVar14 ? -1 : 1
```
When `cVar14 == 0` (slide-in): multiplier = **+1** (frames advance forward).
When `cVar14 != 0` (slide-out): multiplier = **-1** (frames advance in reverse).

### 5.2 Base-frame constants per element type (verified from decompile)

These constants come from the `iStack_174`-based conditional assignments in the
pre-loop setup block:

| Variable | cVar14==0 value | cVar14!=0 value | Used for |
|---|---|---|---|
| `iStack_13c` | 5 | 10 | SDBTNANM "active" buttons (closed end-frame for slide-in) |
| `iStack_114` | 0xB (11) | 0x10 (16) | SDBTNANM "inactive" button slots |
| `iStack_10c` | 1 | 6 | SDMPBTN frame base |
| `local_118` | 0 | 5 | SDMPBTN secondary frame base |
| `iStack_110` | 0 | 5 | Radar-open frame base |

### 5.3 Universal 4-case formula per element

For each draw element with schedule entry `S = local_17c[slot]`:

```
delta = current_tick - S

if delta < 0 OR delta == -1:
    frame = iStack_174 * (-1) * base_B + base_A   // "before entry" — held at first frame
    -- simplified: frame = -iStack_174 * base_B + base_A
                         = cVar14==0 ? base_A : base_A + base_B

if 0 <= delta < 6 AND delta != -2:
    frame = delta * iStack_174 + base_A            // "during transition" — 6 steps
    -- slide-in: frame = delta + base_A     (advances 0..5)
    -- slide-out: frame = -delta + base_A   (reverses base_A..base_A-5)

if delta >= 6 OR delta == -2:
    frame = held at terminal value
    -- slide-in (cVar14==0): frame = 6 + base_A (or equivalent max via SBB/AND mask)
    -- slide-out (cVar14!=0): frame = base_A (or equivalent min)
```

The exact assembly for the "before" and "after" terminal frames uses `NEG / SBB / AND`
to compute:
- `-(uint)(cVar14 != 0) & mask + addend`

which resolves to one of two constants depending on `cVar14`. The pattern is
repeated identically for every draw element.

### 5.4 SDMPBTN.SHP frame selection (cVar15 gate)

SDMPBTN.SHP is drawn only when `cVar15 != 0` (record byte +0xDA is set).

```
delta = current_tick - local_17c[iStack_168]

before entry (delta < 0 or -1):
    frame = cVar14==0 ? iStack_10c + 0   (=1)
          : iStack_10c + (-1)*(-1)*6 + 0 (=7 via mask)   // held at "out" end
    -- simpler: slide-in held at frame 1; slide-out held at frame 6

during transition (0 <= delta < 6):
    FIRST: draw SDBTNANM SHP at frame 1 at same position (confirmed from
           CC_Draw_Shape with hardcoded frame=1 at 0x006078C9)
    THEN:  frame = delta * iStack_174 + local_118
               slide-in: frame = delta + 0  (=delta, range 0..5)
               slide-out: frame = -delta + 5 (range 5..0)

after (delta >= 6 or -2):
    slide-in: frame = 0 (terminal open, cVar14==0 → setz → 1... see below)
    slide-out: frame = 1 (terminal closed)
```

Exact terminal frames (from SBB/SETZ at `0x00607820-0x00607838`):
- slide-in terminal: `frame = (cVar14==0) ? 1 : 0`  (SETZ DL: sets 1 when equal-zero)
- slide-out terminal: `frame = (cVar14!=0) ? 1 : 0`

So SDMPBTN.SHP has 6 transition frames (indices 0–5 for slide-in) plus
frame 1 (held before entry) and frame 1/0 at the terminals.

**Evidence:** decompile `0x006071E0` block from `LAB_006077ba`; disassemble
`0x006078B5`–`0x006079CA`.

### 5.5 SDWRNTMP.SHP frame selection (cVar16 gate)

SDWRNTMP.SHP is drawn only when `cVar16 != 0` (record byte +0xD9 is set).

```
delta = current_tick - local_17c[iStack_d0]
        (iStack_d0 = iStack_168 + 2; last-button-slot + 2)

before entry (delta < 0 or -1):
    frame = (cVar14==0) ? 1 : 0    // SETZ: 1 when cVar14 is zero

during transition (0 <= delta < 6):
    FIRST: draw SDBTNANM at frame 1 (same as SDMPBTN path)
    THEN:  frame = delta * iStack_174 + local_118
               slide-in: frame = delta + 0 (0..5)
               slide-out: frame = -delta + 5 (5..0)

after (delta >= 6 or -2):
    frame = (cVar14!=0) ? 1 : 0
```

SDWRNTMP.SHP draw position uses `local_17c[iStack_d0]` as the stagger slot,
placing it 2 slots later in the schedule than SDMPBTN, so it slides in AFTER
the button columns.

**Evidence:** decompile `0x006071E0` block from `LAB_006077ba` to `LAB_006079cf`;
disassemble `0x006077C6`–`0x00607939`.

---

## 6. Draw Positions (Rects)

The draw-position arguments to `CC_Draw_Shape` for both SHPs use coordinates
returned by `FUN_0072a9c0()` (a screen-to-client coordinate helper called
immediately before each `CC_Draw_Shape`). The rect pointers passed are:

- **SDMPBTN path:** `&uStack_cc` / `&uStack_c4` — populated from `FUN_0072a9c0()`
  with `&uStack_fc` as the input (which is `uStack_a4..uStack_98`, i.e., the
  copy of `DAT_00B0FC14`).
- **SDWRNTMP path:** `&uStack_144` / `&uStack_5c` — populated from `FUN_0072a9c0()`
  with `&uStack_c0` as the input (which is `iStack_e0..iStack_d4`, the copy of
  `DAT_00B0FC18`).

Both rects are client-coordinate-adjusted copies of the globals
`DAT_00B0FC14` (SDMPBTN) and `DAT_00B0FC18` (SDWRNTMP), consistent with
the background report.

**Evidence:** decompile `0x006071E0`, rect-load block at
`0x00607392`–`0x006073C3`; `FUN_0072a9c0` call sites inside the loop.

---

## 7. End-of-Animation Signal

After the loop exits:

```c
FUN_007c8b3d(local_17c);    // free schedule array

if (cVar14 == 0) {           // slide-IN complete
    SendMessageA(local_164, 0x4ED, 0, 0);
    return;
}
// slide-OUT complete:
VocClass__PlayAtPos(0x3F800000, 0);
// flush display chain
SendMessageA(local_164, 0x4EC, 0, 0);
```

- Message `0x4ED` = transition-in complete (navigate to new dialog).
- Message `0x4EC` = transition-out complete (close/return).

**Evidence:** decompile `0x006071E0`, epilogue block; disassemble
`0x00607F39`–`0x00607FC0`.

---

## 8. Caller Setup in FUN_00608260

`FUN_00608260 @ 0x00608260` calls `FUN_006071e0()` at `0x00608343` after:
1. Verifying record byte `+0xC1 != 0` and `piVar1[0x2D] == 1`.
2. `IsWindowVisible(param_1)` — window must be visible.
3. `IsWindowEnabled(param_1)` — value stored for post-call restore.
4. `EnableWindow(param_1, 0)` — disables the button during animation.
5. `EnumChildWindows(param_1, LAB_00606800, 1)` — some pre-animation child enum.

The flags `cVar14`/`cVar15`/`cVar16` are NOT set by `FUN_00608260`; they are
read from the dialog record by `FUN_006071E0` itself using the HWND in ECX.
`FUN_00608260` passes no explicit parameters — `ECX` holds `param_1` (the HWND).

**Evidence:** decompile `0x00608260`; `get_xrefs_to 0x006071E0` confirming
call site `0x00608343`.

---

## 9. Confidence Assessment (3-axis model)

- **Content** (what the formula does): **HIGH**. The loop body, delta computation,
  4-case branches, and CC_Draw_Shape calls were directly decompiled and
  disassembled. The frame-index arithmetic is explicit.
- **Identity** (correct SHP assets): **HIGH**. `local_14c = DAT_00b0f9dc` (SDMPBTN)
  and `iStack_e0 = *DAT_00b0fc18` (SDWRNTMP) are consistent with the background
  report; the draw path gates are on `cVar15` and `cVar16` respectively.
- **Binding** (do +0xDA/+0xD9 actually get set for a main-menu click): **MEDIUM**.
  The reads are verified from the binary. Whether +0xDA/+0xD9 are non-zero for
  a standard main-menu button click depends on the dialog's init path (which
  sets these bytes). This was not traced this session (see §10 Q2).

**Active in YR:** Yes. `FUN_00608260` (the button-press handler) calls
`FUN_006071E0` on every main-menu button click in YR. The full loop executes
regardless of `cVar15`/`cVar16` (the flags gate individual SHP draws, not the
loop itself).

---

## 10. Open Questions

1. **Which init path sets record bytes +0xDA (cVar15) and +0xD9 (cVar16)?**
   Without knowing the setter, we cannot confirm whether SDMPBTN/SDWRNTMP are
   actually drawn during a standard main-menu click. Candidate setters:
   classifier helpers `FUN_0060CAF0` / `FUN_0060C930` (per sibling report they
   write +0xD9/+0xDA/+0xDB/+0xDC). If +0xDA is written by one of these, SDMPBTN
   draws during the transition.

2. **What is the exact frame count for SDMPBTN.SHP and SDWRNTMP.SHP in the
   retail SHP files?** Needed to confirm the 0..5 range doesn't overflow. Not
   investigated this run (asset-file inspection required).

3. **What does `iStack_168` resolve to at runtime?** It controls the schedule
   array dimension and the SDMPBTN slot index. It is populated by
   `EnumChildWindows(hWnd, FUN_0060A180, 0)` which writes `DAT_00AC1CAC` and
   subsequently sets `iStack_168` from that count. The child count equals the
   number of visible main-menu buttons.

4. **Does `FUN_00607FD0` set any flag bytes before calling `FUN_006071E0`?**
   It only checks record byte `+0xC2` and clears it after the call. It does not
   appear to set +0xDA/+0xD9 (decompile `0x00607FD0` confirms no writes to those
   offsets). Flags must already be set by dialog init.

5. **WestWood Online callers (`0x00789B60`, `0x00788B00`, etc.):** Also call
   `FUN_006071E0`. These are WOL-specific and not relevant to a standard
   skirmish main-menu flow (out of scope for this session).

---

## 11. Summary of Verified Facts

1. `cVar15` = dialog record byte **+0xDA** (base+4+0xD6), read at `0x00607294`.
   Gates SDMPBTN.SHP draws inside the loop. (Evidence: disassemble `0x006071E0`
   at `0x0060728D`–`0x0060729A`)

2. `cVar16` = dialog record byte **+0xD9** (base+4+0xD5), read at `0x006076DE`.
   Gates SDWRNTMP.SHP draws inside the loop. (Evidence: disassemble `0x006071E0`
   at `0x006076D3`–`0x006076E4`)

3. `cVar14` = dialog record byte **+0xDB** (base+4+0xD7), read at `0x0060727D`.
   Controls direction: 0 = slide-in (frame multiplier +1, terminal message 0x4ED);
   non-zero = slide-out (frame multiplier -1, terminal message 0x4EC).
   (Evidence: disassemble `0x006071E0` at `0x00607272`–`0x00607283`)

4. Loop bound `iStack_bc` = max(schedule_array) + **6**, with `Sleep(30 ms)` per
   tick. Schedule array is built from child-button count (`iStack_168`).
   (Evidence: decompile `0x006071E0`, `max + 6` block at `~0x006076A4`)

5. Both SHPs follow a **6-step linear transition** (delta 0..5) keyed on
   `(current_tick − schedule_entry) × direction_multiplier + base_frame`,
   bracketed by held "before" and "after" terminal frames determined by `cVar14`.
   (Evidence: decompile `0x006071E0`, per-element 4-case branch pattern throughout
   the loop body; disassemble confirms `IMUL EAX, [ESP+0x2C]` at `0x006078E6` etc.)
