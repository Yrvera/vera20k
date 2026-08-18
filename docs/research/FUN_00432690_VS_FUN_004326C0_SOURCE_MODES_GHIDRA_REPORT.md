# FUN_00432690 vs FUN_004326C0 Source Modes - Ghidra Research Report

**Address(es):** `0x00432690`, `0x004326C0`, shared open helper `0x00432750`, archive lookup helper `0x005B4430`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Compare the two Bink object constructors enough to prove their exact source-mode contract, pre-open field initialization, caller ownership, and the `_BinkOpen` source/flag path selected by `0x00432750`.  
**Non-Scope:** Bink frame-loop cadence, BinkGoto restart semantics, VQA decoder internals, full MSAnim playback liveness, and runtime corrupt-BIK visible outcome.  
**Confidence:** High for constructor fields, call relationships, and `_BinkOpen` argument modes; Medium-high for the archive helper's returned structure naming because Ghidra has no applied struct names, but the data flow is verified.  
**Active in YR:** Yes for fullscreen Movies/Sneak Preview through `0x00432690`; Yes for owner-draw BinkMovieHandle through `0x004326C0`; Conditional for MSBinkAnim through `0x004326C0`.

## 0. Working Notes

- **Target question:** Do `FUN_00432690` and `FUN_004326C0` represent different BIK asset source modes, and exactly what reaches `_BinkOpen`?
- **Non-goals:** Do not rediscover resolver order, frame loop, vtable slots, audio-volume update, or movie UI paths except where proving constructor liveness/source mode needs them.
- **Evidence needed to mark COMPLETE:** live Ghidra decompile plus assembly context for `0x00432690`, `0x004326C0`, `0x00432750`; xrefs/caller proof; archive lookup helper proof; Rust scan for parser/source-mode implications.
- **Stop conditions:** stop at constructor/open-source boundary; defer Bink SDK/runtime-only behavior, corrupt-file visible result, and full MSAnim usage.

## 1. Overview

`0x00432690` and `0x004326C0` are two constructors for the same 0x34-byte native Bink object. They do not split loose-file versus archive-file loading. They differ mainly in the initial value of `BinkObject+0x0C`: `0x00432690` clears it for fullscreen/blocking movie playback, while `0x004326C0` stores a caller-supplied surface/context pointer before calling the same open helper.

The loose filename versus Win32 file-handle `_BinkOpen` mode is selected inside `0x00432750`, after both constructors have already initialized the object and forwarded the movie name. If a raw-file availability object accepts the name, `0x00432750` calls `_BinkOpen(name, 0)`. If not, it looks up the name in the global archive index, opens the containing file with `CreateFileA`, seeks to the returned member offset, stores that handle at object `+0x28`, and calls `_BinkOpen(handle, 0x800000)`.

## 2. Class Layout / Key Offsets

| Offset | Writer | Value / meaning | Active in YR | Evidence |
|---:|---|---|---|---|
| `+0x00` | both constructors | byte zeroed | Yes | decompile `0x00432690`, `0x004326C0`; assembly `0x00432695`, `0x004326D3` |
| `+0x04` | both constructors, then `0x00432750` | initialized null; later Bink SDK handle from `_BinkOpen` | Yes | assembly `0x00432697`, `0x004326D5`, write at `0x0043284F` |
| `+0x0C` | `0x00432690` | initialized `0`; open helper may select hidden/primary surface and center movie | Yes for fullscreen BIK | assembly `0x0043269A`; xref `0x005BEDEC` |
| `+0x0C` | `0x004326C0` | initialized from caller's second stack argument, before open | Yes for owner-draw; Conditional for MSBinkAnim | assembly `0x004326C9..0x004326CE`; xrefs `0x005C0878`, `0x005CC7F6` |
| `+0x20` | both constructors | BSurface/helper pointer initialized null; old value destroyed by `0x00432750` before new open | Yes | assembly `0x0043269D`, `0x004326D8`; cleanup `0x0043275B..0x00432772` |
| `+0x28` | both constructors, then archive branch | initialized `-1`; archive path stores `CreateFileA` handle | Yes | assembly `0x004326A8`, `0x004326DB`; open write `0x0043282D` |
| `+0x2C` | both constructors | byte set to `1` | Yes | assembly `0x004326AF`, `0x004326E2` |
| `+0x2D` | both constructors | byte set to `0` | Yes | assembly `0x004326A0`, `0x004326E6` |
| `+0x30` | `0x00432750` | cleared before every open attempt | Yes | assembly `0x0043279C` |

