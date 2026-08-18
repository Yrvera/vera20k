# IMA-ADPCM Sample-Value Certification — Ghidra Research Report

**Date:** 2026-07-19
**Status:** VERIFIED (live Ghidra session, gamemd.exe — decompile + memory reads + emulation vectors) + corpus-proven
**Goal:** upgrade AUD/bag ADPCM decoded SAMPLE VALUES from ratchet-only to a
citable certification — the last parser row on the retail_goldens
UNVERIFIED list. Also the validation spike for the emulate_function
instrument (first use in this repo that produced committed goldens).

## Verdict

**Our IMA-ADPCM nibble decoder (src/assets/aud_file.rs
`ImaAdpcmState::decode_nibble`) is value-identical to the original engine's
(`IMA_ADPCM__DecodeSample @ 0x0040ACD0`), and our bag block decoder
(src/assets/audio_bag.rs) is value-identical to the original's
(`IMA_ADPCM__DecodeBlock @ 0x0040AA70`) on all retail data.** Evidence:
algorithm identity from decompiles, table identity from raw memory reads,
15 machine-derived emulation vectors (including both saturation clamps), and
corpus proofs that the two behavioral divergence classes (invalid-preamble
handling; unaligned mono payloads) never occur in retail data.

## 1. Nibble decoder identity

`IMA_ADPCM__DecodeSample @ 0x0040ACD0` (verified via
`decompile_function 0x0040ACD0`):

```
step  = STEP_TABLE[state.index]           // table at 0x00816558
delta = step>>3
if n&1: delta += step>>2
if n&2: delta += step>>1
if n&4: delta += step
if n&8: delta = -delta
state.predicted = clamp(state.predicted + delta, -0x8000, 0x7FFF)
state.index    += INDEX_ADJUST[n]          // table at 0x00816518
state.index     = clamp(state.index, 0, 0x58)
return (i16)state.predicted
```

Identical arithmetic, clamp bounds, and operation ORDER (predictor clamps
before the index update) to our `decode_nibble`.

Tables (verified via `read_memory`):
- `0x00816518` (index adjust, 16 × i32): `[-1,-1,-1,-1, 2,4,6,8]` twice —
  entries 8–15 mirror 0–7, so the native's full-nibble indexing equals our
  8-entry `code & 7` indexing for every nibble.
- `0x00816558` (step table): entries 0–9 = `7,8,9,10,11,12,13,14,16,17` and
  entry 88 @ `0x008166B8` = `32767` — matches our `STEP_TABLE` head and tail.

### Emulation vectors (instrument: `emulate_function`)

15 vectors captured by emulating `0x0040ACD0` with ECX = nibble and EDX
pointing at (predictor, index) pairs; committed as the unit test
`adpcm_nibble_matches_original_engine_emulation_vectors` in
src/assets/aud_file.rs — all 15 match our decoder:

| state (pred, idx) | state source | nibbles → samples |
|---|---|---|
| (0, 0) | unmapped memory (reads 0) | 0→0, 3→4, 5→8, 7→11, 8→0, B→−4, F→−11 |
| (0, 4) | image bytes @ `0x007EC450` | 7→19, F→−19 |
| (24, 40) | image bytes @ `0x007EC45C` | 7→655, A→−186 |
| (36, 64) | image bytes @ `0x008469FC` | 5→4609, D→−4537 |
| (60, 84) | image bytes @ `0x007EC464` | 7→**32767** (clamp+), F→**−32768** (clamp−) |

Instrument note: the tool's `memory` parameter did not apply writes in this
session (both JSON and `addr=hex` forms silently no-op; state pointed at
unmapped memory reads zeros). Workaround that keeps vectors machine-derived:
point the state pointer at existing image bytes forming the desired
(pred, idx) pairs. Sampled vectors are evidence; the PROOF is the algorithm +
table identity above — the vectors guard the restatement.

## 2. Block decoder (bag path) equivalence

`IMA_ADPCM__DecodeBlock @ 0x0040AA70` (verified via
`decompile_function 0x0040AA70`), wired as the bag/WAV decode callback by
`FUN_00409C40` (format tag 1). Grammar: per block, one 4-byte preamble per
channel `{i16 predictor, u8 step_index, u8 reserved}`, predictor emitted as
the first sample, then nibble payload (mono: 4-byte groups; stereo: 8-byte
L/R groups, remainder truncated). Matches src/assets/audio_bag.rs
`decode_ima_adpcm_blocks` with two behavioral differences:

| Divergence class | Native | Ours | Corpus status |
|---|---|---|---|
| Preamble step_index > 0x58 or reserved ≠ 0 | rejects the whole load | clamps / ignores | **never occurs** (0 of 3,325 IMA entries, all blocks) |
| Mono payload not 4-byte aligned | rounds UP, reads past block end | decodes exact bytes | **never occurs** (all mono blocks 4-aligned) |
| Stereo payload remainder < 8 bytes | truncates | truncates | identical by construction (7 stereo entries) |

Proven by `certify_bag_adpcm_block_invariants`
(tests/retail_goldens/certify_audio.rs): 3,325 IMA entries across
AUDIOMD/AUDIO.MIX, zero violations (2026-07-19 run).

## 3. The .aud chunk path

The DEAF-chunk format-99 files reachable by gamemd (TEXT1–3.AUD only) satisfy
`chunk output == 4×compressed` exactly (SHP-suite corpus scan), and the four
trailing-sample files are unreachable — see
`AUD_TRAILING_SAMPLE_UNREACHABLE_GHIDRA_REPORT.md`. With the nibble decoder
certified (§1) and per-chunk state continuity identical (both decoders carry
state across chunks with no per-chunk reset — chunk walk in our decode_aud,
stream state machine natively), .aud sample values are covered for every
file the original can play.

## Confidence axes

| Claim | Content | Identity | Binding |
|---|---|---|---|
| Nibble algorithm identity | HIGH (decompile, 1:1) | HIGH (named fn, callers traced) | HIGH |
| Table identity | HIGH (raw reads incl. tail) | HIGH | HIGH |
| Emulation vectors | HIGH (15/15 match, clamps included) | HIGH | HIGH (same fn the block decoder calls) |
| Block grammar + divergence classes | HIGH (decompile) | HIGH | HIGH (wired at `FUN_00409C40` format 1) |
| Corpus invariants | machine-proven (named test) | — | — |
