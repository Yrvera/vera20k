# AUD Declared Trailing Sample — Unreachable in gamemd — Ghidra Research Report

**Date:** 2026-07-19
**Status:** VERIFIED (live Ghidra session, gamemd.exe) with one bounded open corner (§4)
**Trigger:** `tests/retail_goldens` `certify_aud_chunk_walk` found 4 unique
retail .aud files whose FINAL chunk declares `output_size = 4*compressed + 2`
— one 16-bit sample more than the IMA nibble stream can produce:
`intro.aud` (ra2.mix→local.mix), `wipe.aud`, `efficien.aud`, `mouseon.aud`
(both `#0x330A4ADF` and `#0x74AA300F` nested archives in ra2.mix). Our
input-driven `decode_aud` (src/assets/aud_file.rs) ends one sample short of
the declared total on these files.

## Verdict

**The discrepancy is unobservable in gamemd: the engine never decodes those
four files.** No change to `decode_aud` is needed for parity. The trailing
declaration is a Westwood-encoder quirk in files only the original installer /
RA2-era shell ever consumed.

## Evidence

1. **The only .aud filenames referenced anywhere in gamemd.exe are
   TEXT1.AUD / TEXT2.AUD / TEXT3.AUD** (score-screen narration).
   Verified via `search_strings "(?i)\.aud"` — exactly 6 matches, all six the
   TEXT*.AUD literals + their log strings at `0x0083E5C8–0x0083E640`, consumed
   by `ScoreScreen__LoadNarrationAudio @ 0x00690640` (verified decompile:
   loads raw bytes via `LoadFileFromMIX` / whole-file read `FUN_004a3890`).
   No `intro.aud`, no `wipe.aud`, no `"%s.aud"` format string exists.

2. **Music/themes are WAV, not AUD.** `StreamPlayer__PlayFile @ 0x00407B60`
   (verified decompile) parses the stream with `WAV__ParseHeader` — the theme
   and streamed-audio path is RIFF/WAV. (Consistent with the retail corpus:
   the music tracks are RIFF files, which is why the MIX sniffer finds only
   23 .aud files total.)

3. **SFX / EVA / voices decode via the audio.bag block-IMA path, which has no
   DEAF chunks.** `FUN_00409c40` (verified decompile) wires the decode
   pipeline: format tag 1 → `Audio__DecodeCompressedBlock @ 0x00409DE0`
   (pull state machine, verified decompile) whose `+0xB0` block callback is
   `IMA_ADPCM__DecodeBlock @ 0x0040AA70` (verified decompile: 4-byte
   per-channel preambles `{i16 predictor, u8 step_index ≤ 0x58, u8 0}`,
   then nibble groups — the WAV/bag Microsoft-IMA block layout, input-driven,
   matching src/assets/audio_bag.rs). Per-nibble math in
   `IMA_ADPCM__DecodeSample @ 0x0040ACD0` (verified decompile) matches our
   `ImaAdpcmState::decode_nibble` exactly (step/8 + conditional step/4,
   step/2, step; sign bit 8; clamp ±0x8000/0x7FFF; index adjust table at
   `0x00816518`, step table at `0x00816558`, index clamp 0..0x58).

4. **No code in gamemd validates the AUD chunk magic `0x0000DEAF`.**
   `search_byte_patterns "afde0000"` (the 32-bit LE immediate): zero matches.
   `search_byte_patterns "afde"` (16-bit): 22 matches, all coincidental
   arithmetic constants — three spot-verified as false positives
   (`FUN_0053bba0` screen-warp effect, `Apply_area_damage @ 0x00489280`,
   `FUN_00620050` alpha-blend line blitter).
   - **Open corner:** the exact routine that decodes TEXT*.AUD's DEAF-chunk
     stream at score-screen playback was not pinned down (the play call is
     behind unlabeled score-screen presentation code; a chunk walker that
     *skips* the magic field without comparing it would leave no immediate to
     search for). This does not affect the verdict: TEXT1–3.AUD all satisfy
     `chunk output == 4*compressed` exactly (retail_goldens corpus scan), so
     no trailing-sample behavior is exercised even there.
     Status: UNVERIFIED-pending-deeper-trace, bounded to "which function",
     not "what happens".

5. **Reachability of the four files.** `wipe/efficien/mouseen.aud` live only
   inside the unnamed nested archives `#0x330A4ADF` / `#0x74AA300F` (Westwood
   installer sound sets); gamemd mounts a fixed archive list and has no
   string/hash reference to them. `intro.aud` (RA2-era shell theme) has no
   reference in gamemd (YR shell music goes through the WAV theme system).

## Consequences for the Rust engine

- `decode_aud` stays input-driven; no fix. The one-sample shortfall can only
  manifest if OUR engine chooses to play one of those four files — none of
  which the original plays.
- `tests/retail_goldens/certify_audio.rs` already encodes the corpus shape
  (`output == 4*compressed`, final-chunk `+2` exception for exactly these
  files) and records them. Its comment calling the native trailing-sample
  semantics "unverified" can now be sharpened to "unreachable in gamemd".

## Confidence axes

| Claim | Content | Identity | Binding |
|---|---|---|---|
| Only TEXT*.AUD referenced by name | HIGH (exhaustive string search) | HIGH | HIGH |
| Themes/stream = WAV | HIGH (decompiled PlayFile) | HIGH (named fn) | HIGH |
| Bag path = block IMA, input-driven | HIGH (3 decompiles) | HIGH | HIGH |
| No DEAF magic check anywhere | HIGH (immediate search + spot checks) | — | MEDIUM-HIGH (a magic-skipping walker would evade the search; covered by §4 open corner) |
| Four files unreachable | HIGH for wipe/efficien/mouseon (unmounted nested archives) | — | MEDIUM-HIGH for intro.aud (absence of reference; hash-only lookup not exhaustively excluded) |
