# BINKW32 Import Boundary / No Internal BIK Codec Parser - Ghidra Research Report

**Address(es):** BINKW32 IAT `0x007E1590..0x007E15C0`; wrapper functions `0x00432750`, `0x00432E40`, `0x00432C70`, `0x00433060`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Inventory BINKW32 imports used by `gamemd.exe` and determine whether `gamemd.exe` contains internal BIK container/codec/header parsing logic.  
**Non-Scope:** BINKW32.DLL internals, VQA decoder internals, exact Bink SDK corruption behavior, and runtime PCM/pixel oracle capture.  
**Confidence:** High for `gamemd.exe` import boundary and static no-signature/no-internal-parser finding; Medium for exhaustive absence beyond the documented searches.  
**Active in YR:** Yes. Bink paths are active in standard YR main-menu RA2TS owner-draw movie playback and Movies/Sneak Preview fullscreen playback.

## 1. Working Notes

- **Target question:** Does `gamemd.exe` parse BIK codec/container/header bytes itself, or does it delegate BIK parsing/decoding to `BINKW32.DLL`?
- **Non-goals:** Do not investigate BINKW32.DLL internals; do not re-investigate RA2TS timing/vtable loop, BIK-before-VQA resolver order, WDT/MSAnim VQA-first split, embedded audio volume semantics, or VQA playback.
- **Evidence needed to mark COMPLETE:** Static import inventory, xrefs from BINKW32 IAT entries to all call sites, decompile confirmation of wrapper behavior around `_BinkOpen`, and documented negative searches for BIK magic/header signatures or internal codec strings inside `gamemd.exe`.
- **Stop conditions:** Stop once all BINKW32 imports and their call sites are accounted for, `FUN_00432750` is verified as an `_BinkOpen` wrapper rather than a raw parser, and BIK signature/header searches produce no `gamemd.exe` parser evidence. Defer BINKW32.DLL itself.

## 2. Overview

`gamemd.exe` does not contain a BIK bitstream decoder or a BIK container/header parser on the verified paths. It resolves movie names, opens loose/archive file sources, calls `_BinkOpen`, and then consumes fields and services from the returned Bink handle through imported BINKW32 APIs.

The current Rust architecture is therefore intentionally different from native: Rust parses and decodes BIK internally. For exact parity, parser/decoder proof should be against Bink SDK/BINKW32 runtime behavior or captured runtime oracles, not against a hidden `gamemd.exe` parser.

## 3. BINKW32 Import Inventory

Static Ghidra import listing exposes exactly these BINKW32 external locations:

| Import | External location | IAT slot | Verified call-site xrefs | Active in YR? |
|---|---:|---:|---|---|
| `_BinkSetSoundSystem@8` | `0x00410BBA` | `0x007E1590` | `0x004327B4` | Yes, conditional on sound init |
| `_BinkOpenDirectSound@4` | `0x00410BD2` | `0x007E1594` | `0x004327AE` | Yes, conditional on sound init |
| `_BinkGetError@0` | `0x00410B9A` | `0x007E1598` | `0x00432857`, `0x00432C7D` | Yes, failure paths |
| `_BinkOpen@8` | `0x00410BAC` | `0x007E159C` | `0x00432849` | Yes |
| `_BinkSetVolume@8` | `0x00410B86` | `0x007E15A0` | `0x00432897`, `0x00432E7A` | Yes |
| `_BinkClose@4` | `0x00410B5E` | `0x007E15A4` | `0x0043272A`, `0x0043277D`, `0x00432A8A` | Yes |
| `_BinkDDSurfaceType@4` | `0x00410B6E` | `0x007E15A8` | `0x00432A3E` | Yes |
| `_BinkGoto@12` | `0x00410BEC` | `0x007E15AC` | `0x00432BDE` | Yes, looping/restart path |
| `_BinkPause@8` | `0x00410BFC` | `0x007E15B0` | `0x00432C3E`, `0x00432EAB`, `0x00432ED1` | Yes |
| `_BinkNextFrame@4` | `0x00410C0C` | `0x007E15B4` | `0x00432FFB` | Yes |
| `_BinkCopyToBuffer@28` | `0x00410C20` | `0x007E15B8` | `0x00432FDC`, `0x00433155`, `0x00433251` | Yes |
| `_BinkDoFrame@4` | `0x00410C38` | `0x007E15BC` | `0x00432F62` | Yes |
| `_BinkWait@4` | `0x00410C4A` | `0x007E15C0` | `0x00432F41`, `0x00433005`, `0x00433027` | Yes |

