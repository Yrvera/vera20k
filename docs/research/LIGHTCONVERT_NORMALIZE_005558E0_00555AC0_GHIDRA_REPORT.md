# LightConvert Normalize `0x005558E0` / `0x00555AC0` -- Ghidra Report

**Target:** `LIGHTCONVERT_NORMALIZE_005558E0_00555AC0`  
**Status:** COMPLETE for the requested helper slice.  
**Active in YR:** Yes. `0x005558E0` is called by active cell lighting compute `0x00484180`; `0x00555AC0` is called by active cache compare/lookup paths `0x00483E30` and `0x00544E70`.

## Target Question

What exact clamp, normalization, scaling, and cache-key quantization rules do `0x005558E0` and `0x00555AC0` apply to map-lighting RGB values before `LightConvertClass` cache lookup and cell light-field storage?

## Non-goals

- Do not re-investigate lamp falloff, scenario `[Lighting]` field parsing, dirty-cell scheduling, or `LightConvertClass__Constructor`.
- Do not rename, type, comment, or otherwise mutate Ghidra.
- Do not edit Rust, INI, existing docs, or `.swarm-claims.md`.
- Do not treat Lightning Storm superweapon lighting as ordinary map ambience.

## Evidence Needed To Mark COMPLETE

- Decompile both target helpers.
- Check assembly context for clamp/mask branches, FPU conversion calls, scale multiply, and cache-call argument flow.
- Verify constants used by the FPU math.
- Verify the integer conversion helper enough to state signedness and truncation/rounding.
- Tie both helpers back to active standard-YR callers.

## Stop Conditions

- Stop after helper arithmetic, direct call sites, and cache-key output semantics are known.
- Stop before draining `LightConvertClass__Constructor`, blitter tables, or downstream palette generation.
- Stop if a missing Ghidra boundary requires mutation. No mutation was needed.

## Verified Findings

### 1. `0x005558E0` normalizes cell-light RGB and a 16.16 brightness scale

`0x005558E0` decompiles as:

```c
void __fastcall FUN_005558e0(int *scale16, int *additive, uint *red, uint *green, uint *blue)
```

`0x00484180` calls it after summing scenario ambience, lamp contributions, height, and ground. The call site at `0x004845C8..0x004845CF` passes the scale/additive pointers through `ECX`/`EDX` and RGB pointers on the stack. Immediately after the call, `0x004845D4..0x004845DA` multiplies the bottom ambient value by `*scale16` and arithmetic-shifts right 16.

Active in YR: Yes. `0x00484180` is the active per-cell lighting compute path used by `0x00483E30`.

### 2. `0x005558E0` first signed-low-clamps RGB to zero, then high-clamps to 2000

For each RGB input, the helper uses the signed mask idiom:

- `TEST value,value`
- `SETLE`
- `DEC`
- `AND value,mask`

This makes any `<= 0` value become `0`. It then clamps values greater than `1999` to `2000`.

Evidence:

- Decompile `0x005558E0`: first writes `*param_3 = *param_3 & ((int)*param_3 < 1) - 1`, same for green/blue, then applies `if (1999 < (int)value) value = 2000`.
- Assembly context `0x005558EE..0x005558F8`: `TEST`, `SETLE`, `DEC`, and `AND` pattern begins the red low clamp.

Active in YR: Yes.

### 3. Neutral RGB bypasses normalization but still high-clamps additive intensity

After the RGB clamp, `0x005558E0` initializes `*scale16 = 0x10000`. If red, green, and blue are exactly `1000`, it skips RGB normalization and leaves additive intensity unchanged until the final high clamp:

```c
if (red == 1000 && green == 1000 && blue == 1000) goto final_additive_high_clamp;
```

At the final clamp, additive values greater than `1999` become `2000`. There is no low clamp for additive in this helper.

Active in YR: Yes.

### 4. Non-neutral RGB becomes normalized RGB plus a 16.16 scale

For non-neutral clamped RGB, the helper selects the maximum channel and computes:

```text
scale16 = Math__ftol(max_channel * 65536.0 * 0.001)
```

The constants are verified from Ghidra memory:

- `0x007ED088` bytes `00 00 00 00 00 00 F0 40` = `65536.0`
- `0x007E3818` bytes `FC A9 F1 D2 4D 62 50 3F` = `0.001`

If `scale16 < 66`, the helper resets to neutral:

```text
scale16 = 0x10000
red = green = blue = 1000
additive = 0
```

Otherwise:

- the max channel is set to `1000`;
- the two non-max channels are normalized to approximately `channel * 1000 / max_channel`;
- additive intensity is scaled by `(*scale16 * additive) >> 16`;
- additive is high-clamped to `2000`.

Signedness/truncation notes:

- `Math__ftol @ 0x007C5F00` uses `FISTP qword` under control word `0x0E7F`; that control word has x87 rounding-control bits set for truncate-toward-zero. Assembly `0x007C5F03..0x007C5F32` either uses the existing control word or loads `DAT_00822D80` before `FISTP`.
- Positive FPU conversions here therefore truncate toward zero. Example: `max=1` gives `65.536 -> 65`, which is `< 66` and triggers the neutral reset; `max=2` gives `131.072 -> 131`.
- The additive fixed-point scale uses integer `IMUL` then `SAR 0x10` at `0x00555A93..0x00555A9C`. For negative additive values, `SAR` is arithmetic and rounds negative fractional results downward, not toward zero.

Active in YR: Yes.

### 5. Max-channel tie behavior is deterministic

The decompiled comparisons and branch layout make ties deterministic:

- red wins ties against green or blue when red is tied for maximum;
- if red is lower and green ties blue for maximum, green wins;
- the all-`1000` neutral case bypasses this branch entirely.

This matters only around exact equal-channel non-neutral values.

Active in YR: Yes.

### 6. Red-max normalization differs slightly from green/blue-max normalization

The red-max branch keeps the floating denominator on the FPU stack:

- `0x00555983..0x00555991`: computes `red * 65536.0 * 0.001`, duplicates it, and converts one copy to `scale16`.
- `0x005559A8..0x005559C3`: normalizes green/blue with `FDIV ST0,ST1` against the retained floating denominator.

The green-max and blue-max branches store `scale16` first, then use integer-denominator `FIDIV dword ptr [EBP]`:

- green-max evidence: `0x00555A24..0x00555A41`
- blue-max evidence: `0x00555A6B..0x00555A88`

This can differ by one unit near conversion boundaries because the red-max branch divides by the untruncated floating scale while the other branches divide by integer `scale16`.

Active in YR: Yes.

### 7. `0x00555AC0` clamps cache-key RGB to 0..1000, then quantizes by detail setting

`0x00555AC0` decompiles as:

```c
void __fastcall FUN_00555ac0(uint *red, uint *green, uint *blue)
```

It first high-clamps each component:

```text
if component > 999: component = 1000
```

Then it signed-low-clamps:

```text
if component <= 0: component = 0
```

Finally it masks low bits based on `g_ExtraAnimationsEnabled` / the detail-level global at `0x00A8EB78`:

| Global value | Mask | Quantization |
|---:|---:|---|
| `0` | `0xFFFFFF80` | multiples of 128 |
| `1` | `0xFFFFFFC0` | multiples of 64 |
| `2` | `0xFFFFFFE0` | multiples of 32 |
| other | no mask in this helper | no verified normal YR default path |

Evidence:

- Decompile `0x00555AC0`.
- Assembly `0x00555AC4..0x00555B21` performs high clamps and signed low clamps.
- Assembly `0x00555B23..0x00555B7B` dispatches on `0x00A8EB78` and applies the three masks.

Active in YR: Yes. `0x00483E30` calls it before comparing an existing `CellClass+0x34` cache pointer; `0x00544E70` calls it before linear cache lookup/create for non-neutral input.

### 8. `0x00544E70` special-cases raw neutral RGB before `0x00555AC0`

`0x00544E70` checks for raw `(1000,1000,1000)` and returns cache entry 0 when the cache is initialized before calling `0x00555AC0`.

Evidence:

- Decompile `0x00544E70`: the neutral/cache-count guard precedes `FUN_00555ac0(&param_3)`.
- Assembly `0x00544E94..0x00544EBD` performs the neutral fast path.
- Assembly `0x00544EC0..0x00544ECD` passes `&param_1`, `&param_2`, and `&param_3` into `0x00555AC0`.

Important edge case: if `0x00555AC0` is actually invoked on `1000`, the mask can reduce it to `896`, `960`, or `992` at detail levels `0`, `1`, or `2`. The raw-neutral fast path is what preserves the default full-bright singleton in `0x00544E70`.

Active in YR: Yes.

## Edge Cases

| Input case | `0x005558E0` result | `0x00555AC0` result | Active in YR |
|---|---|---|---|
| RGB negative | channel low-clamps to `0` before high clamp | channel low-clamps to `0` after high clamp | Yes |
| RGB over `2000` before `0x005558E0` | clamps to `2000`, then may normalize with scale above `0x10000` | not applicable to this helper | Yes |
| RGB over `1000` before `0x00555AC0` | not applicable to this helper | clamps to `1000`, then may mask to lower multiple | Yes |
| RGB all `1000` in `0x005558E0` | scale stays `0x10000`; additive only high-clamped | not applicable | Yes |
| RGB all `0` in `0x005558E0` | `scale16=0`, threshold fails, RGB resets to `1000/1000/1000`, additive becomes `0` | not applicable | Yes |
| raw `(1000,1000,1000)` in `0x00544E70` | not applicable | bypasses `0x00555AC0`, returns default cache entry if initialized | Yes |
| `max_channel=1` in `0x005558E0` | `scale16=65`, threshold fails, neutral reset | not applicable | Yes, if such values are produced |
| negative additive in scaling branch | `IMUL` + `SAR 16`; no low clamp, possible negative result | not applicable | Yes, negative lamps can feed additive |

