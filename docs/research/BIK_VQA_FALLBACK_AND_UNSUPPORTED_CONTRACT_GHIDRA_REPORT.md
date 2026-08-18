# BIK/VQA Fallback And Unsupported Contract - Ghidra Research Report

**Address(es):** `0x005C0640`, `0x005C07D0`, immediate callers `0x005BED40`, `0x005BF390`, `0x006153E0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** extension stripping, `.BIK` before `.VQA` availability checks, wrapper/vtable selection for BIK versus VQA, null behavior when neither candidate exists, and the Rust-facing unsupported-VQA contract.
**Non-Scope:** Bink frame stepping, Bink pixel copy, Bink audio decode, VQA decoder internals, full campaign movie trigger semantics, and global MIX registration order.
**Confidence:** High for resolver order, branch/vtable selection, null behavior, and immediate caller xrefs; Medium for lower `CCFileClass` archive priority because this slice only proves the wrapper's candidate order.
**Active in YR:** Yes for the generic movie wrapper and main-menu owner-draw path; Conditional for campaign/movie-credit users depending on caller mode and content.

## 0. Working Notes

- Target question: Does active `gamemd.exe` resolve movie tokens by stripping extensions, trying BIK before VQA, selecting different wrapper classes/vtables, and returning null when neither exists; what must Rust do while VQA playback is intentionally unsupported?
- Non-goals: Do not re-investigate Bink frame stepping, Bink surface copy, Bink audio, VQA decode/playback internals, broad movie dialog composition, or global MIX priority.
- Evidence needed to mark COMPLETE: live MCP decompile plus assembly context for `0x005C0640`/`0x005C07D0`, live xrefs for immediate callers, string/vtable bytes for `.BIK`/`.VQA` and class selection, Rust surface scan, and implementation handoff with test-name proposals.
- Stop conditions: stop once wrapper-level filename candidate order, class/vtable branch, caller null behavior, and unsupported-VQA Rust contract are proven; defer decoder/runtime playback details.

## 1. Overview

The movie wrapper is not VQA-only. Helper `0x005C0640` copies the caller token, truncates it at the first dot, appends uppercase `.BIK`, checks availability, and only if that fails appends uppercase `.VQA` and checks again. `VQMovieHandle__Constructor` at `0x005C07D0` then branches on the resolved suffix: `.bik` creates a Bink-backed generic handle with vtable `0x007EE154`; non-BIK resolved names create the legacy VQA-backed handle with vtable `0x007EE0F4`.

Active in YR: Yes. Live MCP xrefs show `0x005C07D0` is called by owner-draw static movie paths and by `FUN_005BF390`; `0x005C0640` is also called by the full-screen movie/credits helper `0x005BED40`.

## 2. Class Layout / Key Offsets

| Object / offset | Purpose | Evidence | Active in YR |
|---|---|---|---|
| generic movie handle `+0x00` | vtable pointer: Bink `0x007EE154` or VQA `0x007EE0F4` | writes at `0x005C0897`, `0x005C0924`; memory reads of both vtables | Yes |
| generic movie handle `+0x08` | width copied from concrete object | Bink copy `0x005C08A6..0x005C08B4`; VQA copy `0x005C092D..0x005C0941` | Yes when construction succeeds |
| generic movie handle `+0x0C` | height copied from concrete object | same ranges as width | Yes when construction succeeds |
| generic movie handle `+0x10` | concrete Bink object pointer or VQA object pointer | Bink write `0x005C089D`; VQA write `0x005C092A` | Yes |
| generic VQA handle `+0x14` | byte initialized to `0` on VQA branch | `0x005C0920` | Conditional: VQA fallback only |

## 3. Core Logic

### 3.1 Resolver `0x005C0640`

Verified behavior:

1. Input token is copied into a stack buffer at `ESP+0x80`.
2. The buffer is walked byte-by-byte until either NUL or `'.'`; the first dot or NUL location is overwritten with `0`. This strips any caller extension before candidate appends.
3. String bytes at `0x0082419C` are `.BIK`; the resolver appends that suffix to the truncated base and constructs a temporary file object through `0x004739F0`.
4. The temporary file object's vtable slot `+0x14` is called with argument `0`; its result byte is the availability gate.
5. If the BIK check is false, string bytes at `0x008241A4` are `.VQA`; the resolver repeats the temporary file object check for that suffix.
6. If either candidate succeeds, the resolved filename is copied to the optional output pointer and the helper returns `AL=1`; if both fail, it returns `AL=0`.

Evidence: live MCP decompile of `0x005C0640`; assembly context `0x005C067B..0x005C0699` for extension stripping, `0x005C06A8..0x005C0712` for `.BIK`, `0x005C0714..0x005C077E` for `.VQA`, `0x005C0780..0x005C07B8` for success copy/return, and `0x005C07BB..0x005C07C7` for failure. Memory reads confirm `0x0082419C = ".BIK"` and `0x008241A4 = ".VQA"`.

Active in YR: Yes. Direct live MCP xrefs are `0x005BED54` from `FUN_005BED40` and `0x005C07EA` from `VQMovieHandle__Constructor`.

### 3.2 Constructor `0x005C07D0`

Verified behavior:

1. Null surface/input pointer returns null before resolver work (`TEST ESI,ESI`, `JZ 0x005C0964`).
2. Resolver failure returns null before any movie handle allocation (`CALL 0x005C0640`, `TEST AL,AL`, `JZ 0x005C0964`).
3. The constructor finds the resolved name's last dot with `_strrchr` at `0x007C8DF0` and compares the suffix to lowercase `.bik` at `0x0082D9CC` using case-insensitive helper `0x007C8D20`.
4. If the suffix is `.bik`, it logs/registers `Play_Movie() as Bink!\n`, allocates `0x34` bytes for the concrete Bink object, calls `0x004326C0`, then allocates a `0x14`-byte generic wrapper and writes vtable `0x007EE154`.
5. If the suffix is absent or not `.bik`, it enters the legacy VQA path: calls `0x005BFAA0`, allocates a `0x18`-byte wrapper, initializes byte `+0x14` to zero, writes vtable `0x007EE0F4`, stores the VQA object at `+0x10`, and copies width/height via `0x00759F70` / `0x00759F80`.

Evidence: live MCP decompile of `0x005C07D0`; assembly context `0x005C07D0..0x005C07F1` for null/resolver failure, `0x005C082B..0x005C0856` for suffix compare, `0x005C0858..0x005C08B7` for Bink allocation/vtable/size copy, and `0x005C08C4..0x005C0944` for VQA allocation/vtable/size copy. Memory read confirms `0x0082D9CC = ".bik"`. Decompile of `0x007C8D20` shows case-folded byte comparison, returning zero for equal strings.

Active in YR: Yes. Live MCP xrefs to `0x005C07D0` are `0x005BF3C5` in `FUN_005BF390`, `0x0061611C` in `OwnerDraw_Static_006153E0`, and `0x006161E1` in `OwnerDraw_Static_006153E0`.

### 3.3 Null and Unsupported-Fallback Contract

When neither `base.BIK` nor `base.VQA` is available, `0x005C0640` returns false and `0x005C07D0` returns null. Owner-draw callers store the returned pointer, test it, and if null do not arm the movie timer; the load path kills timer `0x65` and clears prior movie state. The full-screen movie helper `0x005BED40` returns without playback if its resolver call fails or if `g_GameMode != 0`.

Rust implication: while VQA playback is intentionally unsupported, Rust should still model the resolver result as a typed outcome: BIK candidate, VQA-selected-but-unsupported, or missing. VQA-selected is not the same as missing and should not be silently collapsed into "no movie file."

Evidence: live MCP xrefs and decompile of `OwnerDraw_Static_006153E0`, especially calls at `0x0061611C` and `0x006161E1`; decompile of `0x005BED40` with resolver call at `0x005BED54` and `g_GameMode == 0` gate; constructor null return at `0x005C0964..0x005C0970`.

Active in YR: Yes/Conditional. Owner-draw static path is active for main-menu RA2TS; `0x005BED40` is active only in the movie/credits/full-screen movie path and when `g_GameMode == 0`.

## 4. INI Keys

This wrapper slice reads no INI keys. Relevant movie declarations exist outside the slice:

| INI surface | Evidence | Effect in this report | Active in YR |
|---|---|---|---|
| `[Movies]` in `art.ini` / `artmd.ini` | `ini/art.ini:14565`, `ini/artmd.ini:19546` | Names movies available to UI/content, but not read by `0x005C0640`/`0x005C07D0` in this slice | Conditional by caller |
| `FinalMovie=` in `battle.ini` / `battlemd.ini` | multiple campaign sections; default comments say none | Campaign movie selection is outside this resolver slice | Conditional by campaign |
| `MovieOn` / `MovieOff` sounds | `ini/sound.ini`, `ini/soundmd.ini` | UI sounds, not part of resolver selection | Conditional by UI action |

## 5. Integration Points

| Integration point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| `OwnerDraw_Static_006153E0` `0x4E4` path | Destroys prior movie, calls `0x005C07D0`, stores returned handle at static record `+0x58`, sizes child from handle `+8/+0xC`, then arms timer `0x65` at `0x22` ms on success | live decompile and assembly around `0x0061611C` | Yes for main-menu movie control |
| `OwnerDraw_Static_006153E0` variant path | Same constructor/timer flow from second call site | live xref `0x006161E1` | Conditional by owner-draw message variant |
| `FUN_005BF390` | Calls `0x005C07D0`, then sets position/callback and queues the returned handle if non-null | live decompile and xref `0x005BF3C5`; callers from `TriggerAction__Execute` | Conditional by trigger action/content |
| `FUN_005BED40` | Calls resolver `0x005C0640`; if resolved suffix is `.bik`, uses full-screen Bink path; otherwise uses VQA path through `0x005BFAA0` | live decompile and xref `0x005BED54` | Conditional by movie/credits/full-screen movie path and `g_GameMode == 0` |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/ui/main_menu_shell/layout.rs::MainMenuMovieBase::asset_name` | Returns physical `ra2ts_s.bik` / `ra2ts_l.bik` names | Matches stock BIK asset existence but skips native base-token resolver mechanism |
| `src/app_main_menu_shell_render.rs::ensure_movie_for_current_layout` | Calls `AssetManager::get_with_source_ref(asset_name)` for the physical BIK, then constructs `BinkMovieSurface` | No typed BIK/VQA/missing result; VQA fallback would be reported as missing or bypassed |
| `src/assets/asset_manager.rs` | Provides exact-name lookup and `contains`/`get_ref` primitives | Can support candidate order but does not itself implement movie base resolution |
| `src/render/bink_movie.rs` | Bink-only surface/parser path | Should stay Bink-specific; VQA-selected unsupported belongs above it in a movie resolver/facade |
| `src/bin/bik-player.rs`, `src/bin/bik-survey.rs` | Physical `.bik` tools | Not generic native movie resolver behavior |

