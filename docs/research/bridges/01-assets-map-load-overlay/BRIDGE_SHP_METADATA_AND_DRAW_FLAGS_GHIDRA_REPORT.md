# Bridge SHP Metadata and Draw Flags - Ghidra Research Report

**Date:** 2026-05-16
**Scope:** Follow-up dump of retail bridge SHP frame metadata and verification of the live `gamemd.exe` draw flags for high bridge body, high bridge body shadow, and bridge railings.
**Primary addresses:** `0x0047F6A0`, `0x0047F510`, `0x004802A0`, `0x00547230`, `0x004AED70`, `0x00490B90`, `0x0069E740`, `0x0069E7E0`, `0x0069E900`.
**Confidence:** HIGH for stock retail SHP metadata, draw-call flags, and draw-call Z parameter formulas. MEDIUM for final railing table values because the table entries are still runtime/theater data and were not live-debugger captured here.
**Active in YR:** Yes. These paths are reached from the normal Tactical draw path in standard Yuri's Revenge.

> **Correction 2026-08-14 — high-bridge body route:** the earlier argument
> mapping below incorrectly treated the body row-Z base as the explicit
> `CC_Draw_Shape` flag-`0x10` gate. The body call pushes that gate as zero, so
> effective flags remain `0x4E00`. Stock body frames are format 3 and therefore
> take `Blitter_selector_extended @ 0x00490E50`, slot `+0x158`, then the
> `0x004990E0` strict Z-read/write leaf. The independent native base is
> `-2 - 15 * (signed cell.level + 4)`, and gradient entry 0 changes the candidate
> by `-1` per full-canvas scanline. Claims of effective `0x4E10`, standard slot
> `+0xC0`, or `cell+0x10E` as the body gate/base are superseded for this route.

## Summary verdict

The stock retail bridge SHPs do not expose a signed-offset bug for current Rust bridge body/railing parsing:

- `bridge.tem/sno/urb/des/lun/ubn`, `bridgb.tem/sno/urb/des/lun/ubn`, and `railbrdg.tem/sno/urb/des/lun/ubn` were dumped from the retail MIX chain.
- All 18 files have 36 frames.
- All 648 dumped frame headers use format byte `3`.
- No dumped frame has a negative signed `x` or `y` offset.
- `bridge.shp`, `bridgb.shp`, and `railbrdg.shp` were not found in the stock chain; the live theater files are the relevant assets.

Therefore Rust's current `u16` storage for `frame_x/frame_y` is not player-visible-dangerous for these stock bridge body, body-shadow, and railing SHPs. It is still structurally different from `gamemd.exe`, because the binary reads those fields as signed shorts. That difference remains a modded/malformed asset risk, not a demonstrated stock YR bridge risk.

The draw-call side confirms the larger parity problem is not asset parsing. `gamemd.exe` draws:

| Visible element | Function | SHP source | Frame source | CC flags at call | Effective selector flags with non-zero Z | Z parameter |
|---|---|---|---|---:|---:|---|
| High bridge body | `CellClass__DrawOverlay_Body @ 0x0047F6A0` | overlay type image pointer, e.g. `BRIDGE.<theater>` / `BRIDGB.<theater>` | `cell+0x11E`, with Latin-square only for states `0` and `9` | `0x4E00` | `0x4E00` (format-3 extended slot `+0x158`; explicit `0x10` gate is zero) | `-2 - 15 * (signed(cell.level) + 4)`, then `-1` per full-canvas row |
| High bridge body shadow | `CellClass__DrawOverlay_Shadow @ 0x0047F510` | same SHP as body | `shp.frame_count / 2 + cell+0x11E` | `0x4601` | `0x4611` | `signed(cell.level) * -15 - 2` |
| Bridge railings | `FUN_00547230 @ 0x00547230` | `DAT_00ABC554`, the theater-loaded railing SHP pointer | `table_frame_1based - 1` | `0x4601` | `0x4611` | `signed(cell.level) * -15 + 0x3A` |

