# Power Bar Three-Band Split Trace

**Mechanic:** Drain / Output / Surplus segment distribution (Phase 2 of `compute_targets`).  
**Scenario:** `max_segments=50` (bar_height_px=150), `theoretical_total=400` → `filled=25`.  
**Date:** 2026-05-20  
**Status:** COMPLETE

---

## Fixtures and Results

### Common Phase 1 (shared by all three fixtures)

```
total = 400, FILL_SCALE = 400.0
empty_ratio = 400 / (400 + 400) = 0.5
empty = ftol(50 * 0.5) = 25, clamped to [0,49] = 25
filled = 50 - 25 = 25
```

Verified against POWER_BAR_RENDERING.md table (total_segments=50, total_power=400 → filled=25). PASS.

---

### Fixture A: Output=300, Drain=100

```
surplus_raw = 300 - 100 = 200 >= OUTPUT_CAP(100)
  → output_portion = 100.0
  → surplus_portion = 100.0
sum = 100 + 100 + 100 = 300
drain_frac = 100/300 = 1/3
output_frac = 100/300 = 1/3
surplus_frac = 100/300 = 1/3

filled * drain_frac   = 25/3 = 8.333... → ftol = 8
filled * output_frac  = 25/3 = 8.333... → ftol = 8
filled * surplus_frac = 25/3 = 8.333... → ftol = 8
total_error = 0.333 + 0.333 + 0.333 = 1.0
gamemd drain = ftol(1.0 + 8 + 0.01) = ftol(9.01) = 9
our drain    = 8 + residual(1) = 9

target_drain=9, target_output=8, target_surplus=8, sum=25 ✓
```

**Verdict: PASS** (both produce drain=9, output=8, surplus=8).

---

### Fixture B: Output=150, Drain=100

```
surplus_raw = 150 - 100 = 50 < OUTPUT_CAP(100)
  → output_portion = 50.0
  → surplus_portion = 0.0
sum = 100 + 50 + 0 = 150
drain_frac   = 100/150 = 2/3
output_frac  =  50/150 = 1/3
surplus_frac = 0

filled * drain_frac   = 25 * 2/3 = 16.667 → ftol = 16
filled * output_frac  = 25 * 1/3 =  8.333 → ftol = 8
filled * surplus_frac = 0 → 0
total_error = 0.667 + 0.333 + 0 = 1.0
gamemd drain = ftol(1.0 + 16 + 0.01) = ftol(17.01) = 17
our drain    = 16 + residual(1) = 17

target_drain=17, target_output=8, target_surplus=0, sum=25 ✓
```

**Verdict: PASS** (both produce drain=17, output=8, surplus=0).

---

### Fixture C: Output=50, Drain=100

```
surplus_raw = 50 - 100 = -50 < 0
  → output_portion = 0.0
  → surplus_portion = 0.0
sum = 100 + 0 + 0 = 100
drain_frac = 1.0, output_frac = 0.0, surplus_frac = 0.0

filled * drain_frac   = 25 * 1.0 = 25.0 → ftol = 25
filled * output_frac  = 0 → 0
filled * surplus_frac = 0 → 0
total_error = 0
gamemd drain = ftol(0 + 25 + 0.01) = ftol(25.01) = 25
our drain    = 25 + residual(0) = 25

target_drain=25, target_output=0, target_surplus=0, sum=25 ✓
```

**Verdict: PASS** (both produce drain=25, output=0, surplus=0).

---

## Constants Verified from Binary

| Constant | Address | Binary (LE hex) | Decoded | Expected | Match |
|----------|---------|-----------------|---------|----------|-------|
| OUTPUT_CAP | 0x007E2AC0 | `0000000000005940` | 100.0 | 100.0 | PASS |
| Rounding epsilon | 0x007E3808 | `7b14ae47e17a843f` | 0.01 | 0.01 | PASS |
| Zero baseline | 0x007E2800 | `0000000000000000` | 0.0 | 0.0 | PASS |

Verified via `read_memory` at each address, decoded as IEEE 754 little-endian doubles.

---

## gamemd Algorithm (Verified from Assembly at 0x0063f960)

`PowerClass__Calc_Power_Distribution(param_1=surplus*, param_2=output*, param_3=drain*)`

### Branch structure (0x63f9c5–0x63fa0f)

