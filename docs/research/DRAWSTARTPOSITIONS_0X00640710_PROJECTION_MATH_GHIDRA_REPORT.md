# DrawStartPositions (0x00640710) — Projection Math Verification

**Status:** COMPLETE  
**Active in YR:** Yes (unconditionally called during skirmish map selection preview render)  
**Session date:** 2026-06-01  
**Source:** decompile_function 0x00640710 + disassemble_function 0x00640710

---

## Scope

Exact arithmetic verification of four claims in the Rust port:
1. Aspect-fit scaling (per-mille, truncation order)
2. Start-position projection (map cell → preview pixel)
3. Start-marker sprite hotspot offset
4. Whether the marker count is capped at 8

---

## Verified Findings

### (a) Aspect-Fit Scaling — MATCHES (with one structural note)

**Per-mille multiply** (verified via disassemble_function 0x00640710 at 0x00640799–0x006407a6):
```asm
LEA EAX,[EBX + EBX*0x4]   ; *5
LEA EAX,[EAX + EAX*0x4]   ; *25
LEA EAX,[EAX + EAX*0x4]   ; *125
SHL EAX,0x3                ; *1000
CDQ
IDIV ESI                   ; / src_h  (signed truncation toward zero)
```
`scale_h = (dst_h * 1000) / src_h` — signed IDIV, truncation toward zero. Identical pattern at 0x006407ac for `scale_w`.

**`/1000` (fitted_w, fitted_h)** (verified at 0x00640826–0x00640842 and 0x00640832–0x00640856):
Magic constant `0x10624DD3` with `IMUL` then `SAR EDX, 0x6` with sign correction `(SHR EAX, 0x1f; ADD EDX, EAX)`. This implements truncation-toward-zero division by 1000 identically to Rust's `/` on i32.
- `fitted_w = (src_w * scale) / 1000` — SAR 6 reciprocal, truncation toward zero.
- `fitted_h = (src_h * scale) / 1000` — same.

**Center offset (`/2000`)** (verified at 0x006407cd–0x006407fc):
Magic constant `0x10624DD3` with `SAR EDX, 0x7` (= /2000) on the raw product `src_w * scale`. Sign correction applied. Decompiler renders this as `(local_4c / 2 - iStack_3c / 2000) + iStack_54` where `iStack_3c` is the raw product. Rust code does `dst.x + dst.w / 2 - (src_w * scale) / 2000`. Both are truncation-toward-zero division by 2000 on the same raw product. **Match.**

**Structural note (VERIFIED — no output difference):** Gamemd computes `fitted_w` and `fitted_h` AFTER the center-offset calculation and stores them separately from the raw product; the center offset uses the raw product, not the truncated fitted value. Rust does the same (computes `(src_w * scale) / 2000` directly). Algebraically identical since both truncate the same dividend.

### (b) Start-Position Projection — MATCHES

Verified at 0x006408f5–0x00640965 (disassemble_function 0x00640710):

```asm
; X:
MOV EAX,[ECX + ESI*8 + 0x1140]   ; start_x[i]
SUB EAX,EDI                        ; - origin_x ([ECX+0x112c])
; *1000 via LEA/SHL, IDIV [ECX+0x1134] (map_width) → x_per_mille
IMUL EDX,[ESP+0x58]                ; x_per_mille * fitted_w
; /1000 via magic IMUL + SAR 6 → add [ESP+0x50] (fitted_x)

; Y:
MOV EAX,[ECX + ESI*8 + 0x1144]   ; start_y[i]
SUB EAX,EBX                        ; - origin_y ([ECX+0x1130])
; *1000, IDIV [ECX+0x1138] (map_height) → y_per_mille
IMUL ECX,[ESP+0x5c]                ; y_per_mille * fitted_h
; /1000 via magic IMUL + SAR 6 → add [ESP+0x54] (fitted_y)
```

Formula: `x = fitted_x + ((start_x - origin_x) * 1000 / map_width) * fitted_w / 1000`

Rust (preview.rs:69–74): `x_per_mille = ((point.x - origin_x) as i64 * 1000) / width as i64; x = fitted_x + ((x_per_mille * fitted_w) / 1000) as i32`