## Implementation Handoff

| Verified behavior | Evidence | Rust-facing effect | Do not do |
|---|---|---|---|
| Model two stages: `0x005558E0` light normalization and `0x00555AC0` cache-key quantization. | `0x004845CF`, `0x00483F0B`, `0x00544ECD` | Keep post-sum RGB normalization separate from LightConvert cache-key rounding. | Do not use one `clamp(0.0, 2.0)` RGB float as both brightness and cache key. |
| `0x005558E0` clamps RGB to `0..2000`, computes `scale16=trunc(max*65.536)`, normalizes max to `1000`, and scales additive. | `0x005558E0` decompile; assembly `0x00555983..0x00555A9C`; constants at `0x007ED088` and `0x007E3818`; `Math__ftol @ 0x007C5F00` | Implement integer milli-unit lighting plus a 16.16 scale output if renderer parity needs cell fields equivalent to `+0x104/+0x108/+0x110..+0x114`. | Do not clamp overbright RGB to `2.0` and stop; gamemd carries overbright via `scale16`. |
| `0x00555AC0` clamps to `0..1000` and masks low bits by detail setting. | `0x00555AC0`; assembly `0x00555B23..0x00555B7B` | Cache LightConvert/profile entries by quantized integer RGB triple. | Do not key cache by raw f32, source identity, cell coordinate, height, or unquantized RGB. |
| Raw neutral `(1000,1000,1000)` is special in `0x00544E70`. | `0x00544E94..0x00544EBD` | Seed and reuse a default full-bright profile separately from non-neutral quantized entries. | Do not assume `00555AC0(1000)` preserves `1000`; the neutral fast path is separate. |
| `Math__ftol` truncates toward zero under control word `0x0E7F`; additive fixed-point scaling uses arithmetic shift. | `0x007C5F00` decompile; assembly `0x007C5F03..0x007C5F32`; memory `0x00822D80 = 0x0E7F` | Unit tests should include `max=1`, `max=2`, and negative additive scaling edge cases. | Do not use Rust `round()` or floating `as i32` blindly without matching these two conversion sites. |

Suggested focused tests:

- `light_normalize_max_one_resets_to_neutral`
- `light_normalize_overbright_red_produces_scale_and_normalized_rgb`
- `light_normalize_negative_additive_uses_arithmetic_shift`
- `lightconvert_key_quantizes_by_detail_level`
- `lightconvert_raw_neutral_uses_default_profile_before_quantization`

## Negative Facts / Do Not Do

- Do not treat `0x005558E0` and `0x00555AC0` as the same operation. One normalizes lighting into scale/RGB; the other quantizes a cache key.
- Do not expect `0x00555AC0` to preserve `1000` when masking runs. Detail `2` turns `1000` into `992`; the default raw-neutral fast path lives in `0x00544E70`.
- Do not low-clamp additive intensity inside `0x005558E0`; the helper only high-clamps it to `2000`, except for the low-scale neutral reset path that writes `0`.
- Do not use per-light normalization. `0x005558E0` is called after all point-light and scenario contributions are summed by `0x00484180`.
- Do not model negative RGB channels as wrapping unsigned values. The helper comparisons are signed for low clamp.

## Remaining Uncertainty

- `LightConvertClass__Constructor @ 0x00555DA0` palette generation remains out of scope. This report verifies the keys and normalized inputs handed to it, not the palette tables it builds.
- The global at `0x00A8EB78` is named `g_ExtraAnimationsEnabled` by current Ghidra labeling, but prior lighting reports tie the same field to `[Options] DetailLevel`. This report verifies the numeric mask behavior, not the final semantic name.
- The exact visual meaning of cell fields beyond the helper outputs still belongs to the broader render-consumer investigations.

## Sources

- Ghidra decompiles: `0x005558E0`, `0x00555AC0`, `0x00484180`, `0x00483E30`, `0x00544E70`, `0x007C5F00`.
- Ghidra assembly contexts: `0x004845C8..0x004845DA`, `0x00483EB0..0x00483F68`, `0x00544E94..0x00544ECD`, `0x005558EE..0x00555AB7`, `0x00555AC4..0x00555B7B`, `0x007C5F03..0x007C5F32`.
- Ghidra memory reads: `0x007ED088`, `0x007E3818`, `0x00822D80`.
- Prior context checked only for scope alignment: `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`, `MAP_LIGHTCONVERT_CACHE_00483E30_00544E70_GHIDRA_REPORT.md`.