Evidence: live Ghidra MCP `list_imports`, `list_external_locations`, and `get_bulk_xrefs` for `0x007E1590..0x007E15C0`.

## 4. Core Logic Boundary

### 4.1 Open/init delegates to `_BinkOpen`

`FUN_00432750` clears any prior BSurface helper, closes any old Bink handle with `_BinkClose`, closes any archive Win32 file handle, optionally registers Bink DirectSound, chooses a filename or Win32 handle source, and calls `_BinkOpen@8`.

The function has two verified `_BinkOpen` modes:

| Mode | First argument | Flags | Evidence | Active in YR? |
|---|---|---:|---|---|
| Loose/raw filename source | caller filename/path pointer | `0` | decompile `0x00432750`, `_BinkOpen` call `0x00432849` | Yes |
| Archive/file-handle source | Win32 handle stored at object `+0x28` | `0x800000` | `CreateFileA`/`SetFilePointer` before `_BinkOpen`, decompile `0x00432750` | Yes |

After `_BinkOpen` succeeds, `gamemd.exe` reads the returned Bink handle fields for width/height and FPS-derived timing. It does not read BIK header bytes from the source buffer. The verified reads are:

| HBINK field | Use in gamemd wrapper | Evidence | Active in YR? |
|---:|---|---|---|
| `+0x00` | width copied into local clipping calculations | decompile `0x00432750` after `_BinkOpen` success | Yes |
| `+0x04` | height copied into local clipping calculations | decompile `0x00432750` after `_BinkOpen` success | Yes |
| `+0x08` | total/end marker consumed by loop/finished predicate | `0x00432C50`, `0x00432C70` | Yes |
| `+0x0C` | current marker consumed by predicate and stored at object `+0x30` | `0x00432C50`, `0x00432E40`, `0x00432C70` | Yes |
| `+0x14` / `+0x18` | integer FPS quotient used for `object+0x24 = 60 / (field14 / field18)` | decompile `0x00432750` | Yes |

### 4.2 Decode/copy also delegates to BINKW32

The per-frame update path `0x00432E40` calls `_BinkWait`, `_BinkDoFrame`, `_BinkCopyToBuffer`, and `_BinkNextFrame`. The explicit draw path `0x00433060` calls `_BinkCopyToBuffer`. The restart path `0x00432BD0` calls `_BinkGoto`. No verified path performs macroblock, Huffman, DCT, audio packet, or frame-table parsing in `gamemd.exe`.

### 4.3 Failure reporting delegates to BINKW32

If open/init has no valid Bink handle, `FUN_00432750` calls `_BinkGetError@0` and logs `Bink Error: %s\n`. The fullscreen loop `0x00432C70` also calls `_BinkGetError@0` if the object has no handle. This is another boundary signal: failure reason text is supplied by Bink, not by a native `gamemd.exe` parser.

## 5. Negative Search Evidence

Handoff-critical negative claims were checked with live Ghidra MCP searches:

| Search | Method | Result | Interpretation |
|---|---|---|---|
| `BIKi` magic bytes `42 49 4B 69` | `search_byte_patterns` over `gamemd.exe` | no matches | no embedded BIKi signature check |
| `BIKk` magic bytes `42 49 4B 6B` | `search_byte_patterns` | no matches | no embedded BIKk signature check |
| `BIKb` magic bytes `42 49 4B 62` | `search_byte_patterns` | no matches | no embedded BIKb signature branch |
| `KB2f` bytes `4B 42 32 66` | `search_byte_patterns` | no matches | no Bink 2 signature branch found |
| `.BIK` bytes `2E 42 49 4B` | `search_byte_patterns` | matches `0x0082419C`, `0x00826354` | extension/name resolution strings only |
| `.VQA` bytes `2E 56 51 41` | `search_byte_patterns` | matches `0x008241A4` plus VQA strings | extension/name resolution strings only |
| Bink-related strings | `search_strings (?i)(BIK|BINK|BinkOpen|Bink|BINKW32|KB2|BIKi|BIKk|RAD Game|DirectSound)` | import names, `binkw32.dll`, `Bink Error`, pause/resume logs, `RENEGADE.BIK`, `Play_Movie() as Bink!`, RTTI names | no internal codec/parser diagnostics or magic strings |
| Functions named `Bink` | `search_functions Bink` | only wrapper/update/draw helper names | no parser/decoder-named function found |