Codegraph search confirmed the relevant symbols: `BinkMovieSurface`, `AssetManager`, `MainMenuMovieBase`, `asset_name`, and `movie_base_for_screen_width`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x005C0640` extension stripping | verified | live assembly `0x005C067B..0x005C0699` | none |
| `0x005C0640` `.BIK` candidate | verified | live assembly `0x005C06A8..0x005C0712`; string bytes `0x0082419C` | lower archive priority outside this slice |
| `0x005C0640` `.VQA` fallback | verified | live assembly `0x005C0714..0x005C077E`; string bytes `0x008241A4` | VQA decoder/playback out of scope |
| `0x005C0640` success/failure return | verified | live assembly `0x005C0780..0x005C07C7` | none |
| `0x005C07D0` null-on-input/resolver-failure | verified | live assembly `0x005C07D9..0x005C07F1`, `0x005C0964..0x005C0970` | none |
| `0x005C07D0` Bink branch and vtable | verified | live assembly `0x005C0858..0x005C08B7`; vtable bytes at `0x007EE154` | corrupt BIK runtime behavior deferred |
| `0x005C07D0` VQA branch and vtable | verified | live assembly `0x005C08C4..0x005C0944`; vtable bytes at `0x007EE0F4` | VQA internals out of scope |
| immediate caller xrefs for `0x005C07D0` | verified | live MCP xrefs: `0x005BF3C5`, `0x0061611C`, `0x006161E1` | broader trigger/movie content out of scope |
| immediate caller xrefs for `0x005C0640` | verified | live MCP xrefs: `0x005BED54`, `0x005C07EA` | none for this slice |
| Rust generic movie resolver | touched-not-exhausted | Codegraph and `rg` scans | implementation not requested in this research slot |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is `0x005C0640` active in YR? -> Yes, direct live xrefs from `0x005BED40` and `0x005C07D0`; both are active infrastructure, conditionally reached by their owners.` (evidence: `0x005BED54`, `0x005C07EA`)
- `[RESOLVED] OQ-02 - Is `0x005C07D0` active in YR? -> Yes, live xrefs from owner-draw static and trigger movie path.` (evidence: `0x0061611C`, `0x006161E1`, `0x005BF3C5`)
- `[RESOLVED] OQ-03 - Does the caller extension survive? -> No, resolver truncates at first dot before appending candidates.` (evidence: `0x005C067B..0x005C0699`)
- `[RESOLVED] OQ-04 - Is BIK tried before VQA? -> Yes, `.BIK` availability check precedes the conditional `.VQA` check.` (evidence: `0x005C06A8..0x005C077E`)
- `[RESOLVED] OQ-05 - What exact strings are appended? -> Uppercase `.BIK` and `.VQA`.` (evidence: memory `0x0082419C`, `0x008241A4`)
- `[RESOLVED] OQ-06 - How is BIK branch recognized after resolution? -> `_strrchr` finds suffix and `0x007C8D20` compares case-insensitively against lowercase `.bik`.` (evidence: `0x005C083A..0x005C0856`, decompile `0x007C8D20`)
- `[RESOLVED] OQ-07 - Which vtable is installed for BIK? -> `0x007EE154`, wrapper size `0x14`, concrete pointer at `+0x10`.` (evidence: `0x005C0883..0x005C08B4`)
- `[RESOLVED] OQ-08 - Which vtable is installed for VQA? -> `0x007EE0F4`, wrapper size `0x18`, byte `+0x14=0`, concrete pointer at `+0x10`.` (evidence: `0x005C0910..0x005C0944`)
- `[RESOLVED] OQ-09 - What if neither candidate exists? -> resolver returns false, constructor returns null, owner-draw callers do not arm playback timer.` (evidence: `0x005C07BB..0x005C07C7`, `0x005C0964..0x005C0970`, `0x00616121..0x00616170`)
- `[RESOLVED] OQ-10 - Is VQA-selected equivalent to missing? -> No. A present VQA candidate reaches the VQA object construction branch, so Rust should distinguish selected-but-unsupported from missing.` (evidence: `0x005C0714..0x005C077E`, `0x005C08C4..0x005C0944`)
- `[RESOLVED] OQ-11 - Are INI keys read by this wrapper? -> No INI reads are present in `0x005C0640` or `0x005C07D0`; INI movie lists are caller/content surfaces.` (evidence: decompile `0x005C0640`, `0x005C07D0`; INI scan)
- `[RESOLVED] OQ-12 - Current Rust comparison point? -> Rust hardcodes physical BIK names for main-menu RA2TS and lacks a typed native movie resolver.` (evidence: `src/ui/main_menu_shell/layout.rs:39`, `src/app_main_menu_shell_render.rs:300`)
- `[RESOLVED] OQ-13 - Does this report need Bink frame stepping? -> No; vtable selection is enough for this target, and prior focused Bink reports cover stepping.` (evidence: target scope and vtable branch at `0x005C0897`)
- `[DEFERRED] OQ-14 - What exactly happens with a corrupt BIK that resolves by name but fails `_BinkOpen`?` (category: `needs-runtime-debugger`; reason: static branch can return a wrapper with null concrete Bink pointer, but visible runtime outcome needs a controlled corrupt file and breakpoints; next-step-if-pursued: run retail with corrupt `foo.bik` and break on `0x00432750` plus update/destroy vtable calls)
- `[DEFERRED] OQ-15 - What are VQA playback/frame/audio semantics after the fallback branch?` (category: `out-of-scope`; reason: this slot only proves selection and unsupported contract; next-step-if-pursued: dedicated VQA vtable/decoder investigation rooted at `0x005BFAA0`)
- `[DEFERRED] OQ-16 - Full archive stack order for every movie source?` (category: `requires-different-system-context`; reason: wrapper proves candidate extension order, not global archive registration priority; next-step-if-pursued: dedicated MIX movie-source priority investigation)