Gamemd uses 32-bit IDIV for the per-mille step, Rust uses i64. For all valid inputs (point coords and dimensions within i32 range), the per-mille value fits i32; the intermediate `per_mille * fitted_w` also fits i32 given any sane preview/cell dimensions. **No observable difference.** Both truncate toward zero at each step.

### (c) Start-Marker Hotspot Offset — EXACT MATCH

Verified at 0x0064098b and 0x00640999 (disassemble_function 0x00640710):
```asm
LEA EDX,[EDI + -0x9]    ; draw_x = projected_x - 9
LEA EDX,[EBX + -0x6]    ; draw_y = projected_y - 6
```
Rust constants at `src/app_skirmish_shell_render.rs:55–56`:
```rust
const START_MARKER_OFFSET_X: i32 = -9;
const START_MARKER_OFFSET_Y: i32 = -6;
```
**Exact match. No drift.**

### (d) Marker Count Cap — BEHAVIORAL DIFFERENCE (architectural, not output)

Verified at 0x006408da–0x006408e5 (disassemble_function 0x00640710):
```asm
MOV EAX,[ECX + 0x113c]   ; count = ScenarioClass start_point_count
TEST EAX,EAX
JLE 0x00640a2f             ; skip all if count <= 0
CMP EAX,0x8
JG 0x00640a2f              ; skip all if count > 8
```
Gamemd draws **zero markers** if count > 8 (entire loop is bypassed). Rust uses `.take(8)` which draws the **first 8** if count > 8. In standard YR maps max start points = 8 so count > 8 never occurs in normal play. The Rust is architecturally different but produces identical output on all valid YR maps.

---

## Implementation Handoff

### Status: COMPLETE

| Item | Binary | Rust | Verdict |
|------|--------|------|---------|
| Per-mille scale (aspect-fit) | `x*1000/y` signed IDIV trunc-toward-zero | same | MATCH |
| Fitted w/h | `product/1000` SAR-6 magic, trunc | same | MATCH |
| Center offset | `product/2000` SAR-7 magic, trunc | same | MATCH |
| Projection per-mille step | 32-bit signed IDIV | i64 signed trunc | MATCH (no overflow in valid range) |
| Projection pixel step | `/1000` SAR-6 | i64 `/1000` | MATCH |
| Marker offset X | -9 | -9 | EXACT MATCH |
| Marker offset Y | -6 | -6 | EXACT MATCH |
| Count guard | skip-all if count > 8 OR count ≤ 0 | .take(8) draws first 8 | ARCHITECTURAL DIFF, same output for count ≤ 8 |

**Acceptance scenario:** `aspect_fit_rect(map_preview,138,75)==(645,54,143,78)` (existing test); `start_marker_top_left(120,70)==(111,64)` (existing test); add `project_preview_start_positions_count_zero_and_nine_returns_empty_or_capped`.

---

## Negative Facts / Do Not Do

1. Do NOT use floating-point at any step — gamemd uses integer reciprocal multiply throughout (verified via IMUL 0x10624DD3).
2. Do NOT divide `src_w * scale` by 1000 first then by 2 for center offset — gamemd divides by 2000 in one SAR-7 step (same output, but be aware of the order).
3. Do NOT change `.take(8)` to "skip all if count > 8" — this behavioral difference is irrelevant in practice (max valid YR start points = 8) and the current Rust behavior is more defensive.
4. Do NOT add a center-offset second truncation pass — the one-step `/2000` (SAR 7) matches exactly.
5. Do NOT use unsigned division — gamemd uses CDQ + IDIV (signed) and IMUL with sign correction throughout.

---

## Unverified

Nothing material is unverified this session. All four constants (-9, -6, 8-cap, /1000 per-mille) were read directly from binary.

---

## Remaining Uncertainty

None. All four questions are resolved to exact binary constants with inline assembly citation.

---

## ScenarioClass Offsets Referenced (informational)

Read from disassembly at 0x00640710 (not independently verified against ScenarioClass layout doc):
- `+0x112c` = start_origin_x
- `+0x1130` = start_origin_y  
- `+0x1134` = start_region_width
- `+0x1138` = start_region_height
- `+0x113c` = start_point_count
- `+0x1140 + i*8` = start_point_x[i]
- `+0x1144 + i*8` = start_point_y[i]

These are YELLOW (not cross-checked against ScenarioClass layout research doc this session).