```
surplus_int = Power_Output() - Power_Drain()
FILD surplus_int
FCOM [0x7e2800]  ; compare with 0.0
if surplus_int < 0:   (C0=1, JZ not taken → 0x63f9d2)
    output_portion = 0.0
    surplus_portion = 0.0
else:
    FCOM [0x7e2ac0]  ; compare with 100.0
    if surplus_int < 100:   (C0=1, JZ not taken → 0x63fa03)
        output_portion = (double)surplus_int
        surplus_portion = 0.0
    else:   (JZ taken → 0x63fa07)
        output_portion = 100.0        ; [ESP+0x18] initialized at 0x63f9b9
        surplus_portion = surplus - 100.0
```

### Proportional split (0x63fa82–0x63fa9a)

```
sum = drain + output_portion + surplus_portion
if sum > 0:
    drain_frac   = drain / sum
    surplus_frac = surplus_portion / sum
    output_frac  = output_portion / sum   ; stored to [ESP+0x30]
else:
    drain_frac = 1.0 (from default at [ESP+0x3c]=0x3ff00000)
    surplus_frac = 0.0, output_frac = 0.0
```

### ftol sequence (0x63faac–0x63fae6)

```
; FPU stack: { 25.0(filled), surplus_frac, drain_frac }
FMUL ST3   → 25 * drain_frac  → ftol → *param_3 (drain)
FLD ST0    → dup 25.0
FMUL [output_frac_mem] → ftol → *param_2 (output)
FMUL ST1   → 25.0 * surplus_frac → ftol → *param_1 (surplus)
```

### Residual (rounding) correction (0x63fae8–0x63fb0d)

```
error = (output_pre - output_segs) + (surplus_pre - surplus_segs) + (drain_pre - drain_segs)
new_drain = ftol(error + drain_segs + 0.01)
*param_3 = new_drain   ; REPLACES drain (does not add to it)
```

This is algebraically equivalent to:
`new_drain = ftol(filled - output_segs - surplus_segs + fractional_err_drain + 0.01)`

For typical cases with small fractional error, this equals `filled - output_segs - surplus_segs`.

**Key insight:** gamemd REPLACES drain (not adds to it). Our code adds `residual` to initial drain, which is equivalent because:
- `gamemd: drain = ftol(err + drain_init + 0.01)` = `drain_init + ftol(total_frac_error + 0.01)` = `drain_init + residual`
- Our: `drain = drain_init + residual`
- For all three fixtures these produce identical integer results (verified numerically above).

---

## Rust Code vs. gamemd Comparison

**File:** `src/sidebar/power_bar_anim.rs`, fn `compute_targets()`, lines 242–264.

| Aspect | gamemd | Our code | Match |
|--------|--------|----------|-------|
| surplus_raw < 0 branch | output_portion=0, surplus_portion=0 | `(0.0, 0.0)` | PASS |
| surplus_raw in [0, 100) | output_portion=surplus_raw, surplus_portion=0 | `(surplus_raw, 0.0)` | PASS |
| surplus_raw >= 100 | output_portion=100, surplus_portion=surplus_raw-100 | `(OUTPUT_CAP, surplus_raw-OUTPUT_CAP)` | PASS |
| sum == 0 fallback | drain_frac=1.0, rest=0 | `(1.0, 0.0, 0.0)` | PASS |
| ftol for segment counts | truncating (`FISTP`) | Rust `as i32` (truncates) | PASS |
| Residual → drain | `drain = ftol(err+drain_init+0.01)` | `drain_init += residual` | PASS (equiv.) |
| OUTPUT_CAP value | 100.0 at 0x007E2AC0 | `const OUTPUT_CAP: f64 = 100.0` | PASS |

---

## Verdict Tally

PASS: 7 | FAIL: 0 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

---

## Adjacent Findings (out of scope)

- **Step priority in `step_one_segment`**: Our code steps surplus first, then drain, then output. gamemd's AI function checks drain first (+0x1534), then surplus (+0x152C), then output (+0x1530). This is the step priority (which band moves first during animation), NOT the split computation. This is a slot 4 (slide animation) concern — out of scope for this trace.
- **0.01 epsilon purpose**: Guards against exact half-integer cases where the sum of truncated fracs could be one less than `filled` without the epsilon nudging the residual upward. Has no effect when total fractional error is an exact integer (rare in practice).
- **IHouse vtable reads**: `Power_Drain()` is vtable[9] (+0x24 from +0x24 base), `Power_Output()` is vtable[8] (+0x20 from +0x24 base). Our code takes pre-computed `power_output`/`power_drain` as i32 parameters — equivalent, since the values come from the same HouseClass fields (+0x53A4, +0x53A8).
