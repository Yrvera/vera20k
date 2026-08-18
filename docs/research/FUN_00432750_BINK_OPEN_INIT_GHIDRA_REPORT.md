# FUN_00432750 Bink Open / Init - Ghidra Research Report

**Address(es):** `0x00432750` primary; caller constructors `0x00432690`, `0x004326C0`; Bink branch wrapper `0x005C07D0`; MSAnim caller `0x005CC760`  
**Investigation Mode:** exhaustive-slice attempted; **PARTIAL** because no live Ghidra MCP instance was available, so new verification used read-only local `gamemd.exe` PE disassembly plus prior Ghidra reports.  
**Claimed Scope:** open/init field reads, open flags, success/failure side effects, and Rust parser implications for active YR Bink wrappers.  
**Non-Scope:** per-frame update loop internals, explicit draw/copy format, loop/end `BinkGoto` semantics, BIK-before-VQA resolver details beyond liveness, full MSAnim behavior.  
**Confidence:** Medium-high for primary function instruction-level facts; High where prior Ghidra reports and local disassembly agree; Partial overall due missing live Ghidra decompile/xref tool.  
**Active in YR:** Yes for main-menu RA2TS path; Conditional for MSAnim users.

## 0. Working Notes Required Before Investigation

- **Target question:** What exactly does `FUN_00432750` pass to Bink open/init, which Bink handle fields does it consume, and what must Rust's BIK parser/video surface preserve?
- **Non-goals:** Do not re-investigate RA2TS asset choice, archive priority, per-frame cadence, explicit draw pixel format, or loop restart semantics except where needed to bound this open/init slice.
- **Evidence needed to mark COMPLETE:** live Ghidra decompile plus disassembly for `0x00432750`, caller/xref proof for active YR paths, exact object/Bink-field reads, failure behavior, and Rust surface scan.
- **Stop conditions:** stop if Ghidra MCP is unavailable after connection check; write a PARTIAL report from prior verified docs plus read-only binary disassembly, and list live-Ghidra-only gaps.

## 1. Overview

`FUN_00432750` is the concrete Bink object open/init routine called by both Bink constructors at `0x00432690` and `0x004326C0`. On the active main-menu path, `VQMovieHandle__Constructor @ 0x005C07D0` allocates a Bink object, calls `0x004326C0`, stores the object into a generic movie wrapper, and copies the opened Bink handle's width/height into wrapper fields.

The function is not a BIK container parser. It delegates decoding and validation to `binkw32.dll` via `_BinkOpen@8`, then consumes fields from the returned Bink handle. Rust differs architecturally because `src/assets/bink_file.rs` parses BIK headers directly before decoding; those parser validations are Rust-side guardrails, not proven gamemd wrapper checks.

## 2. Class Layout / Key Offsets