Adversarial checks answered or deferred: input with `.vqa` extension, input with `.bik` extension, only VQA present, neither candidate present, and selected VQA while Rust lacks VQA playback.

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `OwnerDraw_Static_006153E0` load path `0x0061611C` / `0x006161E1` | caller sends movie load message; old movie destroyed first | caller token resolved through BIK/VQA helper | `MoveWindow` uses handle width/height | concrete movie path later handles conversion | yes/conditional by message | movie object install |
| 2 | `OwnerDraw_Static_006153E0` timer path | timer `0x65`, period `0x22` after successful handle | selected BIK or VQA | current static child | concrete vtable update | yes after success | decode/update |
| 3 | `OwnerDraw_Static_006153E0` paint message `0x4F0` | handle present at static record `+0x58` | selected BIK or VQA current frame | static control | concrete vtable draw | yes after success | visible blit |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `base.BIK` | yes if availability check succeeds | yes through Bink vtable | yes for active movie control | content | no | no | no | no when present | resolver `0x005C06A8..0x005C0712`; Bink vtable `0x007EE154` |
| `base.VQA` | only if BIK missing and VQA availability succeeds | yes in native VQA path | conditional | content | no | no | no | inactive when BIK exists | resolver `0x005C0714..0x005C077E`; VQA vtable `0x007EE0F4` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Resolver strips any caller extension, then checks `base.BIK` before `base.VQA` | live assembly `0x005C067B..0x005C077E`; strings `0x0082419C`, `0x008241A4` | missing generic resolver; RA2TS shortcut uses physical BIK name | new movie resolver above `AssetManager`; `src/app_main_menu_shell_render.rs` caller | implement base-token resolution with BIK-first candidate order and extension stripping | `movie_resolver_strips_extension_and_prefers_bik_over_vqa`: fixtures `foo.vqa`, `foo.bik`, and input `foo.vqa` resolve to BIK | Do not preserve a caller-supplied `.vqa` as authoritative |
| VQA-present-after-BIK-missing is a real selected branch, even if Rust cannot play VQA yet | live assembly `0x005C0714..0x005C077E`, `0x005C08C4..0x005C0944`; vtable `0x007EE0F4` | missing typed unsupported result | movie resolver/facade, not `src/render/bink_movie.rs` | return/report `VqaUnsupported` or equivalent selected-but-unplayable state distinct from missing | `movie_resolver_returns_vqa_unsupported_when_only_vqa_exists`: fixture only `foo.vqa` returns unsupported-VQA, not not-found | Do not silently fall through to "missing movie" or try to parse VQA bytes as BIK |
| Neither candidate found returns null/no playback; owner-draw does not arm timer | live assembly `0x005C07BB..0x005C07C7`, `0x005C0964..0x005C0970`, owner caller null test after `0x0061611C` | current main-menu logs missing physical BIK and keeps no movie; generic missing state absent | `src/app_main_menu_shell_render.rs::ensure_movie_for_current_layout`, future resolver tests | preserve no-playback/null outcome separately from VQA-unsupported | `movie_resolver_returns_missing_when_bik_and_vqa_absent`: no candidates yields missing and no Bink surface construction | Do not invent PCX/still-image fallback at this wrapper |

