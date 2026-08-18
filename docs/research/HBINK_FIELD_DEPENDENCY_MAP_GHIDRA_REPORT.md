# HBINK Field Dependency Map - Ghidra Research Report

**Address(es):** `0x00432750`, `0x00432BF0`, `0x00432C50`, `0x00432C70`, `0x00432E40`, `0x00432AB0`, `0x00433020`, `0x00433060`, `0x004331F0`, unlabeled helper bytes `0x004333F0..0x0043355F`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** direct `gamemd.exe` reads/writes through the `HBINK` pointer stored at Bink object `+0x04`, plus Bink API calls that receive the handle.  
**Non-Scope:** BINKW32 internal struct layout, BIK bitstream/header validation inside BINKW32, VQA internals, pixel decoder equivalence, and runtime debugger proof for corrupt files.  
**Confidence:** High for listed direct HBINK fields and call sites; Medium for negative "no other HBINK fields" wording, bounded to the known Bink object/vtable helpers and BINKW32 import-call search.  
**Active in YR:** Yes for owner-draw/fullscreen Bink movie paths after `.BIK` resolution; conditional for less common MSAnim/WDT users.

## 0. Working Notes

- **Target question:** Which fields of the `HBINK` returned by `_BinkOpen` does `gamemd.exe` directly read/write, and which Rust parser/decoder metadata is therefore a gamemd-facing contract rather than only a Rust-side parser detail?
- **Non-goals:** Do not reverse BINKW32 internals, prove the BIK codec, decode audio/video samples, re-open BIK/VQA resolver ordering, or decide `BinkGoto(1)` frame-index semantics.
- **Evidence needed to mark COMPLETE:** live Ghidra decompile plus assembly for all known Bink object helpers, import-call search for Bink API use, direct offset map, Rust surface scan, and final open-question ledger with no unresolved in-scope items.
- **Stop conditions:** stop after every direct HBINK subfield read in known Bink helpers is mapped; defer SDK/runtime questions and unrecognized BINKW32 internals instead of expanding into DLL reversing.

## 1. Overview

`gamemd.exe` does not parse BIK header bytes directly on the active Bink playback path. It asks BINKW32 to open the file or Win32 file handle, stores the returned `HBINK` at Bink object `+0x04`, and then directly reads only six observed handle fields: `+0x00`, `+0x04`, `+0x08`, `+0x0C`, `+0x14`, and `+0x18`.

No direct `gamemd.exe` writes to the `HBINK` struct were found in this slice. State mutation of the handle is delegated to BINKW32 calls such as `_BinkDoFrame`, `_BinkNextFrame`, `_BinkGoto`, `_BinkPause`, `_BinkSetVolume`, and `_BinkClose`.

## 2. HBINK Field Map