| Offset / field | Behavior in this slice | Active in YR | Evidence |
|---:|---|---|---|
| Bink object `+0x04` | Bink handle slot; written with `_BinkOpen` result; width/height/fps fields are read through this handle after success. | Yes | Local disasm `0x00432849-0x0043284F`, `0x0043289D-0x004328B5`, `0x004328BD-0x004328CA`; prior docs `MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md` section 4. |
| Bink handle `+0x00/+0x04` | Width/height read as 32-bit integers after open; `0x005C07D0` also copies them into generic wrapper `+0x08/+0x0C`. | Yes | Local disasm `0x004328C3-0x004328CA`; wrapper copy `0x005C08A6-0x005C08B4`. |
| Bink handle `+0x14/+0x18` | FPS numerator/divisor-like fields read with unsigned `DIV`; wrapper computes `object+0x24 = 60 / (handle[0x14] / handle[0x18])`. | Yes | Local disasm `0x0043289D-0x004328B5`; prior `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 6. |
| Bink object `+0x20` | BSurface/event-surface pointer; old value destroyed before new open, new BSurface created after success. | Yes | Local disasm cleanup `0x0043275B-0x00432772`, create/store `0x004328B8-0x004328C0`. |
| Bink object `+0x24` | Ticks per movie frame: unsigned integer result from `60 / integer_fps`. For RA2TS 15 fps, value is `4`. | Yes | Local disasm `0x004328A2-0x004328B5`; survey output `ra2ts_* 15 fps / 431 frames` in prior docs. |
| Bink object `+0x28` | Win32 file handle slot; constructor initializes `-1`; open stores `CreateFileA` handle only on archive/file-handle path; cleanup closes if not `-1`. | Yes | Constructor disasm `0x004326DB`; cleanup `0x00432786-0x00432795`; open `0x00432824-0x0043282D`. |
| Bink object `+0x2C/+0x2D` | Constructor defaults `+0x2C=1`, `+0x2D=0`; open/init itself does not parse these from the BIK header. | Yes | Constructor disasm `0x004326E2-0x004326E6`; prior playback report maps these to playing/force-frame flags. |
| Bink object `+0x30` | Last-frame marker cleared to `0` before open. | Yes | Local disasm `0x0043279C`. |

## 3. Core Logic

### 3.1 Open-Time Cleanup and Sound Setup

Active in YR: Yes, when the main-menu `0x71A` static receives the `0x4E4` movie assignment and constructs the Bink-backed wrapper.

Before opening a new file, the function destroys any existing `+0x20` BSurface pointer, closes any existing Bink handle at `+0x04` via `_BinkClose@4`, closes an existing Win32 handle at `+0x28` if it is not `-1`, and clears `+0x30`.

If `FUN_00407000()` returns nonzero, it calls `FUN_0040A7A0()` and then `_BinkSetSoundSystem@8(BinkOpenDirectSound, IDirectSound*)` before `_BinkOpen`. For main-menu RA2TS this does not create audio output because both RA2TS BIK files have zero audio tracks.

Evidence: local disasm `0x0043275B-0x004327B4`; prior `RA2TS_BINK_AUDIO_ENABLE_GHIDRA_REPORT.md` sections 2-4; local command `cargo run --quiet --bin bik-survey -- ra2ts --avsync` reported no audio track for both RA2TS assets.

### 3.2 File / Handle Open Flags

Active in YR: Yes. The active RA2TS path may resolve through archive-backed file access; the exact source priority is covered by the archive-priority report.

There are two `_BinkOpen@8` modes:

| Mode | `_BinkOpen` first arg | `_BinkOpen` flags | Evidence |
|---|---|---:|---|
| raw filename path | caller-provided filename/base-resolved path pointer | `0` | Local disasm `0x004327DE-0x004327E4` jumps to `_BinkOpen` call at `0x00432849`. |
| Win32 file-handle path | handle stored at object `+0x28` | `0x800000` | Local disasm `0x00432824-0x00432849`; prior audio report identifies this as `BINKOPENFILEHANDLE`, not a sound flag. |

The Win32 handle is opened read-only with `CreateFileA` argument sequence visible at `0x00432810-0x00432824`: desired access `0x80000000`, share mode `3`, creation disposition `3`, flags/attributes `0x8000080`, template `0`. The function then seeks the handle to the archive entry offset before `_BinkOpen` by calling the imported file-position API at `0x00432832-0x0043283A`.

### 3.3 Success Path Field Consumption

Active in YR: Yes for main-menu RA2TS; also shared by other Bink users that call these constructors.

After `_BinkOpen` succeeds:

- It copies global float volume `DAT_00A8EB9C` into `DAT_0089C490`, multiplies by constant `32768.0` at `0x007E3A70`, converts via `0x007C5F00`, and calls `_BinkSetVolume@8`.
- It computes integer FPS with unsigned division: `eax = handle[0x14] / handle[0x18]`, then `object+0x24 = 0x3C / eax`.
- It creates a BSurface via `0x006C99D0`, stores it at object `+0x20`, reads Bink handle `+0/+4` as width/height, and computes/stores the initial clipped rect at object `+0x10/+0x14/+0x18/+0x1C`.
- It calls `_BinkDDSurfaceType@4` using a field from `DAT_00887308` and stores the result at object `+0x08`.

Evidence: local disasm `0x00432877-0x00432A44`; `0x007E3A70` local PE read = IEEE float `32768.0`; prior `FUN_00432AB0_BINK_CLIP_RECT_SETTER_GHIDRA_REPORT.md` sections 4 and 8.

Important boundary detail: `FUN_00432750` does not guard zero or invalid Bink FPS fields before the unsigned `DIV` instructions. Any invalid header/FPS rejection is delegated to `_BinkOpen`/Bink itself before this point. Rust's parser currently rejects zero `fps_num` or `fps_den`; that may be a good safety check, but it is not a verified gamemd wrapper branch.

### 3.4 Failure Path

Active in YR: Conditional; only if `_BinkOpen` returns null.

If object `+0x04` is still null after the attempted open, the function calls `_BinkGetError@0`, logs string `Bink Error: %s\n` at `0x00818B2C`, returns `AL=0`, and does not run volume, fps, BSurface, clip, or DDSurfaceType setup.

Evidence: local disasm `0x00432852-0x00432874`; string local PE read at `0x00818B2C`; prior `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md` asset/load failure section.

## 4. INI Keys

No INI keys are read by `FUN_00432750`. Active in YR: not applicable. Asset selection and archive lookup are upstream systems, and audio availability is gated by initialized DirectSound globals, not by a BIK-specific INI key in this function.

## 5. Integration Points

| Function/path | Relationship | Active in YR | Evidence |
|---|---|---|---|
| `0x004326C0` constructor | Initializes Bink object defaults, stores caller surface at `+0x0C`, then calls `0x00432750`. | Yes for main-menu Bink branch | Local disasm `0x004326C0-0x004326EE`; branch from `0x005C07D0`. |
| `0x00432690` constructor | Alternate constructor with no explicit surface argument; calls `0x00432750`. | Conditional | Local xref scan found call at `0x004326B3`; liveness for standard RA2TS is not this constructor. |
| `VQMovieHandle__Constructor @ 0x005C07D0` | `.bik` branch allocates Bink object, calls `0x004326C0`, installs `vtable__BinkMovieHandle`, and copies width/height from Bink handle into generic wrapper. | Yes | Local disasm `0x005C0858-0x005C08B4`; prior `MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md` sections 2-4. |
| `MSAnim__Constructor @ 0x005CC760` | Conditional in-game Bink animation caller of `0x004326C0`; then calls clip setter. | Conditional | Local disasm `0x005CC7CE-0x005CC80B`; prior audio report section 6. |
| `OwnerDraw_Static_006153E0` | `0x4E4` message creates movie wrapper and starts timer. | Yes for main-menu `0x71A` | Prior `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` sections 2-4. |

## 6. Current Rust Implementation Status

Rust does not call Bink DLL APIs. It parses BIK files in `src/assets/bink_file.rs`, decodes frames in `src/assets/bink_decode.rs`, and presents the main-menu movie through `src/render/bink_movie.rs`.

Relevant current surfaces:

- `src/assets/bink_file.rs:145-157` reads BIK header tag, file size, frame count, largest frame, width, height, fps numerator/denominator, video flags, and audio track count.
- `src/assets/bink_file.rs:159-189` rejects zero frames, zero dimensions, zero fps numerator/denominator, excessive dimensions/tracks, and `largest_frame > file_size`.
- `src/assets/bink_file.rs:223-264` parses audio descriptors only when `num_audio_tracks > 0`; RA2TS has zero audio tracks.
- `src/render/bink_movie.rs:36-46` parses, creates a decoder, decodes frame 0 immediately, uploads a texture at header width/height, and sets `current_frame = 1`.
- `src/render/bink_movie.rs:85-90` uses `header.fps()` as a floating schedule. This is not the same as gamemd's open-time `object+0x24 = 60 / integer_fps`, but per-frame cadence is slot 3 scope.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00432750` cleanup/open/success/failure sequence | verified-with-local-disasm | Local disasm `0x00432750-0x00432A50`; prior Ghidra docs | Live Ghidra decompile should be re-run to cross-check register naming and xrefs. |
| `_BinkOpen` raw filename vs file-handle flags | verified-with-local-disasm | `0x004327DE-0x00432849`; prior audio report | None for flag values; archive lookup details owned by slot 2. |
| Width/height handle fields | verified-with-local-disasm | `0x004328C3-0x004328CA`; wrapper copy `0x005C08A6-0x005C08B4` | None for offsets `+0/+4`; pixel copy use owned by slot 4. |
| FPS handle fields and division signedness | verified-with-local-disasm | unsigned `DIV` at `0x004328A2-0x004328B1` | Need live Ghidra decompile only for naming, not mechanism. |
| Frame count/current frame handle fields | touched-not-exhausted | Prior playback report: end test uses handle `+0x08/+0x0C`; not primary open code | Slot 5 owns loop/end exactness. |
| Audio track header consumption | verified for RA2TS asset outcome; not consumed directly by wrapper | `bik-survey --avsync`; prior audio report; Rust `bink_file.rs:157,223-264` | Audio-bearing Bink files are separate follow-up. |
| Corrupt BIK/open failure after wrapper allocation | touched-not-exhausted | Local disasm failure return; prior full-composition report marks exact UI outcome medium | Runtime behavior for corrupt-but-resolved BIK remains open. |
| MSAnim caller activity | deferred | Local disasm `0x005CC7CE-0x005CC80B`; prior report says conditional | Requires separate MSAnim liveness investigation. |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is the main-menu RA2TS path active in standard YR?` -> Yes; `0x005C07D0` `.bik` branch constructs a Bink wrapper for `0x71A`, and prior reports prove main-menu state `0x12` sends `0x4E4` with `Ra2ts_s/l`. (evidence: `MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md` sections 2-4; local disasm `0x005C0858-0x005C08B4`)
- `[RESOLVED] OQ-2 - What flags are passed to _BinkOpen?` -> `0` for filename path; `0x800000` for Win32 file-handle path. (evidence: local disasm `0x004327DE-0x00432849`; prior `RA2TS_BINK_AUDIO_ENABLE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-3 - Is `0x800000` an audio flag?` -> No; it is used only when the first `_BinkOpen` argument is the Win32 handle stored at object `+0x28`. (evidence: local disasm `0x00432824-0x00432849`; prior audio report)
- `[RESOLVED] OQ-4 - Which Bink handle fields does open/init read for dimensions?` -> 32-bit fields at handle `+0/+4`. (evidence: local disasm `0x004328C3-0x004328CA`; wrapper copy `0x005C08A6-0x005C08B4`)
- `[RESOLVED] OQ-5 - Which Bink handle fields does open/init read for fps?` -> 32-bit fields at handle `+0x14/+0x18`, using unsigned integer division before `60 / fps`. (evidence: local disasm `0x0043289D-0x004328B5`)
- `[RESOLVED] OQ-6 - Does wrapper code validate zero fps before division?` -> No wrapper guard was found in `0x00432750`; invalid FPS must be rejected inside `_BinkOpen` or would fault in wrapper arithmetic. (evidence: local disasm `0x0043289D-0x004328B5`)
- `[RESOLVED] OQ-7 - Are RA2TS files audio-bearing?` -> No; both report zero audio tracks. (evidence: `cargo run --quiet --bin bik-survey -- ra2ts --avsync`, 2026-05-27)
- `[RESOLVED] OQ-8 - Does open/init parse raw BIK header fields itself?` -> No; it consumes a returned Bink handle from `_BinkOpen`. (evidence: local disasm `0x00432849-0x004328CA`)
- `[RESOLVED] OQ-9 - What does failure do?` -> Logs `_BinkGetError` string and returns `0` before success setup. (evidence: local disasm `0x00432852-0x00432874`; string `0x00818B2C`)
- `[DEFERRED] OQ-10 - What exact UI state results if `_BinkOpen` fails after wrapper allocation?` (category: needs-runtime-debugger; reason: prior report marks no fallback high but exact blank/crash outcome medium; next-step-if-pursued: force corrupt RA2TS in retail/runtime and trace static vtable calls)
- `[DEFERRED] OQ-11 - Are MSAnim Bink users active in standard YR scenarios and audio-bearing?` (category: requires-different-system-context; reason: outside main-menu open/init target; next-step-if-pursued: dedicated MSAnim liveness trace)
- `[DEFERRED] OQ-12 - Does BinkGoto(1) correspond to Rust packet index 0 or another visible frame after Bink internals?` (category: out-of-scope; reason: loop/end slot owns vtable `+0x14/+0x1C`; next-step-if-pursued: slot 5 trace of `_BinkGoto(handle, 1, 1)` and first copied frame)

## 9. Visual/UI Composition Ledger

This report touches a visual movie open path but does not claim full composition. The only visual-open facts in scope are:

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `0x005C07D0` -> `0x004326C0` -> `0x00432750` | `.bik` branch after resolver finds RA2TS BIK | `ra2ts_s.bik` or `ra2ts_l.bik`; no frame copy yet | opens and seeds size/clip fields | Bink DDS surface type only | Yes | movie object creation |
| 2 | `0x00432750` success setup | `_BinkOpen` non-null | Bink handle width/height | initial clipped rect into object `+0x10..+0x1C` | `_BinkDDSurfaceType` stored at `+0x08` | Yes | downstream frame-copy setup |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `ra2ts_s.bik` | Yes at width 640 | Not by this function | Yes after later update/draw path | Content | No | No | No | No | Prior main-menu reports; survey output |
| `ra2ts_l.bik` | Yes at non-640 width | Not by this function | Yes after later update/draw path | Content | No | No | No | No | Prior main-menu reports; survey output |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `_BinkOpen` wrapper accepts filename mode with flags `0` and file-handle mode with flags `0x800000`; the latter is not an audio flag. | Local disasm `0x004327DE-0x00432849`; prior audio report. | Rust parses bytes from `AssetManager` and has no Bink DLL open flags; no parser delta for RA2TS. | `src/assets/bink_file.rs`, `src/render/bink_movie.rs`, asset loading code. | Keep archive/file resolution separate from audio enablement; do not infer audio from `0x800000`. | Load RA2TS from archive bytes and assert zero audio tracks and successful video parse. Proposed test: `bink_open_filehandle_flag_does_not_imply_audio`. | Treating `0x800000` as sound-related would create false audio work and wrong diagnostics. |
| Open/init consumes Bink handle width/height at `+0/+4`, and wrapper `0x005C07D0` copies those to generic width/height fields. | Local disasm `0x004328C3-0x004328CA`, `0x005C08A6-0x005C08B4`. | Rust uses BIK header width/height and uploads texture at those dimensions; matches RA2TS observed values. | `src/assets/bink_file.rs:152-153`, `src/render/bink_movie.rs:41-65`. | Parser must preserve exact RA2TS dimensions `632x570` and `472x450`, and renderer must not scale as a substitute for native size. | Survey/load both RA2TS assets and assert dimensions. Proposed test: `ra2ts_open_dimensions_match_bink_handle_fields`. | Do not use dialog template rect `304x266` or shell panel size as movie dimensions. |
| Open/init derives `object+0x24` as unsigned integer `60 / (handle[0x14] / handle[0x18])`; no wrapper-side zero-fps guard exists. | Local disasm `0x0043289D-0x004328B5`; prior playback report section 6. | Rust parser rejects zero fps and `BinkMovieSurface` uses floating `fps()`; exact timer/catch-up equivalence is slot 3. | `src/assets/bink_file.rs:154-179`, `src/render/bink_movie.rs:68-90`. | For RA2TS, parsed fps must be `15/1` and a derived gamemd-open tick value must be `4` if modeled/tested. | Add focused pure test deriving gamemd open ticks from parsed header. Proposed test: `ra2ts_open_ticks_per_frame_uses_integer_bink_fps`. | Do not replace gamemd's `60 / integer_fps` open field with the owner-draw `0x22 ms` timer interval. |

### Negative Facts / Do Not Do

- Do not implement `0x800000` as an audio/sound flag. Evidence: it is only pushed after loading object `+0x28` Win32 handle at `0x00432840-0x00432849`; prior audio report found no `BINKSND`/sound-on import.
- Do not make Rust depend on RA2TS audio output. Evidence: `cargo run --quiet --bin bik-survey -- ra2ts --avsync` reports no audio track for `ra2ts_l.bik` and `ra2ts_s.bik`.
- Do not size the RA2TS movie from dialog static `0x71A` template dimensions. Evidence: Bink wrapper copies opened handle width/height at `0x005C08A6-0x005C08B4`; prior visual-assets report notes runtime replaces template sizing with movie dimensions.
- Do not treat Rust parser validation bounds as proven gamemd wrapper branches. Evidence: `FUN_00432750` reads the returned Bink handle and performs no raw-header validation before width/height/fps use; Bink DLL owns validation.
- Do not use this report to settle `BinkGoto(handle, 1, 1)` frame-index semantics. Evidence: this report only covers open/init; loop reset is vtable `+0x1C` / slot 5 scope.

### Stale Docs / Follow-up Docs

- `docs/plans/2026-05-17-initial-main-menu-dialog-0xe2-plan.md`: replace any wording that treats `BinkGoto(1)` as proven Rust decoder packet index `0` with: "Native loop reset calls `_BinkGoto(handle, 1, 1)`; the mapping from Bink's 1-based frame argument to the first Rust packet/frame after reset is not proven by the open/init report and must be verified by the loop/end investigation."

## Sources

- Local read-only PE disassembly of `<ra2-install>/gamemd.exe` using Capstone; image base `0x00400000`.
- Local disassembly ranges: `0x00432690-0x004326F1`, `0x00432750-0x00432A50`, `0x005C0858-0x005C08B4`, `0x005CC7CE-0x005CC80B`.
- Local PE constant/string reads: `0x007E3A70` = float `32768.0`; `0x00818B2C` = `Bink Error: %s\n`.
- `docs/research/RA2TS_BINK_AUDIO_ENABLE_GHIDRA_REPORT.md`
- `docs/research/MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`
- `docs/research/MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md`
- `docs/research/FUN_00432AB0_BINK_CLIP_RECT_SETTER_GHIDRA_REPORT.md`
- `docs/research/MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- Rust files scanned read-only: `src/assets/bink_file.rs`, `src/assets/bink_decode.rs`, `src/render/bink_movie.rs`, `src/bin/bik-survey.rs`, `src/bin/bik-player.rs`
- Local commands: `cargo run --quiet --bin bik-survey -- ra2ts`; `cargo run --quiet --bin bik-survey -- ra2ts --avsync`