No separate railing-shadow draw call was found in the main railing emitter. Railings themselves use the same `0x4601` shadow/darken-style flag family as body shadows, but against `RAILBRDG.<theater>` frames selected by the railing table.

## Dump artifacts

Full per-frame dump:

- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_SHP_METADATA_DUMP.csv`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_SHP_METADATA_RAW_DUMP.txt`

The CSV has 648 data rows with:

```text
file, source, bytes, canvas_w, canvas_h, frame_count,
frame, x_signed, y_signed, x_raw, y_raw, w, h,
format, extra_9_11, rgb_12_15, unk_16_19, data_offset
```

## Retail SHP summary

| File family | Theater files found | Canvas | Frames | Formats | Negative signed x/y? | Offset ranges |
|---|---|---:|---:|---|---|---|
| `bridge.*` | `tem/sno/urb/des/lun/ubn` | `180x180` | 36 each | `3:36` each | No | `x=16..30`, `y=3..76` |
| `bridgb.*` | `tem/sno/urb/des/lun/ubn` | `253x242` | 36 each | `3:36` each | No | `x=46..107`, `y=31..119` |
| `railbrdg.*` | `tem/sno/urb/des/lun/ubn` | `180x180` | 36 each | `3:36` each | No | `x=15..45`, `y=4..76` |

Stock missing files:

```text
bridge.shp    missing
bridgb.shp    missing
railbrdg.shp  missing
```

The `ubn` files come from a YR expansion archive node identified by hash in the current asset manager dump (`ra2md.mix -> #0xA01A9A03`), while the classic theater files resolve from nested theater archives such as `temperat.mix`, `snow.mix`, `urban.mix`, `desert.mix`, and `lunar.mix`.

## Representative per-frame metadata

The full CSV is the canonical dump. These `*.tem` rows are included here because they cover the concrete body, wood body, and railing frame shapes used by the common Temperate theater.

### `bridge.tem`

| frame | x | y | w | h | fmt |
|---:|---:|---:|---:|---:|---:|
| 0 | 16 | 3 | 148 | 91 | 3 |
| 1 | 16 | 3 | 148 | 91 | 3 |
| 2 | 16 | 3 | 148 | 91 | 3 |
| 3 | 16 | 3 | 148 | 91 | 3 |
| 4 | 16 | 3 | 148 | 91 | 3 |
| 5 | 16 | 3 | 148 | 91 | 3 |
| 6 | 16 | 3 | 148 | 91 | 3 |
| 7 | 18 | 8 | 147 | 86 | 3 |
| 8 | 16 | 6 | 145 | 87 | 3 |
| 9 | 16 | 18 | 148 | 91 | 3 |
| 10 | 16 | 18 | 148 | 91 | 3 |
| 11 | 16 | 18 | 148 | 91 | 3 |
| 12 | 16 | 18 | 148 | 91 | 3 |
| 13 | 16 | 18 | 148 | 91 | 3 |
| 14 | 16 | 18 | 148 | 91 | 3 |
| 15 | 16 | 18 | 148 | 91 | 3 |
| 16 | 16 | 18 | 148 | 91 | 3 |
| 17 | 16 | 23 | 148 | 86 | 3 |
| 18 | 30 | 61 | 150 | 74 | 3 |
| 19 | 30 | 61 | 150 | 74 | 3 |
| 20 | 30 | 61 | 150 | 74 | 3 |
| 21 | 30 | 61 | 150 | 74 | 3 |
| 22 | 30 | 61 | 150 | 74 | 3 |
| 23 | 30 | 61 | 150 | 74 | 3 |
| 24 | 30 | 61 | 150 | 74 | 3 |
| 25 | 30 | 61 | 150 | 74 | 3 |
| 26 | 30 | 61 | 150 | 74 | 3 |
| 27 | 30 | 76 | 150 | 74 | 3 |
| 28 | 30 | 76 | 150 | 74 | 3 |
| 29 | 30 | 76 | 150 | 74 | 3 |
| 30 | 30 | 76 | 150 | 74 | 3 |
| 31 | 30 | 76 | 150 | 74 | 3 |
| 32 | 30 | 76 | 150 | 74 | 3 |
| 33 | 30 | 76 | 150 | 74 | 3 |
| 34 | 30 | 76 | 150 | 74 | 3 |
| 35 | 30 | 76 | 150 | 74 | 3 |

