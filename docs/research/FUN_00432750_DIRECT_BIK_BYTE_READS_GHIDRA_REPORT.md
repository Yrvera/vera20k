# FUN_00432750 Direct BIK Byte Reads - Ghidra Report

**Target:** `FUN_00432750_DIRECT_BIK_BYTE_READS`  
**Primary address:** `0x00432750`  
**Investigation mode:** exhaustive-slice for the open-boundary byte-read question only  
**Status:** COMPLETE for `gamemd.exe` direct BIK byte/header reads in and immediately around `0x00432750`; codec/DLL internals deferred.  
**Date:** 2026-05-27

## Working Notes

- **Target question:** Does `gamemd.exe` directly read or parse BIK file/header bytes in or immediately around `FUN_00432750` before/after `_BinkOpen`, or does it delegate parsing to BINKW32 and then read only returned `HBINK` fields?
- **Non-goals:** Do not re-investigate resolver extension order, RA2TS owner-draw cadence, VQA playback internals, Bink audio decoding, full `HBINK` field map outside the fields touched by this open routine, or BINKW32.DLL internals.
- **Evidence needed to mark COMPLETE:** Live Ghidra decompile for `0x00432750`, disassembly/call evidence around `_BinkOpen`, caller evidence for active constructors, callee/import evidence for absence of direct file reads in this routine, and source-mode checks for filename vs handle paths.
- **Stop conditions:** Stop if live Ghidra cannot decompile `0x00432750`, if the target requires mutating Ghidra function boundaries, or if evidence shows parsing is inside BINKW32 rather than `gamemd.exe`; record BINKW32 internals as deferred.

## Summary

`FUN_00432750` is not a BIK container/header parser. The verified boundary is:

1. clean old Bink/surface/Win32-handle state;
2. optionally register DirectSound for Bink;
3. test loose-file availability through `RawFileClass`;
4. otherwise resolve an archive entry to a Win32 file handle and seek it to the entry offset;
5. call `_BinkOpen@8`;
6. if the returned `HBINK` is non-null, read Bink handle fields for volume setup, integer frame cadence, dimensions, clipping, and DirectDraw surface type.

No direct `ReadFile`, BIK signature check, BIK header field load, frame table parse, audio descriptor parse, or packet split occurs in `0x00432750`. The only direct file operation inside the function is `CreateFileA` plus `SetFilePointer` for the file-handle source mode before handing the handle to `_BinkOpen`.

## Verified Findings

### 1. Active callers reach this function through the Bink object constructors

**Active in YR:** Yes for Bink movie construction; exact surface/context is caller-dependent.  
**Evidence:** Live Ghidra xrefs to `0x00432750` are `0x004326B3` in `FUN_00432690` and `0x004326E9` in `FUN_004326C0`. Live decompile shows both constructors initialize object state and then call `FUN_00432750(param_2)`.

Details:

- `0x00432690` initializes `+0x04=0`, `+0x0C=0`, `+0x20=0`, `+0x28=-1`, `+0x2C=1`, `+0x2D=0`, then calls `0x00432750`.
- `0x004326C0` additionally stores caller surface/context at `+0x0C`, then initializes the same Bink object fields and calls `0x00432750`.

### 2. Loose-file availability check does not parse BIK bytes

**Active in YR:** Yes when this constructor is given a filename/source token.  
**Evidence:** Live decompile of `0x00432750` calls `RawFileClass__Constructor` at `0x004327C3`, then a RawFile vtable slot at `0x004327CD`. Live vtable inspection/decompile resolves the relevant availability/open helpers around `0x0065CBF0` and `0x0065CB50`: they use `CreateFileA`, `CloseHandle`, and handle bookkeeping, not `ReadFile`.

Details:

- `0x0065CBF0` returns false if the path pointer at RawFile `+0x18` is null.
- In non-open mode it calls `CreateFileA(path, 0x80000000, 1, NULL, 3, 0x80, NULL)`, stores the handle, and immediately closes it on success.
- No BIK magic bytes or header fields are read by this availability branch.

### 3. Archive/file-handle mode resolves offset metadata, then delegates bytes to `_BinkOpen`

