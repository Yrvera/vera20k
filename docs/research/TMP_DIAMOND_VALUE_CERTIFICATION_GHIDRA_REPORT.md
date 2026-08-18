# TMP Diamond Pixel-Value Certification — Ghidra Research Report

**Date:** 2026-07-19
**Status:** VERIFIED (live Ghidra session, gamemd.exe) + corpus-proven
**Goal:** upgrade TMP tile pixel VALUES from ratchet-only to a citable
certification (companion to `SHP_RLE_ZERO_VALUE_CERTIFICATION_GHIDRA_REPORT.md`).

## Verdict

**Our `parse_tile_cell` / `unpack_diamond` / extra-data composition
(src/assets/tmp_decode.rs) produce pixel-for-pixel identical output to what
the original engine's tile blitter consumes and composites, for every tile of
every retail TMP.** Certified by: (a) the native diamond geometry read out of
the binary's embedded template — bit-identical to our formula; (b) a
corpus-wide proof that the header-stored section offsets and tile origins the
native blitter uses equal the sequential/computed values our decoder assumes;
(c) a corpus-wide proof that the opposite extra-data composition orders
(native draws extra OVER the diamond; we composite it BEHIND) never touch the
same nonzero pixel on retail data.

Depth (ZData) BYTES are covered by (a)+(b) — same diamond layout at the
stored offset. Depth *composition semantics* differ mechanically (see §5) and
remain a renderer-side note, not a parser gap.

## 1. Native diamond geometry (verified from binary data)

`TMP_TileBlitter @ 0x00547CF0` (verified via `decompile_function 0x00547CF0`)
lazily builds its scanline tables from a template embedded in the binary:

- Per-row source-offset table at `0x007EC450` (30 × u32, read via
  `read_memory 0x007EC450`): `0,4,12,24,40,60,84,112,144,180,220,264,312,364,
  420,480,536,588,636,680,720,756,788,816,840,860,876,888,896,900` —
  consecutive diffs are exactly the row widths `4,8,…,56,60,56,…,8,4`
  (29 rows, 900 bytes total).
- ASCII-art diamond template at `0x007EC4C8` (29 rows × 60 chars, read via
  `read_memory 0x007EC4C8`, first 4 rows verified byte-level): row j has
  `(60 − w)/2` leading spaces then `w` × `0xDB` block chars — the exact
  centering our `unpack_diamond` computes.
- Diamond pixels start at cell offset `+52` (`piVar4 + 0xd` dwords in the
  decompile) — same as our `TILE_HEADER_SIZE`.
- The blit loop runs 29 rows (`0x1D`); our 30-iteration loop leaves row 29
  empty (width underflows to 0) — same 900 bytes, same rows.

**Conclusion: native template == our formula, bit-for-bit.**

## 2. Section location: stored offsets vs our sequential walk

The native blitter does NOT walk sections sequentially — it uses offsets
stored in the 52-byte cell header (all from the `0x00547CF0` decompile):

| Field | Native source | Our assumption |
|---|---|---|
| ZData ptr | cell + u32@`+0x0C` | cell + 52 + 900 |
| ExtraData ptr | cell + u32@`+0x08` | cell + 52 + 900 (+900 if ZData) |
| ExtraZData ptr | cell + u32@`+0x10` | extra + extra_w×extra_h |
| Extra rect anchor | stored tile origin i32@`+0x00`/`+0x04` (`extra_xy − stored_xy`) | computed `(col−row)·30, (col+row)·15` |
| ZData present | flags@`+0x24` bit 1 | same |
| Extra present | flags@`+0x24` bit 0 | same |

`certify_tmp_value_layout` (tests/retail_goldens/certify_structural.rs)
proves for every non-empty cell of all 5,536 retail TMPs that the stored
values equal our assumptions exactly (zero mismatches, 2026-07-19 run). So
the two location strategies read identical bytes on all retail data.

## 3. Extra-data composition order

Native (`0x00547CF0`, tail block gated on flags bit 0): the extra rect is
blitted AFTER the diamond in the same draw — `if (*extra != 0) *dst = …` —
extra pixels win where nonzero. Our decoder composites extra BEHIND the
diamond (writes only where the diamond left 0, `overlay_rect` with
`behind=true`).

The two orders diverge only where a nonzero extra pixel coincides with a
nonzero diamond pixel. `certify_tmp_value_layout` scanned every extra pixel
of all 5,680 extra-carrying retail tiles: **zero conflicts** — retail extra
data only ever covers diamond-transparent or outside-diamond positions, so
both orders produce identical composites on the whole corpus.

## 4. What this certifies

With §1 (geometry), §2 (framing), §3 (composition): our merged
`TmpTile::pixels` buffer equals the union of what the native tile blit reads
and writes for every retail tile. TMP pixel values move from
UNVERIFIED-pending-instrument to certified (named check:
`certify_tmp_value_layout` + the geometry verification in this doc).

## 5. Depth (ZData) — remaining renderer-side note (NOT parser)

- ZData bytes parse identically (same diamond layout at the stored offset —
  covered by §1/§2).
- Composition differs mechanically: the native z-path adds a per-draw base
  (`DAT_00aa1104`) to each ZData byte and z-tests against the screen
  z-buffer per pixel; extra-z is applied the same way in the extra pass. Our
  decoder pre-merges extra depth behind diamond depth with a `v < 32` filter
  (src/assets/tmp_decode.rs `overlay_rect` depth call) — that filter has no
  verified counterpart in the binary. Whether our renderer's use of the
  merged depth buffer reproduces the native per-pixel z outcome is a
  render-pipeline question (ties into the existing GPU depth-system work),
  out of scope for asset-parser certification. Status:
  UNVERIFIED-pending-render-comparison; flagged, not certified.

## Confidence axes

| Claim | Content | Identity | Binding |
|---|---|---|---|
| Diamond template = our formula | HIGH (raw bytes read) | HIGH (`0x007EC450`/`0x007EC4C8`, referenced from the decompiled init loop) | HIGH |
| Stored-offset consumption | HIGH (decompile) | HIGH (`TMP_TileBlitter @ 0x00547CF0`) | HIGH |
| Extra drawn over, nonzero-gated | HIGH (decompile, no-z and z paths both read) | HIGH | HIGH |
| Corpus equivalences (offsets, origins, zero conflicts) | machine-proven (named test) | — | — |
| Depth composition equivalence | NOT claimed (§5) | — | — |