### `bridgb.tem`

| frame | x | y | w | h | fmt |
|---:|---:|---:|---:|---:|---:|
| 0 | 59 | 34 | 148 | 89 | 3 |
| 1 | 59 | 34 | 148 | 89 | 3 |
| 2 | 59 | 34 | 148 | 89 | 3 |
| 3 | 59 | 34 | 148 | 89 | 3 |
| 4 | 59 | 34 | 148 | 89 | 3 |
| 5 | 59 | 34 | 148 | 89 | 3 |
| 6 | 59 | 34 | 148 | 89 | 3 |
| 7 | 56 | 31 | 151 | 92 | 3 |
| 8 | 52 | 34 | 140 | 92 | 3 |
| 9 | 46 | 49 | 148 | 88 | 3 |
| 10 | 46 | 49 | 148 | 88 | 3 |
| 11 | 47 | 49 | 147 | 88 | 3 |
| 12 | 46 | 48 | 148 | 89 | 3 |
| 13 | 46 | 49 | 148 | 88 | 3 |
| 14 | 46 | 48 | 148 | 89 | 3 |
| 15 | 46 | 48 | 148 | 89 | 3 |
| 16 | 58 | 48 | 136 | 91 | 3 |
| 17 | 46 | 39 | 149 | 98 | 3 |
| 18 | 98 | 99 | 136 | 67 | 3 |
| 19 | 98 | 99 | 136 | 67 | 3 |
| 20 | 98 | 99 | 136 | 67 | 3 |
| 21 | 98 | 99 | 136 | 67 | 3 |
| 22 | 98 | 99 | 136 | 67 | 3 |
| 23 | 98 | 99 | 136 | 67 | 3 |
| 24 | 98 | 99 | 136 | 67 | 3 |
| 25 | 107 | 99 | 127 | 67 | 3 |
| 26 | 98 | 99 | 127 | 63 | 3 |
| 27 | 93 | 119 | 136 | 67 | 3 |
| 28 | 93 | 119 | 136 | 67 | 3 |
| 29 | 93 | 119 | 136 | 67 | 3 |
| 30 | 93 | 119 | 136 | 67 | 3 |
| 31 | 93 | 119 | 136 | 67 | 3 |
| 32 | 93 | 119 | 136 | 67 | 3 |
| 33 | 93 | 119 | 136 | 67 | 3 |
| 34 | 102 | 119 | 127 | 63 | 3 |
| 35 | 93 | 119 | 127 | 67 | 3 |

### `railbrdg.tem`

| frame | x | y | w | h | fmt |
|---:|---:|---:|---:|---:|---:|
| 0 | 15 | 4 | 150 | 93 | 3 |
| 1 | 15 | 4 | 150 | 93 | 3 |
| 2 | 15 | 4 | 150 | 93 | 3 |
| 3 | 15 | 4 | 150 | 93 | 3 |
| 4 | 15 | 7 | 150 | 90 | 3 |
| 5 | 15 | 4 | 150 | 93 | 3 |
| 6 | 15 | 9 | 150 | 86 | 3 |
| 7 | 24 | 15 | 141 | 82 | 3 |
| 8 | 15 | 9 | 145 | 85 | 3 |
| 9 | 15 | 19 | 150 | 93 | 3 |
| 10 | 15 | 19 | 150 | 93 | 3 |
| 11 | 15 | 19 | 150 | 93 | 3 |
| 12 | 15 | 19 | 150 | 93 | 3 |
| 13 | 15 | 19 | 150 | 93 | 3 |
| 14 | 15 | 19 | 150 | 93 | 3 |
| 15 | 15 | 19 | 150 | 93 | 3 |
| 16 | 31 | 19 | 134 | 87 | 3 |
| 17 | 15 | 20 | 139 | 92 | 3 |
| 18 | 30 | 61 | 150 | 74 | 3 |
| 19 | 30 | 61 | 150 | 74 | 3 |
| 20 | 30 | 61 | 150 | 74 | 3 |
| 21 | 30 | 61 | 150 | 74 | 3 |
| 22 | 30 | 61 | 150 | 74 | 3 |
| 23 | 30 | 61 | 150 | 74 | 3 |
| 24 | 30 | 61 | 150 | 74 | 3 |
| 25 | 39 | 67 | 141 | 68 | 3 |
| 26 | 34 | 65 | 132 | 62 | 3 |
| 27 | 30 | 76 | 150 | 74 | 3 |
| 28 | 30 | 76 | 150 | 74 | 3 |
| 29 | 30 | 76 | 150 | 74 | 3 |
| 30 | 30 | 76 | 150 | 74 | 3 |
| 31 | 30 | 76 | 150 | 74 | 3 |
| 32 | 30 | 76 | 150 | 74 | 3 |
| 33 | 30 | 76 | 150 | 74 | 3 |
| 34 | 45 | 76 | 135 | 69 | 3 |
| 35 | 30 | 76 | 137 | 74 | 3 |