Absence is never the same as a mathematical proof over every instruction, but the negative search covers the normal signatures and diagnostics that an internal BIK parser would need or usually expose. Combined with the import/callsite evidence, this strongly supports delegation to BINKW32.

## 6. Integration Points

- Owner-draw/main-menu wrapper path: resolver installs Bink wrapper vtable and reaches Bink object construction/open, then update/draw through vtable slots. Active in YR: Yes for RA2TS main-menu movie.
- Fullscreen Movies/Sneak Preview path: `FUN_005BED40` recognizes `.BIK`, constructs Bink object through `0x00432690`, and enters blocking loop `0x00432C70`. Active in YR: Yes from main-menu Movies/Credits.
- MSAnim/WDT Bink variant: prior report shows `MSBinkAnim` owns an inner Bink object from `FUN_004326C0`. Active in YR: Conditional on configured WDT/MSAnim assets.
- BINKW32 import table is statically linked by import descriptor; no code xref to the `binkw32.dll` import-name string was found, so this slice found no dynamic `LoadLibrary/GetProcAddress` Bink path.

## 7. Current Rust Implementation Status

Rust does not call BINKW32. It implements its own BIK demux/decode stack:

| Rust surface | Current status | Parity implication |
|---|---|---|
| `src/assets/bink_file.rs` | Parses BIK fixed header, audio descriptors, frame table, audio packets, and video packets. | Native `gamemd.exe` does not prove these parser validations; BINKW32/runtime oracle should be the parser spec. |
| `src/assets/bink_decode.rs` | Internal video decoder ported from FFmpeg Bink logic. | Codec parity cannot be proven from `gamemd.exe`; compare against BINKW32 output or trusted runtime captures. |
| `src/assets/bink_audio.rs` | Internal audio decoder ported from FFmpeg Bink audio logic. | Audio PCM parity cannot be proven from `gamemd.exe`; compare against BINKW32 DirectSound/decoded PCM oracle. |
| `src/render/bink_movie.rs` | Game-facing main-menu movie path uses Rust parser/decoder and RGBA upload. | Native wrapper delegates to BINKW32 wait/do/copy/timing and DirectDraw surface type. |
| `src/bin/bik-player.rs`, `src/bin/bik-survey.rs` | Tooling uses the Rust parser/decoder for survey and playback. | Useful diagnostics, but not native proof unless paired with BINKW32 oracle output. |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| BINKW32 import inventory | verified | `list_imports`, `list_external_locations` | none |
| BINKW32 IAT callsite map | verified | `get_bulk_xrefs` for `0x007E1590..0x007E15C0` | none |
| `FUN_00432750` open/init boundary | verified | decompile `0x00432750`; `_BinkOpen` call `0x00432849` | corrupt-BIK runtime behavior belongs to slot 3/runtime |
| Per-frame Bink calls | verified | decompile `0x00432E40`; IAT xrefs | BINKW32 internals deferred |
| Explicit draw Bink calls | verified | decompile `0x00433060`; IAT xref `0x00433155` | exact runtime DDSurfaceType value deferred |
| BIK magic/header signature search | verified | `search_byte_patterns` for `BIKi`, `BIKk`, `BIKb`, `KB2f` | cannot prove against intentionally obfuscated parser, but no evidence found |
| Bink-related string search | verified | `search_strings` Bink/BIK/KB2/RAD query | none |
| BINKW32.DLL parser/decoder internals | deferred | external DLL boundary | runtime/DLL-level investigation |
| Rust parser validation equivalence | deferred | Rust scan of `src/assets/bink_file.rs` | compare to BINKW32 behavior |