## 3. Core Logic

### 3.1 `FUN_00432690` - fullscreen/stack-object constructor

Active in YR: Yes, through the main-menu Movies/Sneak Preview fullscreen BIK path.

Live Ghidra decompile shows `0x00432690(this, name)` initializes a Bink object and calls `FUN_00432750(name)`. Assembly context is tighter:

- `0x00432691` moves `ECX` into `ESI`, proving thiscall object ownership.
- `0x00432693..0x004326A0` zeroes `+0x00`, `+0x04`, `+0x0C`, `+0x20`, and `+0x2D`.
- `0x004326A3..0x004326A7` loads the single stack argument and pushes it for `0x00432750`.
- `0x004326A8` writes `+0x28 = -1`.
- `0x004326AF` writes `+0x2C = 1`.
- `0x004326B3` calls `0x00432750`; `0x004326BB` returns with `RET 0x4`.

Caller evidence: the only live xref returned by Ghidra is `0x005BEDEC` in `FUN_005BED40`. Assembly at `0x005BEDE8..0x005BEDEC` does `LEA ECX,[ESP+0x20]` then `CALL 0x00432690`, so the object is stack/local for blocking fullscreen playback. `MOVIES_CREDITS_DIALOG_PLAYBACK_FUN_005BED40_GHIDRA_REPORT.md` proves this path is reachable from the standard YR Movies/Sneak Preview menu.

Important source-mode detail: `0x00432690` does not force filename mode. It only clears `+0x0C`. Its argument is still passed into the shared `0x00432750`, which can choose either `_BinkOpen(name, 0)` or `_BinkOpen(handle, 0x800000)`.

### 3.2 `FUN_004326C0` - caller-surface constructor

Active in YR: Yes for owner-draw BinkMovieHandle; Conditional for MSBinkAnim.

Live Ghidra decompile shows `0x004326C0(this, name, surface)` stores the third parameter at object `+0x0C` and calls the same open helper. Assembly context proves the argument order:

- `0x004326C0` reads the first stack argument into `EDX`.
- After `PUSH ESI`, `0x004326C9` reads the second original stack argument into `ECX`.
- `0x004326CD` pushes `EDX`, so the first argument is the name forwarded to `0x00432750`.
- `0x004326CE` writes the second argument to `[ESI+0x0C]`.
- `0x004326D1` restores `ECX=ESI` for the thiscall helper.
- `0x004326D3..0x004326E6` zeroes/initializes the same fields as `0x00432690`, except it does not clear `+0x0C`.
- `0x004326E9` calls `0x00432750`; `0x004326F1` returns with `RET 0x8`.

Caller evidence:

- `0x005C0878` in `VQMovieHandle__Constructor` calls `0x004326C0`. Assembly `0x005C0870..0x005C0878` pushes the resolved BIK name buffer, then the owner-draw surface/context pointer, and sets `ECX` to the newly allocated 0x34-byte object. This is the active main-menu RA2TS owner-draw path when the resolver selects `.BIK`.
- `0x005CC7F6` in `MSAnim__Constructor` calls `0x004326C0` after `FUN_007B5440` resolves a filename token. This is a conditional MSBinkAnim construction path; full stock WDT asset liveness is outside this slot.

Important source-mode detail: `0x004326C0` does not receive an archive handle. Its second stack argument is stored at `BinkObject+0x0C`, which is later used as the surface/target context by open/copy logic. The actual file-handle source mode is still selected later by `0x00432750`.

### 3.3 Shared `FUN_00432750` source selection

Active in YR: Yes for both constructor call chains.

The source decision inside `0x00432750` is:

1. Destroy prior object-owned resources: BSurface at `+0x20`, Bink handle at `+0x04`, Win32 file handle at `+0x28` if not `-1`.
2. Clear `+0x30`.
3. If sound is initialized, call `_BinkSetSoundSystem(BinkOpenDirectSound, DirectSound*)` before opening.
4. Construct a raw-file availability object from the forwarded name.
5. If availability returns exactly `1`, call `_BinkOpen(name, 0)`.
6. Otherwise call archive lookup helper `0x005B4430(name, ...)`.
7. If archive lookup fails, skip `_BinkOpen` and go to failure handling.
8. If archive lookup succeeds, open the containing archive/container file with `CreateFileA`, seek to the returned member offset, store the handle at `+0x28`, and call `_BinkOpen(handle, 0x800000)`.

Live assembly evidence:

- Raw-file probe: `0x004327BA..0x004327D9` constructs a RawFileClass-style object from the forwarded name, calls vtable `+0x14`, then destroys the temp file object.
- Filename mode: `0x004327DE..0x004327E4` tests the exact-success byte and pushes `0` plus the forwarded name before jumping to the `_BinkOpen` call at `0x00432849`.
- Archive lookup: `0x004327E6..0x0043280A` calls `0x005B4430` with the forwarded name and output locals; if the return byte is not `1`, execution jumps to failure handling at `0x00432852`.
- Archive file open: `0x00432810..0x00432824` pushes `CreateFileA` arguments including desired access `0x80000000`, share mode `3`, creation disposition `3`, and flags/attributes `0x8000080`.
- Handle storage and seek: `0x0043282A..0x0043283A` stores the handle at `+0x28`, rejects `-1`, and calls `SetFilePointer(handle, returned_offset, 0, FILE_BEGIN)`.
- Bink file-handle mode: `0x00432840..0x00432849` pushes `0x800000` and the stored handle before `_BinkOpen@8`.
- Failure path: `0x00432852..0x00432874` checks `+0x04`, calls `_BinkGetError`, logs `Bink Error: %s`, returns `0`, and does not run volume/surface setup.

Helper `0x005B4430` decompile proves the archive lookup returns structured source data rather than bytes. It hashes the uppercased filename, searches the global MIX/archive chain, and, on hit, writes the archive object and entry offset/size outputs. That output feeds the `CreateFileA`/`SetFilePointer` path above.

## 4. INI Keys

No INI key is read by `0x00432690`, `0x004326C0`, or `0x00432750`. Movie names and menu/campaign availability are upstream systems. This report only proves what happens once a BIK filename/token reaches the native Bink object constructor.

## 5. Integration Points

| Function/path | Relationship | Active in YR | Evidence |
|---|---|---|---|
| `FUN_005BED40` | Fullscreen/blocking movie path constructs a stack Bink object through `0x00432690`, plays with `0x00432C70`, then destroys with `0x00432700`. | Yes for Movies/Sneak Preview BIK | xref `0x005BEDEC`; `MOVIES_CREDITS_DIALOG_PLAYBACK_FUN_005BED40_GHIDRA_REPORT.md` |
| `VQMovieHandle__Constructor @ 0x005C07D0` | Owner-draw `.BIK` branch allocates 0x34-byte object and calls `0x004326C0(name, surface)`. | Yes for RA2TS owner-draw | xref `0x005C0878`; `BIK_VQA_FALLBACK_AND_UNSUPPORTED_CONTRACT_GHIDRA_REPORT.md` |
| `MSAnim__Constructor @ 0x005CC760` | Conditional MSBinkAnim path calls `0x004326C0(resolved_name, context)` and stores result at MSBinkAnim `+0x1C`. | Conditional | xref `0x005CC7F6`; `MSANIM_MSBINKANIM_CLASS_USAGE_SPLIT_GHIDRA_REPORT.md` |
| `FUN_00432750` | Shared open helper for both constructors. | Yes | xrefs from `0x004326B3`, `0x004326E9` |
| `FUN_005B4430` | Archive/MIX lookup helper used only after raw-file availability fails. | Yes when selected BIK is archive-backed | decompile `0x005B4430`; call `0x00432803` |

## 6. Current Rust Implementation Status

Rust currently loads RA2TS bytes through `AssetManager` and parses BIK content directly:

- `src/app_main_menu_shell_render.rs` resolves `ra2ts_s.bik` / `ra2ts_l.bik` with `get_with_source_ref` and passes bytes to `BinkMovieSurface::from_bytes`.
- `src/render/bink_movie.rs` owns `BinkMovieSurface`, parses `BinkFile`, decodes the first packet immediately, and uploads a GPU texture.
- `src/assets/bink_file.rs` parses the BIK header, audio track descriptors, and frame index directly, with Rust-side validation bounds.

Rust does not model native constructor distinction, Win32 file-handle `_BinkOpen`, current-file-position semantics, or the typed loose/archive source mode. For RA2TS visual playback, byte loading may be enough for decoded frames. For exact native source-boundary parity and diagnostics, Rust needs to preserve whether the selected asset came from loose/raw lookup, archive-backed lookup, VQA-unsupported fallback, or missing file.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00432690` field init and call signature | verified | decompile `0x00432690`; assembly `0x00432691..0x004326BB` | none |
| `0x004326C0` field init and argument order | verified | decompile `0x004326C0`; assembly `0x004326C0..0x004326F1` | none |
| `0x00432690` liveness | verified | xref `0x005BEDEC`; Movies/Credits report | none for Movies/Sneak Preview |
| `0x004326C0` owner-draw liveness | verified | xref `0x005C0878`; BIK/VQA fallback report | none for RA2TS owner-draw |
| `0x004326C0` MSAnim liveness | touched-not-exhausted | xref `0x005CC7F6`; MSAnim report | full stock asset/user liveness deferred |
| Direct filename `_BinkOpen(name,0)` | verified | assembly `0x004327DE..0x00432849` | none |
| Archive-backed `_BinkOpen(handle,0x800000)` | verified | assembly `0x004327E6..0x00432849`; decompile `0x005B4430` | exact struct field names for archive object deferred |
| Corrupt BIK after successful source selection | deferred | failure branch `0x00432852..0x00432874` | visible/runtime behavior needs debugger/fixture |
| Rust parser/source-mode scan | verified | `rg` and focused reads of `src/assets/bink_file.rs`, `src/render/bink_movie.rs`, `src/app_main_menu_shell_render.rs` | implementation not performed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does either constructor parse raw BIK bytes? -> No; both initialize object fields and call shared `0x00432750`.` (evidence: `0x00432690`, `0x004326C0`)
- `[RESOLVED] OQ-02 - What does `0x00432690` set at `+0x0C`? -> It writes zero, leaving target surface selection to `0x00432750`.` (evidence: assembly `0x0043269A`)
- `[RESOLVED] OQ-03 - What does `0x004326C0` set at `+0x0C`? -> It copies the caller's second argument before open.` (evidence: assembly `0x004326C9..0x004326CE`)
- `[RESOLVED] OQ-04 - Is `0x00432690` live in standard YR? -> Yes for Movies/Sneak Preview BIK playback through `FUN_005BED40`.` (evidence: xref `0x005BEDEC`; Movies/Credits report)
- `[RESOLVED] OQ-05 - Is `0x004326C0` live in standard YR? -> Yes for owner-draw RA2TS BIK through `VQMovieHandle__Constructor`.` (evidence: xref `0x005C0878`; BIK/VQA fallback report)
- `[RESOLVED] OQ-06 - Does `0x004326C0` receive a file/archive handle? -> No; its second argument is stored at object `+0x0C`, not `+0x28`, and the file handle is created later inside `0x00432750`.` (evidence: `0x004326CE`, `0x0043282D`)
- `[RESOLVED] OQ-07 - Which function selects filename vs file-handle `_BinkOpen`? -> `0x00432750`, not either constructor.` (evidence: `0x004327DE..0x00432849`)
- `[RESOLVED] OQ-08 - What is passed to `_BinkOpen` in direct mode? -> Forwarded name pointer plus flags `0`.` (evidence: `0x004327E2..0x004327E4`, `_BinkOpen` at `0x00432849`)
- `[RESOLVED] OQ-09 - What is passed to `_BinkOpen` in archive mode? -> Win32 handle stored at object `+0x28` plus flags `0x800000`, after `SetFilePointer` to returned member offset.` (evidence: `0x0043282D..0x00432849`)
- `[RESOLVED] OQ-10 - Is `0x800000` an audio flag? -> No; it is selected only on the handle-source path.` (evidence: `0x00432840..0x00432849`; audio report)
- `[RESOLVED] OQ-11 - Does archive lookup return decoded bytes? -> No; helper returns archive/container metadata used for `CreateFileA`/`SetFilePointer`.` (evidence: decompile `0x005B4430`; use at `0x00432810..0x0043283A`)
- `[DEFERRED] OQ-12 - What exact player-visible result follows corrupt BIK after `_BinkOpen` fails?` (category: `needs-runtime-debugger`; reason: static failure branch is proven, but caller-visible loop/update behavior with a null inner object needs controlled runtime proof; next-step-if-pursued: corrupt selected BIK and break on `0x00432750`, `0x00432C70`, and destructor)
- `[DEFERRED] OQ-13 - Which stock WDT assets exercise MSBinkAnim in normal YR?` (category: `requires-different-system-context`; reason: source-mode proof only needed constructor xref; full WDT asset liveness belongs to MSAnim/WDT investigation; next-step-if-pursued: trace WDT loader and stock UIMD/WDT assets)