### Negative Facts / Do Not Do

- Do not implement `VQMovieHandle` as VQA-only. Active in YR: Yes; `.bik` installs Bink vtable `0x007EE154` at `0x005C0897`.
- Do not try `.VQA` before `.BIK`. Active in YR: Yes; `.VQA` is reached only after the `.BIK` availability result is false.
- Do not preserve a caller extension through this resolver. Active in YR: Yes; first dot is overwritten with NUL before suffix append.
- Do not conflate VQA-selected with missing. Active in YR: Yes/Conditional; VQA-selected constructs a wrapper with vtable `0x007EE0F4`.
- Do not place the generic fallback logic inside the Bink parser. Active in YR: Yes; the branch is selected before concrete Bink object construction.

### Stale Docs / Follow-up Docs

- `docs/research/FUN_005C07D0_CDFILECLASS_BIK_BEFORE_VQA_GHIDRA_REPORT.md` is directionally correct on extension stripping, BIK-before-VQA, vtable selection, and null behavior, but it should be superseded by this report for live-MCP evidence.
- Replace the older caller wording "Direct calls to `FUN_005C07D0` also occur at `0x005BF189`, `0x005BF2C9`, `0x005BF3C5`" with: "Live MCP xrefs to `FUN_005C07D0` are `0x005BF3C5` in `FUN_005BF390` and `0x0061611C` / `0x006161E1` in `OwnerDraw_Static_006153E0`; `0x005BED54` calls the resolver helper `0x005C0640` directly."
- Replace broad "unsupported VQA means missing" language, if present in downstream planning, with: "VQA fallback is selected by native code when BIK is absent and VQA exists; Rust may intentionally report selected VQA as unsupported, but must not collapse it into not-found."