## Verified binary draw-call facts

### High bridge body

`CellClass__DrawOverlay_Body @ 0x0047F6A0` resolves the overlay type from `cell+0x44`, calls the type's vtable `+0x9C` image getter, then enters a high-bridge special branch when `cell+0x140 & 0x80` is set.

The high-bridge body frame selector is:

```text
state = uint8(cell+0x11E)
if state == 0 or state == 9:
    state += g_OverlayVarietyLatinSquare[((cell.y & 3) << 2) | (cell.x & 3)]
frame = state
```

The body call is:

```text
CC_Draw_Shape(
    shp,
    frame,
    screen_pos,
    clip_rect,
    0x4E00,
    0,
    (signed(cell.level) + ((cell.flags >> 7) & 1) * 4) * -15 - 2,
    0,
    signed16(cell+0x10E),
    0, 0, 0, 0, 0
)
```

`CC_Draw_Shape @ 0x004AED70` ORs `0x10` into the flags when the Z parameter is non-zero. For high bridge body cells this makes the selector see `0x4E10`, not bare `0x4E00`.

`Blitter_selector @ 0x00490B90` then takes the `flags & 0x10` branch, the `flags & 0x4000` branch, and the `flags & 0x800` branch. That selects the blitter slot at selector-table offset `+0xC0`.

**Evidence:** `0x0047F6A0`, `0x004AED70`, `0x00490B90`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### High bridge body shadow

`CellClass__DrawOverlay_Shadow @ 0x0047F510` resolves the same overlay SHP pointer, computes the same base draw offset, then uses the second half of the SHP:

```text
shadow_frame = signed16(shp+0x06) / 2 + uint8(cell+0x11E)
```

For high-bridge cells with state byte `9..=17`, it applies an additional position shift before drawing:

```text
x -= 15
y += 7
```

The shadow call is:

```text
CC_Draw_Shape(
    shp,
    shadow_frame,
    screen_pos,
    clip_rect,
    0x4601,
    0,
    signed(cell.level) * -15 - 2,
    0,
    1000,
    0, 0, 0, 0, 0
)
```

Because the Z parameter is non-zero, `CC_Draw_Shape` makes the selector see `0x4611`. `Blitter_selector @ 0x00490B90` takes the `flags & 0x10` branch, then `flags & 0x4000`, but not `flags & 0x800`, selecting selector-table offset `+0x58`.

**Evidence:** `0x0047F510`, `0x004AED70`, `0x00490B90`.
**Confidence:** HIGH.
**Active in YR:** Yes.

### Bridge railings

`FUN_004802A0 @ 0x004802A0` is the railing trampoline. It selects the cell's current `IsoTileTypeClass` from `cell+0x38`, reads the cell sub-tile from `cell+0x11A`, and only calls the railing emitter when `IsoTileTypeClass+0x2E1` is non-zero.

The trampoline passes this Z value into the railing emitter:

```text
signed(cell.level) * -15 + 0x3A
```