## 9. Visual/UI Composition Ledger

This report is not a full visual composition report. The visual-surface fact in scope is constructor/open ownership:

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `0x00432690` | fullscreen BIK branch in `FUN_005BED40` | selected Movies/Sneak Preview BIK | `+0x0C` starts null; open helper selects/centers target | Bink DDS path later | Yes for `.BIK` Movies/Sneak Preview | blocking fullscreen movie object |
| 2 | `0x004326C0` | owner-draw `.BIK` branch in `0x005C07D0` | RA2TS BIK for main menu | caller surface/context stored at `+0x0C` before open | Bink DDS path later | Yes for RA2TS | owner-draw movie object |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Constructor choice is not loose-vs-archive. `0x00432690` clears `+0x0C`; `0x004326C0` stores caller surface/context at `+0x0C`; both forward the name to `0x00432750`. | decompile `0x00432690`, `0x004326C0`; assembly `0x0043269A`, `0x004326CE`; xrefs `0x005BEDEC`, `0x005C0878` | Rust has one byte-oriented `BinkMovieSurface` and no native constructor/source context model. | `src/render/bink_movie.rs`; future movie subsystem above parser | Model playback context separately from asset source: fullscreen/blocking vs owner-draw/MSAnim is not the same axis as loose/archive. | Fullscreen Movies BIK and owner-draw RA2TS BIK can use the same parsed byte content but different playback/surface mode. Proposed test: `bink_constructor_context_does_not_change_asset_source_mode`. | Do not implement `0x004326C0` as "archive-handle constructor" or `0x00432690` as "loose-file constructor". |
| `_BinkOpen` source mode is selected in `0x00432750`: raw availability success calls `_BinkOpen(name,0)`, archive lookup success calls `CreateFileA`, `SetFilePointer`, then `_BinkOpen(handle,0x800000)`. | assembly `0x004327BA..0x00432849`; decompile `0x005B4430` | Rust loses source-mode metadata after `AssetManager::get_with_source_ref`, and parser sees only bytes. | asset manager/movie resolver layer; `src/app_main_menu_shell_render.rs`; `src/assets/bink_file.rs` diagnostics | Preserve a typed selected source (`LooseName`, `ArchiveMember`, `VqaUnsupported`, `Missing`) above the BIK parser, while keeping the parser byte-based. | A BIK present only in MIX/archive resolves to an archive-member source and still parses the same bytes. Proposed test: `movie_bik_archive_member_source_uses_byte_parser_without_audio_flag`. | Do not put archive lookup, VQA fallback, or `0x800000` semantics inside `BinkFile::parse`. |
| Archive mode opens the containing archive/container file and seeks to the member offset before passing the Win32 handle to Bink; it does not copy member bytes in gamemd. | `0x005B4430` output writes; `0x00432810..0x00432849` `CreateFileA`/`SetFilePointer`/`_BinkOpen` | Rust currently materializes bytes from archive manager; this can be equivalent for decode but not for exact native diagnostics/source boundary. | asset loading diagnostics, future runtime-oracle tests, movie resolver tests | Record source provenance and offset/length where available so test/oracle tooling can compare native file-handle behavior against byte-slice decoding. | Source provenance for `ra2ts_l.bik` reports `language.mix` priority while parser decodes from extracted bytes. Proposed test: `bik_archive_source_records_container_and_member_offset`. | Do not infer that gamemd validates the BIK header before `_BinkOpen`; BINKW32 owns container parsing/validation. |