## Sources

- Live Ghidra MCP decompile: `0x005C0640`, `0x005C07D0`, `0x005BED40`, `0x005BF390`, `0x006153E0`, `0x005BFAA0`, `0x004326C0`, `0x007C8D20`.
- Live Ghidra MCP assembly contexts: `0x005C067B..0x005C07C7`, `0x005C07D0..0x005C0970`, `0x005BED54`, `0x005BF3C5`, `0x0061611C`, `0x006161E1`.
- Live Ghidra MCP memory reads: `0x0082419C` (`.BIK`), `0x008241A4` (`.VQA`), `0x0082D9CC` (`.bik`), vtables `0x007EE154` and `0x007EE0F4`.
- Prior doc checked: `docs/research/FUN_005C07D0_CDFILECLASS_BIK_BEFORE_VQA_GHIDRA_REPORT.md`.
- Rust/INI scans: `src/ui/main_menu_shell/layout.rs`, `src/app_main_menu_shell_render.rs`, `src/assets/asset_manager.rs`, `src/render/bink_movie.rs`, `src/bin/bik-player.rs`, `src/bin/bik-survey.rs`, `ini/art.ini`, `ini/artmd.ini`, `ini/battle.ini`, `ini/battlemd.ini`, `ini/sound.ini`, `ini/soundmd.ini`.