`FUN_00547230 @ 0x00547230` uses `IsoTileTypeClass+0x294` to select a railing-table entry. The entry stride is 16 bytes:

```text
entry+0x00: shp_frame_1based
entry+0x04: required_sub_tile
entry+0x08: dx
entry+0x0C: dy
```

It skips drawing if the required sub-tile does not match the current sub-tile, or if `shp_frame_1based == 0`.

The railing draw call is:

```text
screen_x = input_x + table_dx + 0x1E + g_RadarViewportOffsetX - clip_x
screen_y = input_y + table_dy + 0x0F + g_RadarViewportOffsetY - clip_y

CC_Draw_Shape(
    DAT_00ABC554,
    shp_frame_1based - 1,
    screen_pos,
    clip_rect,
    0x4601,
    0,
    signed(cell.level) * -15 + 0x3A,
    0,
    1000,
    0, 0, 0, 0, 0
)
```

As with body shadows, the non-zero Z parameter makes `CC_Draw_Shape` select with effective flags `0x4611`.

**Evidence:** `0x004802A0`, `0x00547230`, `0x004AED70`, `0x00490B90`.
**Confidence:** HIGH for formula/flags/source pointer; MEDIUM for final table values until runtime table capture.
**Active in YR:** Yes.

### Railing shadows

No separate railing-shadow function or second railing-shadow draw call was found in the live railing path. The main railing draw itself uses flags `0x4601`, the same call-time flag value as body shadows, but draws `DAT_00ABC554` railing frames rather than using the second half of the body SHP.

This means "railing shadow" is not a separate asset path equivalent to body shadow frames in the verified live code. Any darkening/shadow-like appearance belongs to the `0x4601` blitter semantics for the railing call itself.

**Evidence:** `0x00547230`.
**Confidence:** HIGH for no separate call inside the main emitter; MEDIUM globally because this did not exhaustively audit every non-standard/editor/legacy path.
**Active in YR:** Yes for the standard Tactical draw path.

## Rust comparison

### Stock SHP frame offsets

Rust stores `ShpFrame.frame_x` and `frame_y` as `u16` in `src/assets/shp_file.rs:74` and reads them with `read_u16_le` at `src/assets/shp_file.rs:135`.

The binary reads the same fields as signed shorts in `SHP_frame_rect_getter @ 0x0069E7E0`. For stock retail bridge body, body-shadow, and railing SHPs, the dump shows no negative values. So this is a general SHP parser fidelity gap, but not a currently demonstrated stock bridge rendering bug.

### Frame format byte

All dumped bridge and railing frames are format byte `3`. Rust recognizes `FORMAT_RLE_ZERO = 3` in `src/assets/shp_file.rs:50` and routes it to `decode_rle_frame`.

The binary's `SHP_frame_flag_check @ 0x0069E900` checks `(format & 0x02) >> 1`, so format `3` uses the extended SHP blitter path in `CC_Draw_Shape`. Rust decodes format `3` into RGBA ahead of time instead of preserving the indexed frame for the binary-style blitter. That remains a pixel-parity risk around shadow/darken and Z behavior, not a header parsing failure.

### Body/shadow atlas behavior

Rust correctly resolves body image names through `resolve_overlay_image_id` and `overlay_shp_candidates` in `src/render/bridge_atlas.rs:166`.

Confirmed differences:

- Rust adds numeric-suffix fallback aliases in `src/render/bridge_atlas.rs:174` and `src/render/bridge_atlas.rs:401`; the binary does not.
- Rust clamps requested body/shadow frames in `src/render/bridge_atlas.rs:202`; the binary frame helpers do not clamp to a nearby bridge frame.
- Rust casts frame offsets to `u32` for atlas placement in `src/render/bridge_atlas.rs:228`; safe for stock dumped bridge frames, unsafe for any future negative-offset SHP.
- Rust creates an all-zero bridge depth texture in `src/render/bridge_atlas.rs:342`, while the binary draws through Z-capable SHP blitters selected by `0x4E10` and `0x4611`.

### Railing atlas behavior

Rust loads `railbrdg.<theater>` first in `src/render/bridge_railing_atlas.rs:100`, which matches the verified retail assets.

