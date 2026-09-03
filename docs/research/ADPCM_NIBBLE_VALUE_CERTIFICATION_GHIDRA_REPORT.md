# IMA-ADPCM Sample-Value Certification — Ghidra Research Report

**Date:** 2026-07-19
**Status:** VERIFIED (live Ghidra session, gamemd.exe — decompile + memory reads + emulation vectors) + corpus-proven
**Goal:** upgrade AUD/bag ADPCM decoded SAMPLE VALUES from ratchet-only to a
citable certification — the last parser row on the retail_goldens
UNVERIFIED list. Also the validation spike for the emulate_function
instrument (first use in this repo that produced committed goldens).

## Verdict

**Our IMA-ADPCM nibble decoder (src/assets/ima_adpcm.rs
`ImaAdpcmState::decode_nibble`, moved there from `aud_file.rs` on 2026-09-03
when the crate's three copies were unified) is value-identical to the original
engine's (`IMA_ADPCM__DecodeSample @ 0x0040ACD0`), and our block decoder
(`ima_adpcm::decode_blocks`, shared by the bag and WAV paths as it is natively)
is value-identical to the original's (`IMA_ADPCM__DecodeBlock @ 0x0040AA70`) on
all retail data.** Evidence:
algorithm identity from decompiles, table identity from raw memory reads,
15 machine-derived emulation vectors (including both saturation clamps), and
corpus proofs that the divergence classes in §2 never occur in retail data —
with one residual, a sound shorter than one stride, recorded in §2.1.

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
src/assets/ima_adpcm.rs (moved there 2026-09-03 with the decoder itself, so the
vectors now guard the WAV path too) — all 15 match our decoder:

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
L/R groups). The pump hands it exactly `block_align` bytes every time — see
§2.1 — so the block grammar is stride-determined. Matches
src/assets/ima_adpcm.rs `decode_blocks` with these behavioral differences:

| Divergence class | Native | Ours | Corpus status |
|---|---|---|---|
| Preamble step_index > 0x58 or reserved ≠ 0 | rejects the block, stopping the sound | rejects the block, stopping the sound | **never occurs** (0 of 3,325 IMA entries, all blocks) |
| Stride payload not a multiple of `4*channels` | rounds the group count UP (mono) / runs one group long (stereo), reads past the block end, then reports failure | drops the block | **unreachable**: the pump always presents exactly `block_align` bytes, and `(512-4)%4 == 0`, `(1024-8)%8 == 0` (§2.1) |
| Last block shorter than the stride | decodes a full stride; the bytes past the tail are the previous block's, still in the input buffer | reproduces that buffer, except for a sound shorter than one stride (§2.1) | 2,049 of 2,197 AUDIOMD IMA entries |

Proven by `certify_bag_adpcm_block_invariants`
(tests/retail_goldens/certify_audio.rs): 3,325 IMA entries across
AUDIOMD/AUDIO.MIX.

### 2.1 CORRECTION (2026-09-03) — the remainder rows were wrong twice

**First error (the original table).** It claimed the native "truncates" a short
stereo remainder and was therefore "identical by construction". The disassembly
says otherwise. `0x0040ACB3: TEST ECX,ECX / JG 0x0040abf5` continues the group
loop while any bytes remain, so a remainder of 1..7 bytes runs one **more** full
8-byte group — over-reading the block — and the closing `SETZ AL` then returns
false. `Audio__DecodeCompressedBlock` case 2 turns that false into a `return 0`
(`0x00409F4E`) and the buffer pump abandons the sound, so the block's samples
never reach the mixer. Mono is the same shape: `0x0040AB4E: LEA EAX,[ECX+3] /
SHR EAX,0x2` rounds the group count up, and `0x0040ABD4` reports failure unless
the payload was an exact multiple of 4.

**Second error (the first correction, same day).** All of the above is true of
the *function*, and none of it applies to the *pipeline*. An earlier revision of
this section then claimed "native drops that tail block … VERA now does the
same" and that the over-read values "are discarded with the block". Both are
false, because the block decoder never sees a short payload.

The decoder's available-byte count is `+0x80 - +0x84`, read in the `0x0040AA70`
prologue. Between them those fields have exactly four writes, all in
`Audio__DecodeCompressedBlock @ 0x00409DE0`; `FUN_00409880 @ 0x00409880` writes
neither (`search_instructions` over both functions, 2026-09-03):

| Address | Write |
|---|---|
| `0x00409F8A` | case 3: `+0x80 = +0xAC` = `block_align` |
| `0x00409F90` | case 3: `+0x84 = +0xAC` = `block_align` |
| `0x00409EBE` | case 0: `+0x84 -= bytes copied into the input buffer` |
| `0x00409EA4` | case 0: `+0x84 = 0`, forced when the source is exhausted or NULL |

Case 0 leaves the fill state only with `+0x84 == 0`, so **`avail` is always
exactly `block_align`**. Every retail block therefore has a whole payload
(`(512-4) % 4 == 0`, `(1024-8) % 8 == 0`) and the two failure branches above are
unreachable on retail data — they are reachable only through a stride that is
itself ragged.

**What actually happens at end of stream.** `FUN_00409880` calls the decode
callback with a NULL source pointer and a NULL count at `0x00409B41` (the
`PUSH EDI/PUSH EDI` pair with `EDI == 0`), taken while the stream's
"decoder still holds data" flag `+0xA8` is set. `+0x84` is forced to 0 and the
block decoder runs over a full stride. The input buffer at `+0x7C` is
`malloc(block_align)` (`FUN_00409C40`) and every fill cycle writes it from
offset 0 (`puVar6 = (+0x80 - +0x84) + +0x7C`), so the bytes past a short tail
are **the previous block's bytes at those same offsets** — real audio from
one block earlier, not decay or silence. `Audio__DecodeCompressedBlock` case 1
then copies the whole `+0x94` decoded byte count out to the DirectSound buffer;
there is no length cap. So the native emits `ceil(size / block_align)` blocks
for every sound, each `1 + (block_align - 4*ch)/(4*ch) * 8` frames.

Measured consequences of the two retail shapes:

- `ABIRJ01A` — 14,388 bytes, mono, stride 512 → 29 × 1017 = **29,493 samples**.
  VERA emitted 28,573 before 2026-09-03 (~41.7 ms short).
- `grexselb` — offset 7,063,684, size 41,212, stereo IMA, stride 1024 → 41 ×
  1017 frames = **83,394 samples**. Its 252-byte tail is the corpus's only block
  whose *real* payload is ragged (30 whole groups plus 4 bytes), which is why
  the earlier "native drops it" reading picked this entry — but padded to the
  stride it is not ragged and the native decodes it in full.

Since 2026-09-03 `assets::ima_adpcm::decode_blocks` reproduces that feed: it
rebuilds the flush buffer as `tail ++ previous_block[tail.len()..]`, keeps the
preamble rejection, and gates the ragged-payload rejection on the stride rather
than on where the data ends. Pinned by
`retail_sound_shapes_decode_the_native_block_count` and
`end_of_stream_flush_pads_from_the_previous_block_like_the_native`.

**Residual.** A sound shorter than one stride is flushed against a buffer this
sound never filled — `malloc`ed on first use, otherwise the previous sound's
bytes, since `FUN_00409C40` reallocates only when the stride grows. Not
reproducible from the file; VERA decodes the whole groups such a sound carries.

The corpus claim in the original table was also incomplete: the check only
tested mono alignment, so a stereo violation could not be seen, and the entry
count was understated — AUDIOMD alone holds **9** stereo entries (5 IMA with
`chunk_size` 1024, 4 raw PCM), not 7 across both bags.

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