| HBINK offset | Direct role in `gamemd.exe` | Access kind | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x00` | Bink image width. Used for centering/clipping and wrapper width copy. | read u32/i32 | `0x004328BD..0x004328CA`, `0x00432AB3..0x00432AC8`; prior wrapper copy `0x005C08A6..0x005C08B4` | Yes |
| `+0x04` | Bink image height. Used for centering/clipping and wrapper height copy. | read u32/i32 | `0x004328C3..0x004328CD`, `0x00432AB3..0x00432AC8`; prior wrapper copy `0x005C08A6..0x005C08B4` | Yes |
| `+0x08` | Total/upper-bound frame marker for finished/blocking-loop predicates. | read unsigned u32 | `0x00432C50..0x00432C5D`, `0x00432CB8..0x00432CC3` | Yes |
| `+0x0C` | Current frame/position marker. Used for finished predicate, blocking loop, per-frame BSurface scratch key, and last-marker capture. | read unsigned u32 | `0x00432C50..0x00432C64`, `0x00432CB8..0x00432CCC`, `0x00432F75..0x00432F80`, `0x00432FF1..0x00432FF8` | Yes |
| `+0x14` | FPS numerator-like field. Divided by `+0x18` with unsigned `DIV`. | read unsigned u32 | open/init `0x0043289D..0x004328B5`; frame-delay helper `0x00432BF0..0x00432C06` | Yes |
| `+0x18` | FPS denominator/divisor-like field. Divisor for unsigned `DIV`. | read unsigned u32 | open/init `0x0043289D..0x004328B5`; frame-delay helper `0x00432BF0..0x00432C06` | Yes |

No direct reads were found for BINKW32 audio-track descriptors, codec flags, largest-frame sizes, frame-table offsets, packet offsets, or compressed audio/video payload bytes through `HBINK`.

## 3. Core Logic

### 3.1 Handle Creation And Storage

Active in YR: Yes for `.BIK` playback paths.

`0x00432750` is the only `_BinkOpen` call found by the exact import-call pattern `FF 15 9C 15 7E 00`; the call is at `0x00432849`. The returned handle is stored directly into Bink object `+0x04` at `0x0043284F`.

Constructors `0x00432690` and `0x004326C0` initialize `+0x04 = 0`, `+0x20 = 0`, `+0x28 = -1`, `+0x2C = 1`, `+0x2D = 0`, then call `0x00432750`. `0x004326C0` additionally stores the target surface at object `+0x0C` before opening.

Failure detail: if `_BinkOpen` leaves object `+0x04` null, `0x00432750` logs `_BinkGetError()` and returns `0` before any HBINK field read. Exact corrupt/resolved caller behavior is covered by `BINKOPEN_FAILURE_NULL_OBJECT_BEHAVIOR_GHIDRA_REPORT.md`.

### 3.2 Dimensions: HBINK `+0x00/+0x04`

Active in YR: Yes.

Open/init reads width and height only after `_BinkOpen` success:

- `0x004328BD`: reloads handle from object `+0x04`.
- `0x004328C3`: reads `[HBINK+0x00]` into the width path.
- `0x004328C7`: reads `[HBINK+0x04]` into the height path.

Clip setter `0x00432AB0` repeats the same pair:

- `0x00432AB3`: handle = `[object+0x04]`.
- `0x00432AC3`: `EBP = [HBINK+0x04]`.
- `0x00432AC7`: `ESI = [HBINK+0x00]`.

These fields feed object rect `+0x10/+0x14/+0x18/+0x1C`; they are not written back to the handle.

### 3.3 FPS: HBINK `+0x14/+0x18`

Active in YR: Yes.

Open/init computes object `+0x24` using unsigned integer division:

```text
0x0043289D: ECX = [object+0x04]
0x004328A2: EAX = [HBINK+0x14]
0x004328A5: DIV dword ptr [HBINK+0x18]
0x004328AC: EAX = 0x3C
0x004328B1: DIV ECX
0x004328B5: [object+0x24] = EAX
```

The vtable frame-delay helper `0x00432BF0` computes `1000 / ([HBINK+0x14] / [HBINK+0x18])` the same way with unsigned `DIV`:

```text
0x00432BF0: ECX = [object+0x04]
0x00432BF5: EAX = [HBINK+0x14]
0x00432BF8: DIV dword ptr [HBINK+0x18]
0x00432BFF: EAX = 0x3E8
0x00432C04: DIV ECX
```

No wrapper-side zero guard was found before either division. BINKW32 must reject invalid FPS metadata, or this wrapper would fault.

### 3.4 Current/Total Markers: HBINK `+0x08/+0x0C`

Active in YR: Yes.

Finished predicate `0x00432C50` reads:

- `0x00432C54`: `EDX = [HBINK+0x0C]`, current marker.
- `0x00432C57`: `ESI = [HBINK+0x08]`, total/upper marker.
- `0x00432C5A..0x00432C62`: unsigned `current >= total` or `current < object+0x30` means finished/wrapped.

Blocking fullscreen loop `0x00432C70` repeats that predicate inline:

- `0x00432CB8`: handle = `[object+0x04]`.
- `0x00432CBB`: current = `[HBINK+0x0C]`.
- `0x00432CBE`: total = `[HBINK+0x08]`.
- `0x00432CC1..0x00432CCC`: unsigned end/wrap exits the loop.

Per-frame update `0x00432E40` also reads current marker `+0x0C`:

- `0x00432F75..0x00432F80`: passes `[HBINK+0x0C] * object+0x24` to `0x006C9C60` when object `+0x20` exists.
- `0x00432FF1..0x00432FF8`: writes Bink object `+0x30 = [HBINK+0x0C]` immediately before `_BinkNextFrame`.

This means Rust cannot model the loop as only `current_frame >= frame_count()`: native has both total-bound and wrap/backwards-marker detection.

### 3.5 API-Only HBINK Uses

Active in YR: Yes for the Bink paths that reach these helpers.

The following helpers pass object `+0x04` to BINKW32 but do not directly read HBINK subfields:

| Helper | Direct Bink API use | Direct HBINK field read? | Evidence |
|---|---|---|---|
| `0x00432700`, `0x00432A60` cleanup variants | `_BinkClose(handle)` | No | `_BinkClose` exact-call pattern at `0x0043272A`, `0x00432A8A` |
| `0x00432BD0` restart | `_BinkGoto(handle, caller_frame, 1)` | No | `0x00432BD7..0x00432BE4` |
| `0x00433020` force-if-ready helper | `_BinkWait(handle)`, sets object `+0x2D` if ready | No | `0x00433023..0x00433031` |
| `0x00433060` explicit draw | `_BinkCopyToBuffer(handle, ...)` | No | `0x00433151..0x00433155` |
| `0x004331F0` copy helper | `_BinkCopyToBuffer(handle, ...)` | No | decompile `0x004331F0`, copy call pattern |
| unlabeled helper bytes `0x004333F0..0x0043355F` | `_BinkPause`, `_BinkCopyToBuffer` | No observed direct subfield read | Ghidra bytes plus local Capstone listing from `0x004333F0..0x0043355F` |

`_BinkSetVolume`, `_BinkPause`, `_BinkDoFrame`, `_BinkNextFrame`, `_BinkWait`, `_BinkGoto`, `_BinkCopyToBuffer`, and `_BinkClose` may read/write BINKW32's internal handle data, but those mutations are outside `gamemd.exe`.

## 4. INI Keys

No INI key is read in this HBINK field slice. Movie name resolution, `[Movies]`, campaign `FinalMovie`, and `MovieOn`/`MovieOff` are upstream or sibling systems, covered in the broader BIK/Bink ecosystem reports.

## 5. Integration Points

| Function / path | Relationship | Active in YR | Evidence |
|---|---|---|---|
| `0x00432690`, `0x004326C0` | Constructors initialize object fields then call open/init. | Yes/Conditional depending caller | xrefs to `0x00432750` at `0x004326B3`, `0x004326E9` |
| `0x00432750` | Only observed `_BinkOpen` call; stores returned handle at object `+0x04`. | Yes | exact byte-pattern search `FF 15 9C 15 7E 00` -> `0x00432849` |
| `0x00432C70` | Fullscreen blocking playback loop; inlines current/total/wrap predicate. | Yes for fullscreen BIK playback | Movies/Credits report; direct disassembly `0x00432CB8..0x00432CCC` |
| `0x00432E40` | Owner-draw update loop; calls Bink SDK and captures last current marker. | Yes for RA2TS/menu Bink handle | update-loop report; direct disassembly `0x00432F75..0x00433005` |
| `0x00432BF0` | Frame-delay vtable helper; reads FPS fields. | Conditional but present in Bink vtable thunk | xref from `0x005C05E3`; disassembly `0x00432BF0..0x00432C06` |

## 6. Current Rust Implementation Status

Rust currently parses BIK bytes directly:

- `src/assets/bink_file.rs:63..75` exposes `num_frames`, `width`, `height`, `fps_num`, `fps_den`, `video_flags`, `num_audio_tracks`, and `audio_tracks`.
- `src/assets/bink_file.rs:90..95` computes floating FPS as `fps_num / fps_den`.
- `src/render/bink_movie.rs:36..46` parses the file, decodes packet 0 immediately, creates a texture at parsed width/height, and starts `current_frame = 1`.
- `src/render/bink_movie.rs:72..73` returns `frame_index.len()` for frame count.
- `src/render/bink_movie.rs:90..103` advances with elapsed-time `frames_due(..., 4)` and a Rust `current_frame` index.

The direct-gamemd metadata contract is narrower than Rust's parser surface: width, height, FPS numerator/divisor, total/upper marker, current marker, and the ability to expose/update Bink-style current/last-marker semantics. Audio track descriptors and packet/frame table parsing are needed for Rust's decoder, but they are not directly consumed by `gamemd.exe`; byte/parity proof for those belongs to BINKW32/runtime oracle tests.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `_BinkOpen` result storage | verified | `0x00432849..0x0043284F`; exact byte-pattern search found only this call | none |
| HBINK dimensions `+0/+4` | verified | `0x004328BD..0x004328CA`, `0x00432AB3..0x00432AC8` | none |
| HBINK FPS `+0x14/+0x18` | verified | `0x0043289D..0x004328B5`, `0x00432BF0..0x00432C06` | none |
| HBINK total/current `+0x08/+0x0C` | verified | `0x00432C50..0x00432C67`, `0x00432CB8..0x00432CCC`, `0x00432F75..0x00433005` | SDK exact names remain inferred from role |
| Direct HBINK writes by `gamemd.exe` | verified-not-found in scoped helpers | decompile/disassembly of known Bink object/vtable helpers | BINKW32 internal writes out of scope |
| Audio-related HBINK field reads | verified-not-found in scoped helpers | audio path uses `_BinkSetVolume`/`_BinkPause`; no direct audio subfield read found | BINKW32 audio internals out of scope |
| Unlabeled helper `0x004333F0..0x0043355F` | touched | Bink import-call byte hits decoded locally from Ghidra memory bytes | Ghidra function boundary absent; exact vtable owner belongs to slot-map/API inventory if needed |
| Raw BIK header field reads by gamemd | corroborated by sibling slot, not re-proven globally here | `FUN_00432750_DIRECT_BIK_BYTE_READS_GHIDRA_REPORT.md`, `BINKW32_IMPORT_BOUNDARY_NO_INTERNAL_CODEC_PARSER_GHIDRA_REPORT.md` | DLL/runtime parser proof out of scope |

## 8. Open Questions - Final State

- `[RESOLVED] HBINK-001 - Where is the HBINK stored? -> Bink object +0x04 receives _BinkOpen return.` (evidence: `0x00432849..0x0043284F`)
- `[RESOLVED] HBINK-002 - Are width/height read directly? -> Yes, HBINK +0/+4 are read after open and in clip setup.` (evidence: `0x004328BD..0x004328CA`, `0x00432AB3..0x00432AC8`)
- `[RESOLVED] HBINK-003 - Are FPS fields read directly? -> Yes, HBINK +0x14/+0x18 are read with unsigned DIV in open/init and frame-delay helper.` (evidence: `0x0043289D..0x004328B5`, `0x00432BF0..0x00432C06`)
- `[RESOLVED] HBINK-004 - Are current/total markers read directly? -> Yes, +0x0C and +0x08 drive finished/blocking predicates; +0x0C also seeds object +0x30 after decode.` (evidence: `0x00432C50..0x00432C67`, `0x00432CB8..0x00432CCC`, `0x00432FF1..0x00432FF8`)
- `[RESOLVED] HBINK-005 - Does gamemd write HBINK fields directly? -> No direct HBINK write was found in scoped Bink object/vtable helpers; mutations are via BINKW32 APIs.` (evidence: decompile/disassembly of listed helpers)
- `[RESOLVED] HBINK-006 - Does gamemd read audio-track fields from HBINK? -> No direct audio metadata field read was found; audio uses DirectSound registration, _BinkSetVolume, and _BinkPause API calls.` (evidence: `0x0043279F..0x004327B4`, `0x00432877..0x00432897`, `0x00432E40..0x00432E7A`)
- `[RESOLVED] HBINK-007 - Does object +0x24 equal raw FPS? -> No, it is 60 divided by integer FPS, not the FPS itself.` (evidence: `0x0043289D..0x004328B5`)
- `[RESOLVED] HBINK-008 - Does frame-delay helper use object +0x24? -> No, `0x00432BF0` recomputes 1000 divided by integer FPS from HBINK +0x14/+0x18.` (evidence: `0x00432BF0..0x00432C06`)
- `[RESOLVED] HBINK-009 - Does fullscreen playback use the same current/total predicate? -> Yes, `0x00432C70` inlines the current >= total OR current < last-marker exit test.` (evidence: `0x00432CB8..0x00432CCC`)
- `[RESOLVED] HBINK-010 - Does Rust expose all directly consumed metadata? -> Partly: width/height/fps/frame count exist, but native current-marker/wrap semantics are not represented as BINKW32-populated state.` (evidence: `src/assets/bink_file.rs:63..95`, `src/render/bink_movie.rs:72..103`)
- `[DEFERRED] HBINK-011 - What are the official BINKW32 SDK names for HBINK +0x08/+0x0C/+0x14/+0x18?` (category: requires-different-system-context; reason: gamemd proves roles, not SDK struct names; next-step-if-pursued: inspect BINKW32 headers/DLL oracle)
- `[DEFERRED] HBINK-012 - Does BINKW32 ever populate current marker differently from Rust packet index on corrupt/variable-rate files?` (category: needs-runtime-debugger; reason: requires runtime stepping inside BINKW32; next-step-if-pursued: BINKW32/runtime oracle test)
- `[DEFERRED] HBINK-013 - Exact raw parser equivalence for audio descriptors and packet tables.` (category: requires-different-system-context; reason: gamemd does not read those fields directly; next-step-if-pursued: BINKW32 fixture/oracle tests)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `gamemd` consumes HBINK dimensions `+0/+4` after `_BinkOpen` and in clip setup. | `0x004328BD..0x004328CA`, `0x00432AB3..0x00432AC8` | Rust exposes parsed header width/height and uses them for texture creation. | `src/assets/bink_file.rs:69..70`, `src/render/bink_movie.rs:41..65` | Keep movie dimensions sourced from Bink metadata, not dialog/static rects. | RA2TS large/small assets load with exact Bink dimensions and no UI-template substitution. Test: `bink_movie_dimensions_follow_bink_metadata_not_static_rect`. | Do not substitute shell/static template dimensions for movie texture dimensions. |
| `gamemd` computes integer-FPS-derived timing fields from HBINK `+0x14/+0x18` using unsigned integer division. | `0x0043289D..0x004328B5`, `0x00432BF0..0x00432C06` | Rust exposes floating `fps()` and update loop uses elapsed accumulator. | `src/assets/bink_file.rs:90..95`, `src/render/bink_movie.rs:68..90` | Add/keep a gamemd-style integer FPS/tick derivation for parity tests; do not treat floating FPS scheduling as native mechanism. | For RA2TS 15/1, derived open tick delay is 4 and frame-delay helper is 66 ms by integer division. Test: `bink_metadata_derives_native_integer_fps_delays`. | Do not claim `frames_due` floating accumulator is equivalent to BINKW32 wait-gated timing. |
| Native finished/blocking predicates use HBINK current marker `+0x0C`, total marker `+0x08`, and object last marker `+0x30` with unsigned wrap detection. | `0x00432C50..0x00432C67`, `0x00432CB8..0x00432CCC`, marker capture `0x00432FF1..0x00432FF8` | Rust tracks `current_frame` as a packet index and checks only against frame count in `step`. | `src/render/bink_movie.rs:72..103`, restart path `src/render/bink_movie.rs:137..140` | Model/test current/total/last-marker semantics separately from raw packet index when implementing native movie loop parity. | A synthetic marker state with current below last marker reports finished/wrapped even if current is below total. Test: `bink_finished_detects_bink_marker_wrap_not_only_frame_count`. | Do not collapse native end/wrap behavior to `current_frame >= frame_count()`. |

### Negative Facts / Do Not Do

- Do not implement BIK audio-track parsing because `gamemd.exe` directly reads audio descriptor fields. It does not in this slice; audio control is via BINKW32 calls. Evidence: Bink audio reports plus `0x0043279F..0x004327B4`, `0x00432877..0x00432897`, `0x00432E40..0x00432E7A`.
- Do not treat Rust parser validation of BIK header fields as a gamemd wrapper branch. The wrapper consumes only the returned `HBINK` fields after `_BinkOpen`. Evidence: `_BinkOpen` call/store `0x00432849..0x0043284F`, first metadata reads after success `0x0043289D+`.
- Do not use signed comparisons for current/total marker behavior. Evidence: finished predicate uses unsigned `JNC` and `JC` at `0x00432C5D` and `0x00432C62`.
- Do not use object `+0x24` as FPS. It is `60 / integer_fps`; frame-delay helper separately computes `1000 / integer_fps`. Evidence: `0x004328A2..0x004328B5`, `0x00432BF5..0x00432C04`.
- Do not make a single Rust `frame_count` enough for native loop parity. Native reads both total and current markers from BINKW32 state and also compares current against last marker. Evidence: `0x00432C50..0x00432C67`.

### Remaining Uncertainty

- Official BINKW32 field names for offsets `+0x08`, `+0x0C`, `+0x14`, and `+0x18` remain role-derived from gamemd use, not SDK-header verified.
- BINKW32 internal updates to current/total markers, corrupt-file behavior after successful open, and variable-rate/edge-file semantics require runtime/DLL oracle testing.
- Rust's audio packet parsing and decoder details are necessary for VERA20k, but gamemd does not provide a direct parser spec for them; they must be proven against BINKW32 or fixtures.

### Stale Docs / Follow-up Docs

- In `docs/research/FUN_00432750_BINK_OPEN_INIT_GHIDRA_REPORT.md`, replace "Frame count/current frame handle fields: touched-not-exhausted" with: "HBINK field dependency map is consolidated in `HBINK_FIELD_DEPENDENCY_MAP_GHIDRA_REPORT.md`: gamemd directly reads HBINK `+0x08` as total/upper marker and `+0x0C` as current marker in `0x00432C50`, `0x00432C70`, and `0x00432E40`; no direct audio descriptor or payload fields are read by gamemd."
- In any implementation note that says Rust `BinkHeader` is the gamemd parser contract, replace with: "Rust `BinkHeader` is a decoder/parser contract for VERA20k. The direct `gamemd.exe` wrapper contract is the BINKW32-populated HBINK metadata subset: width, height, integer FPS numerator/divisor, total marker, and current marker."

## Sources

- Ghidra MCP decompile/disassembly: `0x00432690`, `0x004326C0`, `0x00432700`, `0x00432750`, `0x00432A60`, `0x00432AB0`, `0x00432BD0`, `0x00432BF0`, `0x00432C10`, `0x00432C50`, `0x00432C70`, `0x00432E40`, `0x00433020`, `0x00433060`, `0x004331F0`, `0x00433270`, `0x00433330`.
- Ghidra MCP import-call byte-pattern searches: `_BinkOpen` `FF 15 9C 15 7E 00`; `_BinkSetVolume` `FF 15 A0 15 7E 00`; `_BinkClose` `FF 15 A4 15 7E 00`; `_BinkDDSurfaceType` `FF 15 A8 15 7E 00`; `_BinkGoto` `FF 15 AC 15 7E 00`; `_BinkPause` `FF 15 B0 15 7E 00`; `_BinkNextFrame` `FF 15 B4 15 7E 00`; `_BinkCopyToBuffer` `FF 15 B8 15 7E 00`; `_BinkDoFrame` `FF 15 BC 15 7E 00`; `_BinkWait` `FF 15 C0 15 7E 00`.
- Ghidra MCP memory read of unlabeled helper bytes `0x00433380..0x0043357F`, locally disassembled read-only with Capstone for `0x004333F0..0x0043355F`.
- Prior reports: `FUN_00432750_DIRECT_BIK_BYTE_READS_GHIDRA_REPORT.md`, `BINKOPEN_FAILURE_NULL_OBJECT_BEHAVIOR_GHIDRA_REPORT.md`, `BINKW32_IMPORT_BOUNDARY_NO_INTERNAL_CODEC_PARSER_GHIDRA_REPORT.md`, `BINK_FINISHED_PREDICATE_0X00432C50_GHIDRA_REPORT.md`, `BINK_UPDATE_LOOP_0X00432E40_FRESH_MCP_AUDIT_GHIDRA_REPORT.md`, `AUDIO_BEARING_BIK_PATH_AND_VOLUME_GHIDRA_REPORT.md`.
- Rust scan: `src/assets/bink_file.rs`, `src/render/bink_movie.rs`.