Confirmed differences:

- `CONCRETE_RAILING_VALUES` and `WOOD_RAILING_VALUES` remain all-zero placeholders in `src/render/bridge_railing_atlas.rs:64` and `src/render/bridge_railing_atlas.rs:79`, so the builder has no real railing table data to emit.
- Rust packs railing frames as normal RGBA sprites and casts frame offsets to `u32` at `src/render/bridge_railing_atlas.rs:190`; safe for stock dumped `railbrdg.*`, but structurally different from signed binary access.
- The binary railing call uses `0x4601` plus non-zero Z, making effective selector flags `0x4611`. Rust's current rendering path needs a real equivalent for that shadow/darken/Z-tested blitter behavior before enabling railings for pixel parity.

## Confirmed parity gaps

1. **Bridge body parsing for stock files is structurally sufficient.** Stock body/shadow frame offsets are non-negative, every frame uses format `3`, and the files have the expected 36-frame split.
2. **The signed-offset parser gap is not currently a stock bridge bug.** It remains a general parser correctness issue and a mod risk.
3. **Draw flags and blitter behavior are still the primary bridge rendering gap.** Body uses `0x4E10` after Z flag insertion; shadow and railings use `0x4611`.
4. **Railing table values are still missing.** The retail SHP dump proves the source SHP is available and parseable, but not which table entries choose which frames and offsets at runtime.
5. **No separate railing-shadow asset path was verified.** Do not search for or implement a second railing shadow SHP unless a later live path proves one exists.

## Implementation implications

Do not prioritize "fix bridge SHP asset parsing" as the next bridge rendering task for stock YR. For current stock assets, the loader can read the needed files and the unsigned offsets do not change visible bridge pixels.

The next renderer work should instead be gated on:

1. Capturing or reconstructing the runtime railing tables.
2. Implementing a renderer path equivalent to the binary's `0x4601` shadow/darken behavior with Z testing.
3. Re-enabling body shadows only through that correct blitter-equivalent path.
4. Drawing railings through the same `0x4601`-equivalent path, using `z = level * -15 + 0x3A`.
5. Keeping body draw behavior separate: body is `0x4E00` at call, effective `0x4E10` after non-zero Z insertion, with bridge height bonus in Z only.

## Remaining open questions

1. Runtime railing table values remain uncaptured. Static decompilation proves the table shape and draw call, not the final theater-loaded entry values.
2. Exact pixel math inside the `0x4601` and `0x4E10` blitters still needs either blitter decompilation or image-diff validation.
3. The `ubn` source archive is identified by hash in the current asset manager dump rather than a resolved nested name. This does not affect frame metadata, but a better dictionary entry would improve audit readability.
4. General SHP parser signed offsets should still be fixed or guarded eventually, because other SHPs or mods can use negative frame offsets even though stock bridge SHPs do not.

## Sources

- Ghidra: `CellClass__DrawOverlay_Body @ 0x0047F6A0`
- Ghidra: `CellClass__DrawOverlay_Shadow @ 0x0047F510`
- Ghidra: `FUN_004802A0 @ 0x004802A0`
- Ghidra: `FUN_00547230 @ 0x00547230`
- Ghidra: `CC_Draw_Shape @ 0x004AED70`
- Ghidra: `Blitter_selector @ 0x00490B90`
- Ghidra: `SHP_frame_data_getter @ 0x0069E740`
- Ghidra: `SHP_frame_rect_getter @ 0x0069E7E0`
- Ghidra: `SHP_frame_flag_check @ 0x0069E900`
- Retail asset dump: `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_SHP_METADATA_DUMP.csv`
- Retail asset dump: `C:/Users/enok/Documents/ra2-rust-game-docs/BRIDGE_SHP_METADATA_RAW_DUMP.txt`
- Rust comparison: `src/assets/shp_file.rs`
- Rust comparison: `src/render/bridge_atlas.rs`
- Rust comparison: `src/render/bridge_railing_atlas.rs`
- Rust comparison: `src/app_render/draw_passes.rs`