**Active in YR:** Conditional; used when the loose-file availability check does not select direct filename mode and the archive helper finds the asset.  
**Evidence:** Live decompile/disassembly of `0x004327E6..0x00432849`: after the loose-file path fails, `FUN_005B4430` is called. If it returns `1`, `CreateFileA` opens the backing path, object `+0x28` stores the handle, `SetFilePointer` seeks the handle, and `_BinkOpen@8` is called with first argument `object+0x28` and flags `0x800000`.

Details:

- `FUN_005B4430` decompile shows filename normalization, CRC lookup, and archive directory entry metadata output. It copies and uppercases the requested name, hashes it, binary-searches archive index rows, and writes offset/size metadata to output pointers.
- `FUN_005B4430` does not read BIK payload bytes; it resolves the archive entry location.
- `0x00432824` calls `CreateFileA`.
- `0x0043283A` calls `SetFilePointer` with the resolved offset.
- `0x00432843` pushes `0x800000`.
- `0x00432849` calls `_BinkOpen@8`.

### 4. Filename mode passes the caller source pointer directly to `_BinkOpen`

**Active in YR:** Conditional; used when RawFile availability succeeds.  
**Evidence:** Live assembly context at `0x004327DE..0x00432849`: when the availability result is true, the function pushes flags `0`, pushes the caller filename/source pointer, and jumps to the common `_BinkOpen@8` call.

Details:

- The direct branch is `TEST BL,BL; JZ archive_path`.
- True path at `0x004327E2..0x004327E4`: pushes `0` flags and the source pointer.
- Common call at `0x00432849`: `_BinkOpen@8`.
- No direct file read or header validation exists between the availability check and `_BinkOpen`.

### 5. After `_BinkOpen`, `gamemd.exe` reads `HBINK` fields, not BIK file bytes

**Active in YR:** Yes after successful `_BinkOpen`.  
**Evidence:** Live decompile and assembly context at `0x00432877..0x004328CA` show reads through object `+0x04`, the returned Bink handle. Callees for `0x00432750` include `_BinkOpen@8`, `_BinkSetVolume@8`, `_BinkDDSurfaceType@4`, and surface/window helpers; they do not include `ReadFile` or `GetFileSize`.

Direct `HBINK` field reads in this open routine:

- `handle + 0x14` and `handle + 0x18`: unsigned `DIV` sequence for `object+0x24 = 60 / (handle[0x14] / handle[0x18])`.
- `handle + 0x00` and `handle + 0x04`: width/height-like dimensions used for centering and clipping.

The failure branch at `0x00432852..0x00432874` calls `_BinkGetError@0`, logs `Bink Error: %s\n`, and returns `0`; it does not inspect the failed file bytes.

## Import / API Boundary Evidence

Live Ghidra `get_function_callees(0x00432750)` returned:

- Win32/source setup: `CreateFileA`, `SetFilePointer`, `CloseHandle`, `GetClientRect`
- Bink API: `_BinkSetSoundSystem@8`, `_BinkOpen@8`, `_BinkSetVolume@8`, `_BinkClose@4`, `_BinkDDSurfaceType@4`, `_BinkGetError@0`
- Local helpers: `RawFileClass__Constructor`, `FileClass__Constructor`, `FUN_005B4430`, `BSurface__Constructor`, cleanup/logging/math helpers

It did **not** return `ReadFile`, `GetFileSize`, or any local BIK-header parser callee for `0x00432750`.

For contrast, live caller lookup shows `ReadFile` callers elsewhere: `0x0065CCE0`, `0x00774980`, `0x007D0844`. These are generic file helpers or unrelated readers, not callers from `0x00432750`.

## Implementation Handoff

- Verified behavior -> `gamemd.exe` does not parse BIK header bytes in `0x00432750`; it delegates validation/decode to `_BinkOpen` and then reads returned `HBINK` fields -> evidence `0x004327DE..0x00432849`, `0x00432877..0x004328CA`, callee list with `_BinkOpen@8` but no `ReadFile` -> current Rust delta: `src/assets/bink_file.rs` performs direct Rust-side header/frame/audio parsing -> affected surface `src/assets/bink_file.rs` and oracle tests -> acceptance `bink_parser_validation_is_marked_binkw32_oracle_not_gamemd_wrapper` -> do not describe Rust parser validation as matching a gamemd-side BIK header parser.

