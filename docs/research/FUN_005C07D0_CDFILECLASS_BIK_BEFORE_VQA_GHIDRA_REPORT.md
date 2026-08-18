# FUN_005C07D0 / CDFileClass BIK-before-VQA Path - Ghidra Research Report

**Address(es):** `0x005C0640`, `0x005C07D0`, live owner path `0x00531CC0`, `0x006153E0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** movie base-name resolution at the generic VQ-named wrapper layer: extension stripping, `.BIK` before `.VQA`, availability lookup, Bink-vs-VQA handle selection, and standard YR main-menu path into that wrapper.
**Non-Scope:** Bink header parsing, per-frame Bink cadence, pixel copy format, audio tracks, loop/end semantics beyond wrapper ownership, full MIX registration ordering beyond the already settled RA2TS `LANGUAGE.MIX` priority.
**Confidence:** High for extension order, branch selection, failure return, and standard YR main-menu liveness; Medium for naming the lower availability class semantics because this pass had no live Ghidra MCP and relies on prior Ghidra reports plus raw retail-byte disassembly.
**Active in YR:** Yes for the standard initial shell main menu `0xE2` / child static `0x71A`; Conditional for other movie callers that pass movie base names into the same wrapper.

## 0. Working Notes

- Target question: Does the `FUN_005C07D0` wrapper resolve a requested movie base name by trying `.BIK` before `.VQA`, what file lookup/check does it use, and what exact handle branch does Rust need to model?
- Non-goals: Do not re-investigate RA2TS asset dimensions, audio absence, Bink decode cadence, Bink copy format, or global archive priority already covered by adjacent slots.
- Evidence needed to mark COMPLETE: raw address evidence for `.BIK`/`.VQA` order, raw or decompile-backed evidence for handle branch/failure behavior, caller evidence proving standard YR main-menu activation, current Rust surface scan, and at least one implementation handoff.
- Stop conditions: stop at wrapper-layer source selection; record any lower CCFile/MIX internals not needed for extension order as deferred rather than expanding scope.

## 1. Overview

The generic movie constructor named around `VQMovieHandle` is not VQA-only. For a caller-provided movie token such as `Ra2ts_l`, helper `0x005C0640` strips any existing extension, tries `base + ".BIK"` first through a file-availability object, then tries `base + ".VQA"` only if the BIK candidate is unavailable. `FUN_005C07D0` then checks the resolved extension: `.bik` installs a Bink-backed handle and vtable `0x007EE154`; anything else falls to the legacy VQA handle and vtable `0x007EE0F4`.

Active in YR: Yes. `FUN_00531CC0` sends `0x4E3` and `0x4E4` to child static `0x71A` with `"Ra2ts_s"` at exactly 640 width and `"Ra2ts_l"` otherwise; owner-draw static code calls `FUN_005C07D0` from the `0x4E4` path and arms timer `0x65`.

## 2. Class Layout / Key Offsets

| Object / offset | Purpose | Evidence | Active in YR |
|---|---|---|---|
| generic movie handle `+0x00` | vtable pointer: Bink `0x007EE154` or VQA `0x007EE0F4` | raw disassembly `0x005C0897`, `0x005C0924` | Yes |
| generic movie handle `+0x08` | movie width copied from concrete object | raw disassembly `0x005C08A6..0x005C08B4` for Bink, `0x005C092D..0x005C0941` for VQA | Yes |
| generic movie handle `+0x0C` | movie height copied from concrete object | same ranges above | Yes |
| generic movie handle `+0x10` | concrete Bink object or VQA object pointer | raw disassembly `0x005C089D`, `0x005C092A` | Yes |
| generic movie handle `+0x14` | VQA path byte initialized to `0` | raw disassembly `0x005C0920` | Conditional; only VQA fallback |

## 3. Core Logic

### 3.1 Extension resolver `0x005C0640`

Verified behavior:

1. Copies the caller token into a stack buffer, then walks from the first byte until NUL or `'.'`. The byte at the first dot or NUL is overwritten with `0`, so any caller extension is stripped before candidates are appended.
2. Copies string `".BIK"` from `0x0082419C` onto that base and constructs a file object using `0x004739F0`.
3. Calls the object's vtable slot `+0x14` with argument `0`; the result byte controls candidate success.
4. Only if the BIK availability result is zero, copies string `".VQA"` from `0x008241A4` onto the same base and repeats the availability call.
5. If either candidate succeeds, optionally copies the resolved filename to the output pointer and returns `AL = 1`; if both fail, returns `AL = 0`.

Evidence: raw retail `gamemd.exe` disassembly `0x005C067B..0x005C0699` for extension stripping; `0x005C06A8..0x005C0712` for `.BIK` append/open/check; `0x005C0714..0x005C077E` for `.VQA` fallback; `0x005C0780..0x005C07B8` for success copy/return; `0x005C07BB..0x005C07C0` for failure return. String bytes read from the PE: `0x0082419C = ".BIK"`, `0x008241A4 = ".VQA"`.

Active in YR: Yes. Direct call xrefs in retail bytes: `0x005BED54` and `0x005C07EA`; the standard main-menu path reaches `0x005C07EA` through `FUN_005C07D0`.

### 3.2 Generic constructor `0x005C07D0`

Verified behavior:

1. Null movie token / failed resolver returns `0` before any handle allocation.
2. After resolver success, it locates the final dot with `0x007C8DF0` and compares that suffix against lowercase `".bik"` at `0x0082D9CC` using compare helper `0x007C8D20`.
3. If compare succeeds, it logs/registers string `0x0082D9B4` (`Play_Movie() as Bink!\n`), allocates `0x34` bytes for the concrete Bink object, calls `0x004326C0`, then allocates a `0x14`-byte generic wrapper with Bink vtable `0x007EE154`.
4. If the extension is absent or not `.bik`, it creates the legacy VQA object through `0x005BFAA0`, then allocates a `0x18`-byte generic wrapper with VQA vtable `0x007EE0F4`.
5. If Bink concrete allocation/open fails but the generic Bink wrapper allocation succeeds, the wrapper can be returned with `+0x10 = 0` and no width/height copy; this is the corrupt/open-failure edge already marked uncertain in prior docs. If the generic wrapper allocation itself fails, the function returns `0`.

Evidence: raw retail disassembly `0x005C07D0..0x005C07F1` for resolver call/failure; `0x005C082B..0x005C0856` for dot search and `.bik` compare; `0x005C0858..0x005C08B7` for Bink allocation/vtable/size copy; `0x005C08C4..0x005C0944` for VQA allocation/vtable/size copy; prior `MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md` section 3 decompile-backed summary for the same branch.

Active in YR: Yes for standard main-menu RA2TS playback; Conditional for movie/credits callers using the same wrapper.

### 3.3 File lookup / handle selection at this layer

The resolver's availability check is a temporary `CCFileClass`-style file object, not the final movie object. Prior Ghidra work identifies `0x004739F0` as `CCFileClass__Constructor`; its vtable `+0x14` availability/open check can accept raw/search-path/MIX-backed availability, and the object is immediately cleaned up after each candidate check. The winning candidate name, not a pinned low-level file handle from the temporary object, is copied to the caller's resolved-name buffer.

Evidence: raw resolver calls `0x004739F0` then vtable `+0x14` at `0x005C06D2..0x005C06E1` and `0x005C073E..0x005C074D`, followed by cleanup at `0x005C06E4..0x005C070B` and `0x005C0750..0x005C0777`; prior `SKIRMISH_SELECTED_MAP_FILE_OPEN_PRIORITY_LOOSE_SHADOWING_GHIDRA_REPORT.md` sections 3.4/3.5 decompile-backed `CCFileClass` raw-before-MIX behavior for the same constructor/open family; prior `ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md` section 2.6 identifies `0x004739F0`.

Active in YR: Yes as a file-availability mechanism in the live main-menu wrapper path. Exact source priority inside the loaded MIX list is separate; for RA2TS duplicates it is already settled that `LANGUAGE.MIX` wins over `LANGMD.MIX`.

## 4. INI Keys

No INI keys are read by this wrapper slice. The standard main-menu asset token is hardcoded by dialog/main-menu code, not by `rules*.ini` or `art*.ini`.

## 5. Integration Points

| Integration point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| `FUN_00531CC0` main-menu creation | Sends `0x4E3` loop flag and `0x4E4` with `"Ra2ts_s"` only when screen width is `0x280` (640), else `"Ra2ts_l"` | raw disassembly `0x00531D84..0x00531DB0`; string bytes `0x00825CE8 = "Ra2ts_s"`, `0x00825CE0 = "Ra2ts_l"` | Yes |
| `OwnerDraw_Static_006153E0` load movie path | Destroys old movie, calls `FUN_005C07D0`, stores returned handle, sizes child window from handle `+8/+0xC`, sets timer `0x65` period `0x22` | raw disassembly `0x006160E8..0x0061615E`; prior `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` 0x4E4 detail | Yes |
| Owner-draw refresh/load variant | A second nearby path also calls `FUN_005C07D0` and then uses the same vtable size/timer flow | raw disassembly `0x00616199..0x0061620C`; direct xref at `0x006161E1` | Yes/Conditional on owner-draw message variant |
| Other movie users | Direct calls to `FUN_005C07D0` also occur at `0x005BF189`, `0x005BF2C9`, `0x005BF3C5` | raw direct-call scan | Conditional; not expanded in this slot |
| Movies/credits helper | Calls resolver `0x005C0640`, so the same `.BIK` before `.VQA` name resolution applies there too | raw direct-call scan `0x005BED54`; prior `MOVIES_AND_CREDITS_DIALOG_CASE4_GHIDRA_REPORT.md` section 6 | Conditional by movie/credits mode |

## 6. Current Rust Implementation Status

Current Rust main-menu code already hardcodes physical BIK names for the RA2TS panel:

| Rust surface | Current behavior | Delta against wrapper |
|---|---|---|
| `src/ui/main_menu_shell/layout.rs` `MainMenuMovieBase::asset_name` | returns `"ra2ts_s.bik"` / `"ra2ts_l.bik"` | Acceptable for standard RA2TS happy path; does not expose the generic base-name `.BIK` then `.VQA` resolver |
| `src/app_main_menu_shell_render.rs` `ensure_movie_for_current_layout` | asks `AssetManager::get_with_source_ref(asset_name)` for the physical `.bik` name and treats miss/decode error as shell failure | Matches retail BIK-first outcome when BIK exists; mismatch for a base-name API or fallback scenario where `.BIK` is absent but `.VQA` exists |
| `src/assets/asset_manager.rs` | name-based lookup, earlier archive wins through `or_insert`; no wrapper-level extension fallback | Good low-level primitive, but the movie resolver behavior belongs above it |
| `src/render/bink_movie.rs` and BIK tools | Bink-only consumers | Not responsible for VQA fallback; should not absorb generic movie resolution unless deliberately creating a movie resolver facade |

Codegraph context confirmed the primary symbols: `MainMenuMovieBase`, `asset_name`, `AssetManager`, `movie_base_for_screen_width`, and `AssetManager::get`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x005C0640` extension stripping | verified | raw disassembly `0x005C067B..0x005C0699` | none |
| `0x005C0640` `.BIK` candidate | verified | raw disassembly `0x005C06A8..0x005C0712`; string `0x0082419C` | none |
| `0x005C0640` `.VQA` fallback | verified | raw disassembly `0x005C0714..0x005C077E`; string `0x008241A4` | none |
| `0x005C0640` success/failure return | verified | raw disassembly `0x005C0780..0x005C07C0` | none |
| `0x005C07D0` Bink branch | verified | raw disassembly `0x005C082B..0x005C08B7`; prior Ghidra report | corrupt Bink open runtime effect deferred |
| `0x005C07D0` VQA branch | verified | raw disassembly `0x005C08C4..0x005C0944` | VQA decoder internals out of scope |
| standard main-menu liveness | verified | raw disassembly `0x00531D84..0x00531DB0`, `0x006160E8..0x0061615E`; prior owner-draw report | none |
| lower CCFile raw/MIX priority | touched-not-exhausted | prior Ghidra reports `0x004739F0`, `0x00473D10`; raw call sites in resolver | full archive-stack order outside RA2TS not re-investigated |
| Rust generic movie resolver | touched-not-exhausted | Codegraph and file scan surfaces listed in section 6 | no Rust edits in this research slot |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is the target live in standard YR main-menu? -> Yes, `FUN_00531CC0` sends `0x4E4` with `Ra2ts_s/l` to child `0x71A`, and owner-draw static calls `FUN_005C07D0`.` (evidence: `0x00531D84..0x00531DB0`, `0x0061611C`)
- `[RESOLVED] OQ-02 - Is `.BIK` tried before `.VQA`? -> Yes, `.BIK` append/check precedes the conditional `.VQA` fallback.` (evidence: `0x005C06A8..0x005C077E`)
- `[RESOLVED] OQ-03 - Does caller extension survive? -> No, the resolver truncates at the first dot or NUL before appending candidates.` (evidence: `0x005C067B..0x005C0699`)
- `[RESOLVED] OQ-04 - What happens if both candidates fail? -> Resolver returns `AL=0`; `FUN_005C07D0` returns null; owner-draw kills timer/clears movie state.` (evidence: `0x005C07BB..0x005C07C0`, `0x005C07EF..0x005C07F1`, `0x00616170..0x00616186`)
- `[RESOLVED] OQ-05 - Is extension comparison case-sensitive? -> The compare helper is the same case-insensitive family cited by prior reports; raw branch compares suffix pointer to `".bik"` via `0x007C8D20`.` (evidence: `0x005C0846..0x005C0856`; `MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-06 - Does BIK branch install a different vtable? -> Yes, wrapper vtable is `0x007EE154` and concrete pointer is stored at `+0x10`.` (evidence: `0x005C0897..0x005C089D`)
- `[RESOLVED] OQ-07 - Does VQA fallback still exist? -> Yes, non-BIK path calls `0x005BFAA0` and installs vtable `0x007EE0F4`.` (evidence: `0x005C08C4..0x005C092A`)
- `[RESOLVED] OQ-08 - Does the temporary availability check pin the final file handle? -> No evidence of pinning here; the resolver copies the winning name and destroys temp file objects before final movie object creation.` (evidence: `0x005C06E4..0x005C070B`, `0x005C0750..0x005C0777`, `0x005C0780..0x005C07AC`)
- `[RESOLVED] OQ-09 - Are INI keys involved? -> No INI read in this wrapper slice.` (evidence: raw call path and prior main-menu reports)
- `[RESOLVED] OQ-10 - Current Rust surface? -> main-menu Rust requests physical `.bik` names directly and lacks a generic `.BIK` before `.VQA` movie resolver.` (evidence: `src/ui/main_menu_shell/layout.rs`, `src/app_main_menu_shell_render.rs`, Codegraph context)
- `[DEFERRED] OQ-11 - Exact corrupt Bink file runtime outcome after BIK name resolves but `_BinkOpen` fails.` (category: `needs-runtime-debugger`; reason: prior docs mark wrapper state/crash-vs-blank outcome as medium-confidence; next-step-if-pursued: runtime trace with intentionally corrupt BIK and breakpoints on `0x00432750`/vtable update slots)
- `[DEFERRED] OQ-12 - Full global archive-stack priority for all movie files.` (category: `requires-different-system-context`; reason: RA2TS `LANGUAGE.MIX` priority is settled, but this wrapper only proves filename candidate order; next-step-if-pursued: dedicated MIX registration/order investigation)
- `[DEFERRED] OQ-13 - VQA decoder and playback semantics after fallback.` (category: `out-of-scope`; reason: this slot stops at wrapper selection; next-step-if-pursued: investigate `0x005BFAA0` and VQA vtable)

Adversarial corner checks: base with extension, missing BIK but present VQA, both missing, corrupt BIK, and duplicate BIK archives are all either resolved above or explicitly deferred.

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `FUN_00531CC0` | dialog `0xE2` creation; sends `0x4E3=1` then `0x4E4` | base token `Ra2ts_s` at width 640, else `Ra2ts_l` | child `0x71A` repositioned around shell origin | none here | yes | movie setup |
| 2 | `OwnerDraw_Static_006153E0` `0x4E4` path | receives load message | resolved `*.BIK` before `*.VQA` | `MoveWindow` uses movie handle `+8/+0xC` | Bink/VQA object handles later decode | yes | child movie object install |
| 3 | `OwnerDraw_Static_006153E0` timer path | timer `0x65`, period `0x22` after handle success | same movie | offscreen decoded frame | concrete Bink/VQA path | yes | decode/update |
| 4 | `OwnerDraw_Static_006153E0` `0x4F0` path | dialog proc sends on `WM_PAINT` | current decoded movie frame | copied/drawn via vtable slot | concrete Bink/VQA path | yes | visible movie blit |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `ra2ts_s.bik` | yes at width 640 if available | yes | yes | content | no | no | no | no | `0x00531D90..0x00531DA8`, resolver `0x005C0640` |
| `ra2ts_l.bik` | yes at non-640 width if available | yes | yes | content | no | no | no | no | `0x00531D90..0x00531DA8`, resolver `0x005C0640` |
| `ra2ts_s.vqa` / `ra2ts_l.vqa` | only if BIK candidate unavailable and VQA exists | conditional | conditional | content fallback | no | no | no | inactive in stock RA2TS BIK case | resolver fallback `0x005C0714..0x005C077E` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Movie base-name resolver strips any extension, tries `base + ".BIK"` first, then `base + ".VQA"` only if BIK availability fails | raw `0x005C067B..0x005C077E`; strings `0x0082419C`, `0x008241A4` | missing as a generic resolver; RA2TS happy path hardcodes `.bik` | `src/app_main_menu_shell_render.rs`, possible asset/movie resolver helper above `AssetManager` | expose a wrapper-level movie resolver for base tokens instead of relying on callers to know physical extension | fixture has both `foo.bik` and `foo.vqa`; resolving base `foo` chooses `foo.bik`; proposed test `movie_base_resolver_prefers_bik_over_vqa` | Do not ask `AssetManager` for `.vqa` first or preserve a caller-supplied `.vqa` extension as authoritative |
| If BIK is absent but VQA exists, the wrapper selects the VQA branch instead of treating the base name as missing | raw `0x005C0710..0x005C077E`, `0x005C08C4..0x005C0944` | unsupported unless future VQA playback is modeled; current RA2TS path can fail because it asks only for `.bik` | future generic movie playback facade; `src/render/bink_movie.rs` should remain Bink-specific | return a typed "VQA selected/unsupported" result or route to VQA playback when implemented, preserving BIK-first order | fixture has only `foo.vqa`; base resolver returns VQA candidate and does not report missing; proposed test `movie_base_resolver_falls_back_to_vqa_when_bik_missing` | Do not hide VQA fallback by making all movie tokens BIK-only |
| Standard YR main menu reaches this wrapper with extensionless `Ra2ts_s/l` tokens, but Rust currently requests physical `.bik` files directly | raw `0x00531D84..0x00531DB0`, `0x0061611C`; Codegraph/Rust scan | partial: output matches stock RA2TS BIK case, but not exact wrapper mechanism | `src/ui/main_menu_shell/layout.rs::MainMenuMovieBase::asset_name`, `src/app_main_menu_shell_render.rs::ensure_movie_for_current_layout` | either rename the Rust method to physical BIK asset for the current shortcut or implement base-token resolution before loading | remove/rename ambiguity test: `main_menu_movie_base_uses_native_bik_before_vqa_resolution` verifies 640/non-640 base token then `.bik` candidate choice | Do not document the native caller as sending `ra2ts_l.bik`; the native caller sends `Ra2ts_l` and the wrapper appends `.BIK` |

### Negative Facts / Do Not Do

- Do not implement `VQMovieHandle` as VQA-only. Active in YR: Yes; `.bik` suffix installs Bink vtable `0x007EE154` at `0x005C0897`.
- Do not try `.VQA` before `.BIK`. Active in YR: Yes; `.BIK` check occurs at `0x005C06A8..0x005C0712`, and `.VQA` is reached only after `test bl, bl` fails at `0x005C0710..0x005C0714`.
- Do not preserve a caller-provided extension when resolving through `0x005C0640`. Active in YR: Yes; the first dot is overwritten with NUL at `0x005C0686..0x005C0699`.
- Do not treat the resolver's temporary file object as the final playback handle. Active in YR: Yes; temp objects are cleaned before success return, and `0x005C07D0` constructs a new Bink/VQA object from the resolved name.
- Do not add a primitive still-image fallback for missing RA2TS movie at this wrapper. Active in YR: Yes; both-candidate failure returns null and owner-draw clears/kills timer rather than drawing fallback art (`0x005C07BB..0x005C07C0`, `0x00616170..0x00616186`).

### Stale Docs / Follow-up Docs

No prior report was found to be wrong on the core order. The older plan wording is too broad because it says only "`CDFileClass__Constructor @ 0x005C0640` `.BIK` before `.VQA`." Suggested replacement for `docs/plans/2026-05-17-initial-main-menu-dialog-0xe2-plan.md`:

> Movie base-name resolution is handled by helper `0x005C0640`: it strips any caller extension, checks availability for `base + ".BIK"` first through a temporary `CCFileClass`-style object, and only if that fails checks `base + ".VQA"`. `FUN_005C07D0` then branches by the resolved suffix: `.bik` creates a Bink-backed generic movie handle (`vtable 0x007EE154`), otherwise it creates the legacy VQA handle (`vtable 0x007EE0F4`).

## Sources

- Raw local retail binary disassembly: `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`, image base `0x00400000`; Capstone VA-to-file pass for `0x005C0640`, `0x005C07D0`, `0x00531CC0`, `0x006153E0`.
- Prior Ghidra-backed docs: `docs/research/MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md`, `docs/research/OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md`, `docs/research/BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`, `docs/research/MOVIES_AND_CREDITS_DIALOG_CASE4_GHIDRA_REPORT.md`.
- Prior file-layer docs: `docs/research/bridges/01-assets-map-load-overlay/ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_SELECTED_MAP_FILE_OPEN_PRIORITY_LOOSE_SHADOWING_GHIDRA_REPORT.md`.
- Rust scan: `src/ui/main_menu_shell/layout.rs`, `src/app_main_menu_shell_render.rs`, `src/assets/asset_manager.rs`, `src/render/bink_movie.rs`.