## 9. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which BINKW32 imports exist in gamemd.exe? -> Thirteen imports from `_BinkSetSoundSystem@8` through `_BinkWait@4`.` (evidence: `list_imports`, `list_external_locations`)
- `[RESOLVED] OQ-02 - Are all BINKW32 imports xrefed from known wrapper/update/draw functions? -> Yes; xrefs are confined to `0x00432750`, cleanup helpers, pause/restart/update/draw helpers, and fullscreen loop error path.` (evidence: `get_bulk_xrefs 0x007E1590..0x007E15C0`)
- `[RESOLVED] OQ-03 - Does `FUN_00432750` inspect BIK header bytes before `_BinkOpen`? -> No verified raw header reads; it prepares source mode and delegates to `_BinkOpen`.` (evidence: decompile `0x00432750`)
- `[RESOLVED] OQ-04 - What does `FUN_00432750` consume after `_BinkOpen` succeeds? -> Returned HBINK fields for dimensions, current/total markers, and FPS-derived timing, plus Bink APIs for volume/surface type.` (evidence: `0x00432750`, `0x00432C50`, `0x00432E40`)
- `[RESOLVED] OQ-05 - Are BIK magic signatures embedded in gamemd.exe? -> No matches for `BIKi`, `BIKk`, `BIKb`, or `KB2f` byte patterns.` (evidence: `search_byte_patterns`)
- `[RESOLVED] OQ-06 - Are `.BIK` strings parser signatures or resolver strings? -> Resolver/name strings only; xrefs go to extension append/compare sites and hardcoded `RENEGADE.BIK`.` (evidence: xrefs to `0x0082419C`, `0x00826354`)
- `[RESOLVED] OQ-07 - Is there a dynamic BINKW32 loader path? -> No code xref to `binkw32.dll` string found in this slice; Bink APIs appear as static imports.` (evidence: xrefs to `0x00810C58`)
- `[RESOLVED] OQ-08 - Does fullscreen playback decode BIK internally? -> No; it constructs/open Bink object and calls the same Bink wrapper loop/API path.` (evidence: decompile `0x005BED40`, `0x00432690`, `0x00432C70`)
- `[RESOLVED] OQ-09 - Does current Rust parse BIK internally? -> Yes; `BinkFile`, `BinkDecoder`, and `BinkAudioDecoder` parse/decode internally.` (evidence: `rg` over `src/assets/bink_file.rs`, `src/assets/bink_decode.rs`, `src/assets/bink_audio.rs`)
- `[DEFERRED] OQ-10 - Is Rust BIK decode byte/pixel/PCM identical to BINKW32?` (category: needs-runtime-debugger; reason: `gamemd.exe` delegates decoding to external DLL; next-step-if-pursued: capture BINKW32 frame/audio output for stock fixtures and compare)
- `[DEFERRED] OQ-11 - What exact diagnostics/return behavior does BINKW32 produce for corrupt or unsupported BIK variants?` (category: needs-runtime-debugger; reason: failure details live behind `_BinkOpen` and `_BinkGetError`; next-step-if-pursued: runtime oracle with deliberately corrupted BIKs)

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `gamemd.exe` delegates BIK container/codec validation to `_BinkOpen`; no internal BIK magic/header parser was found. | `_BinkOpen` call `0x00432849`; negative `BIKi`/`BIKk`/`BIKb`/`KB2f` searches | Rust parser has its own validation bounds and supported-tag policy. | `src/assets/bink_file.rs` | Treat Rust parser checks as BINKW32-compatibility claims, not gamemd-wrapper claims. | Corrupt/edge BIK fixture behavior is compared against captured BINKW32 open/error outcome. Proposed test `bink_parser_validation_matches_binkw32_open_oracle`. | Do not justify Rust header rejection behavior by citing `gamemd.exe`; native wrapper has no matching branch. |
| Native playback uses BINKW32 APIs for frame timing, decode, copy, goto, pause, volume, and surface typing. | IAT xrefs `0x007E15A0..0x007E15C0`; decompile `0x00432E40`, `0x00433060` | Rust decodes to YUV/RGBA internally and schedules with Rust-side frame logic. | `src/assets/bink_decode.rs`, `src/assets/bink_audio.rs`, `src/render/bink_movie.rs` | Establish parity through BINKW32/runtime oracle tests for decoded frames/audio/timing-sensitive boundaries. | Stock BIK frame N and audio packet block output match a captured BINKW32 oracle for the same asset. Proposed tests `bink_video_decode_matches_binkw32_frame_oracle` and `bink_audio_decode_matches_binkw32_pcm_oracle`. | Do not treat FFmpeg-port decoder correctness as proven gamemd parity without an oracle. |
| BINKW32 is statically imported; no `gamemd.exe` dynamic loader/parser fallback was found. | `list_imports` BINKW32 entries; no xrefs to `binkw32.dll` string `0x00810C58` | Rust has no BINKW32 adapter/test harness. | Future test harness/tooling beside `src/bin/bik-player.rs` / fixtures | If exact proof is required, build a runtime/DLL oracle outside sim/render code rather than adding native DLL dependency to game logic. | Test harness can load stock BIK and record dimensions/error/first-frame/PCM values from BINKW32. Proposed tool/test name `binkw32_oracle_captures_stock_bik_outputs`. | Do not put BINKW32 dependency into deterministic engine runtime as a shortcut. |