- Verified behavior -> loose-file availability only probes file existence/openability before `_BinkOpen`; it does not inspect BIK magic/header -> evidence `0x004327C3..0x004327D9`, RawFile helper `0x0065CBF0`, callees `CreateFileA`/`CloseHandle` only -> current Rust delta: asset loading may conflate resolver success with parse success -> affected surface movie resolver/error types above `AssetManager` -> acceptance `movie_resolver_success_does_not_imply_bik_parse_success` -> do not reject at resolver level based on Rust parser header checks.

- Verified behavior -> archive/file-handle path resolves archive offset/name metadata, seeks a Win32 handle, and passes flags `0x800000` to `_BinkOpen`; the helper does not parse payload bytes -> evidence `FUN_005B4430` decompile plus `0x00432824..0x00432849` -> current Rust delta: archive extraction gives byte slices to the parser rather than a Bink DLL file handle -> affected surface asset archive loading and future runtime-oracle tests -> acceptance `archive_bik_open_treats_0x800000_as_source_mode_not_audio_or_header_variant` -> do not encode `0x800000` as a BIK format/audio/parser flag.

## Negative Facts / Do Not Do

- Do not claim `gamemd.exe` directly validates BIK signatures, BIK revisions, audio descriptors, frame index offsets, or packet sizes in `FUN_00432750`; no such reads or callees appear in live decompile/callee evidence.
- Do not treat `FUN_005B4430` as a BIK parser; it is a filename/archive-index resolver using uppercase/CRC and entry metadata.
- Do not treat `0x800000` passed to `_BinkOpen` as audio, BIK revision, or parse behavior; in this function it is coupled to passing a Win32 handle.
- Do not collapse `_BinkOpen` failure into missing-file resolver failure; resolver/openability can succeed while Bink rejects the content and returns null.
- Do not use this report to prove byte-perfect Rust BIK decoding; the actual parser/decoder behavior belongs to BINKW32.DLL or runtime oracle tests, not `gamemd.exe`.

## Remaining Uncertainty

- Exact BINKW32.DLL validation behavior for corrupt, truncated, unsupported, or unusual BIK variants is not proven here. This report proves only that `gamemd.exe` delegates those bytes to `_BinkOpen`.
- The full `HBINK` field dependency map is broader than this slot. This report records only fields read by `0x00432750` immediately after open.
- Runtime/UI behavior after resolver success but `_BinkOpen` failure still needs the dedicated null-object/failure slot.
- Byte-equivalent Rust parser/decode proof still needs BINKW32.DLL comparison or runtime oracle fixtures; `gamemd.exe` does not provide those codec details.

## Stale Doc Wording

Supersede the PARTIAL status in `FUN_00432750_BINK_OPEN_INIT_GHIDRA_REPORT.md` for the direct-byte-read question with:

> Live Ghidra MCP evidence shows `FUN_00432750` does not directly parse BIK file/header bytes. It probes source availability, optionally resolves an archive entry and seeks a Win32 handle, calls `_BinkOpen@8`, and then reads returned `HBINK` fields. Rust's BIK parser validation must be treated as BINKW32/runtime-oracle behavior, not as a gamemd-side parser branch.

## Sources

- Live Ghidra MCP, `gamemd.exe`, image base `0x00400000`.
- Decompile: `0x00432750`, `0x00432690`, `0x004326C0`, `0x005B4430`, `0x0065CBF0`, `0x0065CB50`.
- Assembly context: `0x004327DE..0x00432849`, `0x00432852..0x004328CA`.
- Xrefs: `0x00432750` called from `0x004326B3` and `0x004326E9`.
- Import/callee evidence: `_BinkOpen@8` called only by `0x00432750`; `ReadFile` callers do not include `0x00432750`.
- Rust surface scan: `src/assets/bink_file.rs`, `src/assets/bink_audio.rs`, `src/render/bink_movie.rs`.