### Negative Facts / Do Not Do

- Do not treat `0x00432690` as the loose-file constructor and `0x004326C0` as the archive-handle constructor. Both call `0x00432750`; source selection happens there. Evidence: xrefs to `0x00432750` at `0x004326B3` and `0x004326E9`.
- Do not treat the second argument to `0x004326C0` as a file handle. It is copied to `BinkObject+0x0C`; the actual file handle is stored later at `+0x28`. Evidence: `0x004326CE`, `0x0043282D`.
- Do not map `_BinkOpen` flag `0x800000` to embedded audio. It is pushed only after loading the Win32 handle from `+0x28`. Evidence: `0x00432840..0x00432849`.
- Do not put generic movie resolver or VQA fallback behavior inside the BIK parser. The parser should consume bytes; resolver/source-mode lives above it. Evidence: constructors/open helper operate on names/handles before BINKW32 parsing.
- Do not claim gamemd copies archive-member bytes into a memory buffer for Bink. Static evidence shows `CreateFileA`, `SetFilePointer`, and `_BinkOpen(handle,0x800000)`. Evidence: `0x00432810..0x00432849`.

### Remaining Uncertainty

- Exact visible behavior after `_BinkOpen` failure for a resolved but corrupt BIK still needs runtime debugger proof.
- Exact stock WDT/MSBinkAnim asset liveness remains outside this constructor/source-mode slice.
- The exact BINKW32 semantic name for flag `0x800000` is inferred from call shape and prior SDK naming; gamemd evidence proves only handle-source behavior.

### Stale Docs / Follow-up Docs

- `docs/research/FUN_00432750_BINK_OPEN_INIT_GHIDRA_REPORT.md`: replace "0x004326C0 constructor | Initializes Bink object defaults, stores caller surface at +0x0C" with: "`0x004326C0` is the caller-surface/context constructor, not an archive-handle constructor; it stores its second stack argument at `BinkObject+0x0C`, forwards the first argument to `0x00432750`, and the shared helper later selects filename versus Win32-handle `_BinkOpen`."
- `docs/research/FUN_00432750_BINK_OPEN_INIT_GHIDRA_REPORT.md`: replace "archive lookup details owned by slot 2" with: "Archive source mode is selected inside `0x00432750`: after raw-file availability fails, `0x005B4430` returns archive/container metadata, `CreateFileA` opens the containing file, `SetFilePointer` seeks to the member offset, and `_BinkOpen(handle,0x800000)` is called."
- Any wording that implies Rust must choose a different BIK parser for `0x00432690` vs `0x004326C0` should be replaced with: "Constructor context controls playback/surface ownership; byte parsing/source provenance is selected upstream/shared by `0x00432750`."

## Sources

- Live Ghidra MCP decompile: `0x00432690`, `0x004326C0`, `0x00432750`, `0x005B4430`, `0x005BED40`, `0x005C07D0`, `0x005CC760`.
- Live Ghidra MCP xrefs: `0x00432690`, `0x004326C0`, `0x00432750`.
- Live Ghidra MCP assembly context: `0x00432690..0x004326F1`, `0x004327BA..0x00432849`, caller sites `0x005BEDEC`, `0x005C0878`, `0x005CC7F6`.
- Prior reports: `MOVIES_CREDITS_DIALOG_PLAYBACK_FUN_005BED40_GHIDRA_REPORT.md`, `BIK_VQA_FALLBACK_AND_UNSUPPORTED_CONTRACT_GHIDRA_REPORT.md`, `MSANIM_MSBINKANIM_CLASS_USAGE_SPLIT_GHIDRA_REPORT.md`, `AUDIO_BEARING_BIK_PATH_AND_VOLUME_GHIDRA_REPORT.md`, `FUN_00432750_BINK_OPEN_INIT_GHIDRA_REPORT.md`.
- Rust files scanned read-only: `src/assets/bink_file.rs`, `src/render/bink_movie.rs`, `src/app_main_menu_shell_render.rs`, `src/ui/main_menu_shell/layout.rs`.