### Negative Facts / Do Not Do

- Do not claim `gamemd.exe` implements a BIK container parser. Evidence: `_BinkOpen` boundary at `0x00432849` plus no `BIKi`/`BIKk`/`BIKb`/`KB2f` byte-pattern matches.
- Do not claim Rust parser rejection of zero FPS, unsupported tags, alpha, grayscale, or frame-table bounds is a verified gamemd branch. Evidence: `FUN_00432750` reads HBINK fields only after `_BinkOpen` success and has no raw-header guard.
- Do not treat `_BinkOpen` flag `0x800000` as a codec/audio/header flag. Evidence: it is used only on the Win32 file-handle source path in `0x00432750`.
- Do not collapse BINKW32 parser/decoder parity into existing `gamemd.exe` Ghidra proof. Evidence: actual bitstream/audio/video parsing is behind external imports.
- Do not use absence of VQA implementation to remove VQA resolver selection. Evidence: `.BIK`/`.VQA` strings and prior live resolver reports prove resolver context; this slot only proves BIK codec parsing boundary.

### Stale Docs / Follow-up Docs

- `docs/research/FUN_00432750_BINK_OPEN_INIT_GHIDRA_REPORT.md`: keep its no-wrapper-parser finding, but if stronger wording is wanted replace "Rust parser implications for active YR Bink wrappers" with: "`gamemd.exe` does not parse BIK header/container bytes on this path; Rust BIK parser validations must be proven against BINKW32/runtime oracle behavior, while `gamemd.exe` only proves the source-mode, `_BinkOpen`, returned-HBINK-field, and Bink API call contract."
- Any doc wording equivalent to "the Rust BIK parser is gamemd-parity because gamemd parses the same header fields" should be replaced with: "`gamemd.exe` delegates BIK parsing and decoding to BINKW32.DLL; Rust's parser/decoder is a replacement for BINKW32 behavior, not for a native `gamemd.exe` parser."

## Sources

- Live Ghidra MCP `list_imports` and `list_external_locations` for BINKW32 imports.
- Live Ghidra MCP `get_bulk_xrefs` for IAT slots `0x007E1590..0x007E15C0`.
- Live Ghidra MCP decompile: `0x00432750`, `0x00432690`, `0x004326C0`, `0x00432700`, `0x00432A60`, `0x00432BD0`, `0x00432C30`, `0x00432C50`, `0x00432C70`, `0x00432E40`, `0x00433020`, `0x00433040`, `0x00433060`, `0x004331F0`, `0x00433270`, `0x00433330`, `0x005BED40`.
- Live Ghidra MCP `search_byte_patterns` for `BIKi`, `BIKk`, `BIKb`, `KB2f`, `.BIK`, `.VQA`.
- Live Ghidra MCP `search_strings` for Bink/BIK/BINKW32/KB2/RAD terms.
- Rust scan: `src/assets/bink_file.rs`, `src/assets/bink_decode.rs`, `src/assets/bink_audio.rs`, `src/render/bink_movie.rs`, `src/bin/bik-player.rs`, `src/bin/bik-survey.rs`.

