# SHP RLE-Zero Pixel-Value Certification — Ghidra Research Report

**Date:** 2026-07-19
**Status:** VERIFIED (live Ghidra session, gamemd.exe) + corpus-proven
**Goal:** upgrade SHP decoded pixel VALUES (formats 2/3) from
ratchet-only to a citable certification, closing the largest
"UNVERIFIED-pending-instrument" row in `tests/retail_goldens`.

## Verdict

**Our `decode_rle_frame` / `decode_length_prefixed_frame`
(src/assets/shp_decode.rs) produce pixel-for-pixel identical output to the
original engine's consumption for every format-2/3 frame in the retail
corpus.** Certified by (a) the decode grammar verified from the binary's
blitters, plus (b) a corpus-wide no-under-run proof
(`certify_shp_rle_row_exactness` in `tests/retail_goldens`) covering the one
input class where the two implementations could diverge. No emulation rig was
needed — grammar equivalence + input-domain proof is stronger than sampled
vectors.

## 1. Native grammar (verified)

`Blitter_Opaque_RLE_Remap @ 0x004978C0` (verified via
`decompile_function 0x004978C0`) — the format-3 row consumer. Inner loop:

```
b = *src++
if b == 0:  count = *src++ ; advance dst/z by count   (transparent run)
else:       *dst = remap[b] ; dst++                    (one literal pixel)
```

- Nonzero byte → exactly one literal pixel whose palette index IS the byte.
- `0x00, count` → `count` transparent pixels (skipped, never written).
- **No back-reference opcode exists.** The row loop is width-driven
  (`param_4` counts down pixels; no row-length bound inside the blitter).
- The same grammar (with leading-clip handling) appears in the function's
  clip preamble.

`Extended_SHP_blitter @ 0x00437A10` (verified via
`decompile_function 0x00437A10`) — the row walker for formats 2/3
(dispatched from `CC_Draw_Shape @ 0x004AED70` when
`SHP_frame_flag_check` reports bit 1 of the format byte, i.e. formats 2/3):

- Vertical-clip skip AND per-row advance are both
  `row_ptr = (u8*)row_ptr + *(u16*)row_ptr` — the u16 prefix **includes its
  own 2 bytes**.
- The row blitter receives `row_ptr + 1` (as u16*, = prefix + 2 bytes) — data
  starts immediately after the prefix.

This matches src/assets/shp_decode.rs framing exactly (u16 self-inclusive
prefix; data = prefix − 2 bytes).

## 2. Divergence analysis (where could outputs differ?)

| Input class | Native (width-driven, no length bound) | Ours (length-bound) | Identical? |
|---|---|---|---|
| Row yields exactly `width` pixels in exactly its bytes | consume all, stop | consume all, stop | YES |
| Row has tail bytes after `width` pixels reached | stops at width, walker jumps by prefix | stops at width, skips to `line_end` | YES |
| Final zero-run overshoots `width` | skips ≤ width remaining (dst clamped by row extent), stops | clamps `fill_count` to width | YES |
| `0x00 0x00` no-op run | consumes 2 bytes, emits nothing | consumes 2 bytes, emits nothing (`fill_count` 0) | YES |
| **Row UNDER-RUNS (fewer than `width` pixels in its declared bytes)** | **reads past the row into the next row's prefix bytes** | **pads with zeros** | **NO — divergence** |
| Format 2 row with fewer than `width` data bytes | reads past row | pads with zeros | NO — divergence |

So value identity on retail data reduces to one corpus property: **no
format-2/3 row under-runs.**

## 3. Corpus proof

`certify_shp_rle_row_exactness` (tests/retail_goldens/certify_structural.rs)
replays the native width-driven walk over the raw bytes of every format-2/3
row of every frame of all 2,450 retail SHPs (measured 2026-07-19):

- **Zero under-runs** (test green; an under-run is an assertion failure).
- Zero `0x00 0x00` runs anywhere in retail data.
- 2,369,733 rows end with an overshooting final zero-run — the benign class
  (identical output both sides; recorded, not failed).

Formats 0/1 (raw) are byte-copies on both sides — value-identical by
construction. Together with the grammar verification this certifies SHP
decoded pixel values across the entire retail corpus.

## 4. Corrections to earlier claims

- `ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md` §4.3 calls format 3
  "RLE + **back-reference**" — WRONG: the verified blitter grammar has no
  back-reference opcode (literal + zero-run only). Patched in place with this
  citation.

## Confidence axes

| Claim | Content | Identity | Binding |
|---|---|---|---|
| Format-3 grammar (literal / zero-run, no back-ref) | HIGH (decompiled blitter inner loop) | HIGH (`Blitter_Opaque_RLE_Remap @ 0x004978C0`) | MEDIUM-HIGH (one of the RLE blitter family verified; siblings assumed same grammar — they share the constructor family at 0x0049AA00+) |
| Row framing (self-inclusive u16, data after prefix) | HIGH (decompiled walker) | HIGH (`Extended_SHP_blitter @ 0x00437A10`) | HIGH (both the clip-skip and the row-advance use the same expression) |
| Dispatch (formats 2/3 → extended path) | HIGH | HIGH (`CC_Draw_Shape @ 0x004AED70`, `SHP_frame_flag_check @ 0x0069E900` per prior report) | HIGH |
| No-under-run corpus property | machine-proven (named test, full corpus) | — | — |
