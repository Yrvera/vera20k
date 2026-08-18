# MSAnim / MSBinkAnim Class Usage Split - Ghidra Research Report

**Address(es):** `0x005CC760`, `0x004F1CA0`, `0x007681E0`, `0x005C07D0`, `0x005BED40`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** MSAnim/MSBinkAnim identity, vtables, constructor/destructor enough to distinguish mission-selector animation classes from owner-draw movie handles, plus the live call sites that instantiate Bink-specific vs generic/VQA-backed movie implementations.  
**Non-Scope:** Bink frame loop internals, Bink audio decode, full VQA decoder semantics, full WDT/MSEngine visual composition, campaign `FinalMovie` launch logic.  
**Confidence:** High for vtable identities, constructors, liveness of owner-draw/main-menu and WDT loader call sites; Medium for exact user-visible WDT asset set because this pass did not dump every referenced UIMD asset.  
**Active in YR:** Yes for standard shell owner-draw movies and standard single-player mission selector WDT paths; Conditional for specific movie assets based on the named file being present/non-empty.

## 0. Working Notes

Target question: Identify the MSAnim/MSBinkAnim class split and usage sites, including vtables/constructors/destructors enough to distinguish generic animation/movie abstractions from Bink-specific implementation and VQA fallback.

Non-goals: Do not redo the settled `0x00432E40` Bink frame loop; do not audit full VQA decoder, full movie/credits playback, or campaign final movie flow; do not modify Rust or unrelated docs.

Evidence needed to mark COMPLETE: Decompile plus assembly/range evidence for MSAnim/MSBinkAnim constructor/vtable/destructor; decompile plus xref/caller evidence for all observed class instantiation paths in scope; binary proof whether VQA fallback installs a distinct vtable; Rust surface scan and concrete implementation handoff.

Stop conditions: Stop after class identity and usage split is proven; move asset enumeration, full VQA playback, audio-bearing BIK behavior, and campaign final movies to Remaining Uncertainty.

## 1. Overview

There are two separate polymorphic movie/animation families in play. The owner-draw shell movie path uses a generic movie handle allocated by `VQMovieHandle__Constructor @ 0x005C07D0`; that wrapper installs either `vtable__BinkMovieHandle @ 0x007EE154` for `.bik` or `vtable__VQMovieHandle @ 0x007EE0F4` for VQA fallback. The mission-selector animation system uses `DynamicVectorClass<MSAnim*>`; `MSBinkAnim` is a subclass of `MSAnim` with vtable `0x007EE988` and an inner Bink object at `+0x1C`.

Active in YR: Yes. The owner-draw path is live for the main-menu RA2TS panel and Movies/Credits panels. The MSAnim path is live in the WDT mission selector loader and creates `MSBinkAnim` when configured animation filenames are non-empty and available.

## 2. Class Layout / Key Offsets

| Class / object | Offset | Field | Verified behavior | Active in YR? | Evidence |
|---|---:|---|---|---|---|
| `MSAnim` base | `+0x00` | vtable | Constructor first writes `0x007EE8E8` before derived overwrite. | Yes | `0x005CC795` |
| `MSAnim` base | `+0x04/+0x08` | origin x/y | Copied from the caller rect pointer. | Yes | `0x005CC76A..0x005CC775` |
| `MSAnim` base | `+0x0C` | active byte | Set to `1` in MSBinkAnim construction. | Yes | `0x005CC778` |
| `MSAnim` base | `+0x10` | start timer | Set from `GetRadarTimer()`. | Yes | `0x005CC77C..0x005CC785` |
| `MSBinkAnim` | `+0x1C` | inner Bink object pointer | Set to `0`, then to `FUN_004326C0(...)` result when file-name gate passes. | Conditional: file token non-empty and Bink object alloc/open attempted | `0x005CC790`, `0x005CC7D7..0x005CC7FF` |
| `MSBinkAnim` | `+0x20` | Bink filename token pointer/context | Copied from constructor arg before open. | Yes when constructor called | `0x005CC79B` |
| `MSBinkAnim` | `+0x24..+0x30` | source/destination rect copy | Four dwords copied from caller rect. | Yes | `0x005CC7A2..0x005CC7BC` |
| `MSBinkAnim` | `+0x34` | MSAnim list / owner context | Stored from constructor arg; later used to notify/redraw sibling animations. | Yes | `0x005CC7B2..0x005CC7B9`, `0x005CC90A..0x005CC938` |
| `MSBinkAnim` | `+0x38` | one-byte loop/end flag | Copied from constructor byte arg; if Bink ended and this byte is `0`, tick returns done. | Yes | `0x005CC7BF..0x005CC7C5`, `0x005CC94C..0x005CC95A` |
| owner-draw generic movie handle | `+0x00` | wrapper vtable | `0x007EE154` for BIK, `0x007EE0F4` for VQA. | Yes / Conditional by resolved suffix | `0x005C0897`, `0x005C0924` |
| owner-draw generic movie handle | `+0x10` | concrete movie object | Bink object or VQA object pointer. | Yes / Conditional by resolved suffix | `0x005C089D`, `0x005C092A` |

## 3. Vtables And Constructors

### 3.1 `MSAnim` base and `MSBinkAnim`

Fresh binary-backed vtable read:

| Vtable | Type | Slot | Target | Role observed | Active in YR? | Evidence |
|---|---|---:|---:|---|---|---|
| `0x007EE8E8` | `MSAnim` base | `+0x00` | `0x005CEB60` | base destructor | Yes as base subobject | vtable dwords; constructor `0x005CC795` |
| `0x007EE8E8` | `MSAnim` base | `+0x04/+0x08/+0x0C` | `0x005CEAC0/0x005CEAD0/0x005CEB20` | shared active/pause/resume slots | Yes | vtable dwords |
| `0x007EE988` | `MSBinkAnim` | `+0x00` | `0x005CEC70` | deleting destructor wrapper | Yes | vtable dwords; destructor body reaches `0x005CC820` |
| `0x007EE988` | `MSBinkAnim` | `+0x10` | `0x005CC8A0` | tick/update/done path for Bink-backed MSAnim | Yes | vtable dwords; `0x005D1E70` calls MSAnim slot `+0x10` |
| `0x007EE988` | `MSBinkAnim` | `+0x14` | `0x005CC970` | draw/copy current Bink frame to surface | Yes when `+0x10` sees a changed frame | disasm `0x005CC8D6..0x005CC8E2`; target lacks Ghidra function boundary |
| `0x007EE988` | `MSBinkAnim` | `+0x18` | `0x005CC850` | returns stored rect at `+0x24` | Yes | disasm `0x005CC850..0x005CC871` |
| `0x007EE988` | `MSBinkAnim` | `+0x1C` | `0x005CC880` | finished predicate wrapper | Yes | disasm `0x005CC880..0x005CC898`; `0x005D1E29` can call slot `+0x1C` |
| `0x007EE9B0` | `MSVQAnim` | `+0x00..+0x20` | `0x005CECB0`, `0x005CCE90`, `0x005CD0E0`, ... | VQA-backed MSAnim sibling, not Bink | Conditional on VQA candidate availability | vtable dwords; constructor at `0x005CCC30` writes `&vtable__MSVQAnim` |

RTTI confirms the names: `0x00830100` is `.?AVMSBinkAnim@@`, with COL at `0x00806718`; `0x00830120` is `.?AVMSVQAnim@@`, with COL at `0x00806768`; base `MSAnim` type descriptor is `0x00830088`.

### 3.2 `MSAnim__Constructor @ 0x005CC760`

Verified behavior:

1. Writes base `MSAnim` vtable `0x007EE8E8` at `0x005CC795`, then overwrites with `MSBinkAnim` vtable `0x007EE988` at `0x005CC7C8`.
2. Calls `FUN_007B54B0(param_3)` at `0x005CC7CE`. Fresh decompile proves this helper is a string-length check: it returns `0` if `*param_1` is null, otherwise `strlen(*param_1)`. This corrects older uncertainty that treated it as a mysterious mode/multiplayer gate.
3. Only when that length is non-zero, allocates `0x34` bytes, resolves a filename pointer through `FUN_007B5440(param_3)`, calls `FUN_004326C0`, stores the returned Bink object at `MSBinkAnim+0x1C`, then calls `FUN_00432AB0(x,y)` directly for clip setup.
4. If the filename gate is zero, the object still returns with `MSBinkAnim` vtable but `+0x1C == 0`. Its slot `+0x10` returns done because the Bink pointer is null.

Active in YR: Yes when the WDT/mission-selector loader or screen helper constructs configured animation entries. Conditional details: Bink open is attempted only for non-empty filename tokens.

### 3.3 `MSBinkAnim` destructor path

The destructor body at `0x005CC820` sets the vtable to `0x007EE988`, reads `+0x1C`, and if non-null calls `FUN_00432700` followed by `operator delete` (`0x007C8B3D`), then writes the base `MSAnim` vtable `0x007EE8E8` before return.

Active in YR: Yes for any live `MSBinkAnim` destroyed by the MSAnim vector driver or owning screen teardown. Evidence: disasm `0x005CC820..0x005CC849`; `FUN_005D1E70` deletes finished entries through virtual slot `+0x00`.

### 3.4 Owner-draw generic movie handle

`VQMovieHandle__Constructor @ 0x005C07D0` is a separate wrapper family, not the same class as `MSAnim`.

| Resolved suffix | Allocation | Vtable installed | Concrete object | Active in YR? | Evidence |
|---|---|---|---|---|---|
| `.bik` | generic wrapper size `0x14`; Bink object size `0x34` | `0x007EE154` (`BinkMovieHandle`) | `FUN_004326C0` result at wrapper `+0x10` | Yes for RA2TS main-menu BIKs | `0x005C0858..0x005C08B7` |
| non-BIK / VQA fallback | generic wrapper size `0x18` | `0x007EE0F4` (`VQMovieHandle`) | `FUN_005BFAA0` result at wrapper `+0x10` | Conditional when BIK missing and VQA candidate selected | `0x005C08C4..0x005C0944` |

## 4. Usage Sites

### 4.1 Main-menu owner-draw static

`OwnerDraw_Static_006153E0` cases `0x4DF` and `0x4E4` destroy any existing movie handle, call `VQMovieHandle__Constructor`, store the returned wrapper at owner-draw record `+0x58`, use wrapper `+8/+0xC` for `MoveWindow`, and arm timer `0x65` at `0x22` ms. Timer `0x65` uses the wrapper vtable slots, not `MSAnim` slots.

Active in YR: Yes. Standard main-menu dialog `0xE2` sends `0x4E4` for `Ra2ts_s/l`.

Evidence: decompile of `OwnerDraw_Static_006153E0`, dispatch ranges around `0x006160E8..0x00616275`, plus prior Bink vtable report.

### 4.2 Movies/Credits fullscreen playback

`FUN_005BED40` is not `MSAnim` and does not allocate the owner-draw generic wrapper for its BIK branch. It resolves the movie name, checks for `.bik`, and if `.bik` directly enters Bink fullscreen playback helpers (`FUN_00432690`, `FUN_00432C70`, display chain pause/resume). If not `.bik`, it constructs the VQA playback object through the VQA path and uses VQA fullscreen helpers.

Active in YR: Yes for the main-menu Movies/Credits panel: Sneak Preview plays hardcoded `RENEGADE.BIK`, and the Movies picker uses `[Movies]` entries from `artmd.ini`.

Evidence: `FUN_005BED40` decompile; caller chain from `Main_Game` case 4 and `MOVIES_AND_CREDITS_DIALOG_CASE4_GHIDRA_REPORT.md`.

### 4.3 WDT / mission-selector MSAnim path

`0x007681E0` loads UIMD/WDT layout keys. For the `Opening` key at string `0x00848ADC`, it reads a filename into a local string, calls `FUN_007B54B0`, allocates `0x3C`, calls `MSAnim__Constructor @ 0x005CC760`, stores the returned pointer at screen object `+0x18`, and adds it to the MSAnim vector via `FUN_005D1C20`.

Active in YR: Yes for the single-player mission selector WDT loader when `Opening` is non-empty. Evidence: `0x0076861F..0x007686BD`; prior `UIMD_ART800_LOADER_GHIDRA_REPORT.md`.

### 4.4 WDT screen helper `0x004F1CA0`

This helper chooses among `MSVQAnim`, `MSBinkAnim`, and fallback animation classes for a WDT/mission-selector screen:

1. It builds/checks `.VQA` first at `0x004F1DCC..0x004F1E21`; if available, allocates `0x30` and calls the `MSVQAnim` constructor at `0x005CCC30`.
2. If VQA is unavailable, it builds/checks `.BIK` at `0x004F1E58..0x004F1EB6`; if available, allocates `0x3C` and calls `MSAnim__Constructor @ 0x005CC760`, yielding `MSBinkAnim`.
3. If both are unavailable or construction fails, it falls back to another MS animation class at `0x005CE4A0` (MSPCXAnim per vtable write `0x007EEA58` in that constructor), not to owner-draw `VQMovieHandle`.

Active in YR: Yes/Conditional. The helper is called in WDT screen construction; the concrete subclass depends on which asset extension is present.

This order is intentionally different from owner-draw `VQMovieHandle__Constructor`, whose resolver tries `.BIK` before `.VQA`.

### 4.5 MSAnim vector driver

`FUN_005D1E70` iterates the `MSAnim*` vector. For each entry it calls virtual slot `+0x10`; if the slot returns `1`, the driver deletes the object through virtual slot `+0x00` and compacts the vector. It also uses slots `+0x14`, `+0x18`, and `+0x1C` in related redraw/finished paths.

Active in YR: Yes for WDT mission selector animations. Evidence: disasm/decompile around `0x005D1E70..0x005D1FC4`; prior `MSFADEANIM_SIBLING_CLASS_GHIDRA_REPORT.md`.

## 5. INI / Data Keys

| Key / source | Meaning in this slice | Active in YR? | Evidence |
|---|---|---|---|
| WDT/UIMD `Opening` | Filename token passed to `MSAnim__Constructor`; non-empty token activates Bink open attempt. | Conditional on key value | binary string `0x00848ADC`; loader `0x0076861F..0x007686BD` |
| WDT/UIMD `Background` | Loaded by a separate MS animation constructor, not MSBinkAnim at the observed `0x007681E0` site. | Yes | binary string `0x008241B4`; loader `0x007686E1..0x0076876C` |
| `artmd.ini [Movies]` | Source list for movie picker; selected names pass to `FUN_005BED40`, not `MSAnim__Constructor`. | Yes | `ini/artmd.ini`, `MOVIES_AND_CREDITS_DIALOG_CASE4_GHIDRA_REPORT.md` |
| `battlemd.ini FinalMovie` | Campaign final-movie token; not traced in this slot. | Deferred | `ini/battlemd.ini`; no binary reader traced here |

## 6. Current Rust Implementation Status

Rust currently models concrete Bink playback for the main-menu RA2TS movie, but not either native polymorphic abstraction:

| Rust surface | Status against this report |
|---|---|
| `src/render/bink_movie.rs` | Bink-specific video surface. It does not model `VQMovieHandle`, `MSAnim`, `MSBinkAnim`, or `MSVQAnim` vtables. |
| `src/ui/main_menu_shell/layout.rs::MainMenuMovieBase::asset_name` | Returns physical `.bik` asset names; native owner-draw caller sends base tokens and `VQMovieHandle__Constructor` resolves extensions. |
| `src/app_main_menu_shell_render.rs` | Loads RA2TS BIK into one GPU texture path. It does not represent owner-draw wrapper class split or VQA fallback. |
| `src/bin/bik-player.rs`, `src/bin/bik-survey.rs` | BIK tools only; no MSAnim abstraction. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MSAnim__Constructor @ 0x005CC760` | verified | decompile plus disasm `0x005CC760..0x005CC816` | none for class identity |
| `MSBinkAnim` vtable `0x007EE988` | verified | binary dword read; RTTI `0x00830100` | exact slot `+0x14` body has no Ghidra function boundary; disasm only |
| `MSBinkAnim` destructor body `0x005CC820` | verified | disasm `0x005CC820..0x005CC849` | deleting wrapper `0x005CEC70` not deeply decompiled |
| `FUN_007B54B0` gate | verified | decompile `0x007B54B0` | none |
| `VQMovieHandle__Constructor @ 0x005C07D0` BIK/VQA split | verified | decompile plus disasm `0x005C0858..0x005C0944`; prior BIK-before-VQA report | full VQA decoder out-of-scope |
| `FUN_005BED40` Movies/Credits fullscreen split | touched-not-exhausted | decompile `0x005BED40`; prior Movies/Credits report | fullscreen Bink/VQA internals and audio-bearing movies |
| WDT `Opening` path `0x007681E0` | verified for MSBinkAnim call | disasm `0x0076861F..0x007686BD`; decompile of containing function | exact UIMD file/default values deferred |
| WDT helper `0x004F1CA0` extension choice | verified | decompile plus disasm `0x004F1DCC..0x004F1F6B` | caller context and all fallback visual classes not exhausted |
| `MSVQAnim` constructor `0x005CCC30` | verified for distinct vtable install | decompile writes `&vtable__MSVQAnim` | VQA update/render semantics deferred |
| Rust abstraction parity | touched-not-exhausted | file scan of Bink/main-menu surfaces | no Rust edit in this research slot |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is MSBinkAnim a distinct class from MSAnim? -> Yes; constructor writes base MSAnim vtable then overwrites with MSBinkAnim vtable 0x007EE988; RTTI names MSBinkAnim at 0x00830100.` (evidence: `0x005CC795`, `0x005CC7C8`, `0x00830100`)
- `[RESOLVED] OQ-02 - What owns the inner Bink object in MSBinkAnim? -> Field +0x1C stores the `FUN_004326C0` result and destructor destroys/frees it.` (evidence: `0x005CC7F6..0x005CC7FF`, `0x005CC824..0x005CC83E`)
- `[RESOLVED] OQ-03 - Is the MSAnim Bink path gated by a global multiplayer flag? -> No; `FUN_007B54B0` is a null/strlen helper over the filename pointer.` (evidence: `0x007B54B0` decompile)
- `[RESOLVED] OQ-04 - Does owner-draw VQA fallback install a distinct vtable? -> Yes; wrapper vtable is `0x007EE0F4`, distinct from Bink wrapper `0x007EE154`.` (evidence: `0x005C0897`, `0x005C0924`)
- `[RESOLVED] OQ-05 - Does MSAnim/WDT VQA fallback install a distinct vtable? -> Yes; `MSVQAnim` constructor writes `&vtable__MSVQAnim`, vtable `0x007EE9B0`.` (evidence: `0x005CCC30` decompile; vtable dwords)
- `[RESOLVED] OQ-06 - Which live path uses `VQMovieHandle__Constructor`? -> Owner-draw static movie messages `0x4DF/0x4E4`, including main-menu RA2TS.` (evidence: `OwnerDraw_Static_006153E0`; callers of `VQMovieHandle__Constructor`)
- `[RESOLVED] OQ-07 - Which live path uses MSBinkAnim? -> WDT/mission-selector loader and helper paths create `MSBinkAnim` for non-empty/configured BIK candidates.` (evidence: `0x0076861F..0x007686BD`, `0x004F1E58..0x004F1F01`)
- `[RESOLVED] OQ-08 - Is Movies/Credits fullscreen playback MSAnim? -> No for the observed `FUN_005BED40` path; it uses direct fullscreen Bink or VQA playback helpers.` (evidence: `FUN_005BED40` decompile)
- `[RESOLVED] OQ-09 - Does WDT helper extension order match owner-draw extension order? -> No; WDT helper checks VQA before BIK, while owner-draw `0x005C0640` checks BIK before VQA.` (evidence: `0x004F1DCC..0x004F1EB6`; prior `FUN_005C07D0...` report)
- `[RESOLVED] OQ-10 - What Rust surface currently models this? -> Rust has Bink-specific RA2TS playback only, not these polymorphic wrappers/classes.` (evidence: `src/render/bink_movie.rs`, `src/app_main_menu_shell_render.rs`, `src/ui/main_menu_shell/layout.rs`)
- `[DEFERRED] OQ-11 - Full VQA decoder/update/draw semantics after fallback.` (category: out-of-scope; reason: this slot proves distinct class/vtable installation only; next-step-if-pursued: investigate `0x005BFAA0`, `0x005CCC30`, and vtables `0x007EE0F4`/`0x007EE9B0` internals)
- `[DEFERRED] OQ-12 - Which exact WDT/UIMD keys are non-empty in stock YR assets.` (category: requires-different-system-context; reason: binary loader proved key usage, but this pass did not extract retail UIMD files; next-step-if-pursued: dump UIMD entries and compare `Opening`, `Background`, and related keys)
- `[DEFERRED] OQ-13 - Campaign `FinalMovie` reader and playback path.` (category: out-of-scope; reason: visible in `battlemd.ini` but not needed for MSAnim/MSBinkAnim class split; next-step-if-pursued: trace `FinalMovie` INI reads to playback caller)
- `[DEFERRED] OQ-14 - Exact body of MSBinkAnim slot `+0x14 @ 0x005CC970`.` (category: bounded-cost-too-high; reason: Ghidra has no function boundary; disassembly proves dispatch target and copy role, but full body requires manual function-boundary reconstruction; next-step-if-pursued: hand-disassemble `0x005CC970..0x005CCA08`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Owner-draw shell movies use a generic wrapper that selects Bink vtable `0x007EE154` or VQA vtable `0x007EE0F4` after BIK-before-VQA resolution. | `0x005C0640`; `0x005C0858..0x005C0944`; owner-draw calls in `0x006153E0` | missing; Rust directly requests `.bik` for RA2TS | `src/app_main_menu_shell_render.rs`, `src/ui/main_menu_shell/layout.rs`, future movie resolver facade | preserve base-token resolution and expose typed BIK/VQA/unsupported result before concrete playback | `movie_handle_resolver_prefers_bik_and_reports_vqa_fallback`: fixture with both extensions chooses BIK; fixture with only VQA reports VQA selected | Do not model `VQMovieHandle` as VQA-only or hardwire every owner-draw movie to BIK-only. |
| MSBinkAnim is an `MSAnim` subclass, not the owner-draw Bink wrapper; it owns an inner Bink object at `+0x1C` and is driven by the MSAnim vector. | `0x005CC760`; vtable `0x007EE988`; driver `0x005D1E70` | missing; no WDT/MSAnim animation system exists | future WDT/mission-selector UI module, not `src/render/bink_movie.rs` alone | represent mission-selector animations as polymorphic/typed entries whose Bink variant can tick/draw/delete through the MSAnim driver ordering | `msanim_bink_variant_constructs_inner_bink_only_for_nonempty_token`: non-empty token creates Bink variant; empty token is immediate-done/no inner Bink | Do not reuse main-menu timer semantics for MSBinkAnim; it is driven by `FUN_005D1E70`, not `WM_TIMER 0x65`. |
| WDT helper `0x004F1CA0` checks `.VQA` before `.BIK` and selects `MSVQAnim` before `MSBinkAnim`; this is opposite the owner-draw wrapper's BIK-before-VQA resolver. | `0x004F1DCC..0x004F1EB6`; `0x005CCC30`; `0x005CC760` | unchecked/missing; Rust has no WDT resolver | future WDT asset resolver | resolver order must be context-specific: owner-draw movie base uses BIK-first, WDT MSAnim helper uses VQA-first | `wdt_msanim_resolver_prefers_vqa_over_bik`: fixture with both extensions selects VQA variant; owner-draw fixture with same names selects BIK | Do not centralize all movie resolution into one global BIK-first helper. |

## 10. Negative Facts / Do Not Do

- Do not treat `FUN_007B54B0()` as a multiplayer/display-mode gate. Active in YR: Yes; decompile proves it is a filename null/length helper.
- Do not conflate `MSBinkAnim` vtable `0x007EE988` with owner-draw `BinkMovieHandle` vtable `0x007EE154`. They share Bink helper callees but are different wrapper classes and drivers.
- Do not assume VQA fallback uses the same vtable as Bink. Owner-draw fallback uses `0x007EE0F4`; WDT/MSAnim fallback uses `MSVQAnim` vtable `0x007EE9B0`.
- Do not apply BIK-before-VQA globally. Owner-draw `VQMovieHandle` is BIK-first; WDT helper `0x004F1CA0` is VQA-first.
- Do not route Movies/Credits fullscreen playback through `MSAnim` based on class names alone; `FUN_005BED40` is a separate direct fullscreen movie path.

## 11. Remaining Uncertainty

- Full VQA playback internals for both owner-draw `VQMovieHandle` and WDT `MSVQAnim`.
- Exact stock UIMD/WDT asset values for every key that can create `MSBinkAnim` or `MSVQAnim`.
- Campaign `FinalMovie` reader and whether it uses `FUN_005BED40`, owner-draw wrapper, or another path.
- Full manual reconstruction of `MSBinkAnim` slot `+0x14 @ 0x005CC970`.

## 12. Stale Docs / Follow-up Docs

- `FUN_00432AB0_BINK_CLIP_RECT_SETTER_GHIDRA_REPORT.md` says the `MSAnim__Constructor` path is gated on `FUN_007b54b0() != 0` and defers what that means. Replacement wording:

  > `FUN_007B54B0` is a filename-token length helper: it returns zero if `*param_1` is null, otherwise `strlen(*param_1)`. The `MSAnim__Constructor @ 0x005CC760` Bink-open path is therefore conditional on a non-empty configured filename token, not on an unknown multiplayer/display gate.

- `RA2TS_BINK_AUDIO_ENABLE_GHIDRA_REPORT.md` describes `MSAnim__Constructor @ 0x005CC760` as a separate Bink usage. Add:

  > This constructor installs `MSBinkAnim` vtable `0x007EE988`; it is an `MSAnim` subclass driven by the WDT/MSEngine `MSAnim*` vector, distinct from owner-draw `BinkMovieHandle` vtable `0x007EE154`.

## Sources

- Live Ghidra MCP decompile: `MSAnim__Constructor @ 0x005CC760`, `VQMovieHandle__Constructor @ 0x005C07D0`, `FUN_005BED40`, `FUN_007B54B0`, `FUN_007B5440`, `0x004F1CA0`, `0x007681E0`, `0x005CCC30`.
- Read-only local binary disassembly / vtable reads from `gamemd.exe`, image base `0x00400000`: `0x005CC760..0x005CCA10`, `0x004F1DCC..0x004F1F6B`, `0x0076861F..0x0076876C`, vtables `0x007EE8E8`, `0x007EE988`, `0x007EE9B0`, `0x007EE154`, `0x007EE0F4`.
- Existing docs: `BINK_VTABLE_0X007EE154_SLOT_MAP_GHIDRA_REPORT.md`, `FUN_005C07D0_CDFILECLASS_BIK_BEFORE_VQA_GHIDRA_REPORT.md`, `FUN_00432AB0_BINK_CLIP_RECT_SETTER_GHIDRA_REPORT.md`, `MSFADEANIM_SIBLING_CLASS_GHIDRA_REPORT.md`, `MOVIES_AND_CREDITS_DIALOG_CASE4_GHIDRA_REPORT.md`, `UIMD_ART800_LOADER_GHIDRA_REPORT.md`, `RA2TS_BINK_AUDIO_ENABLE_GHIDRA_REPORT.md`.
- INI scan: `ini/artmd.ini [Movies]`, `ini/battlemd.ini FinalMovie`.
- Rust scan: `src/render/bink_movie.rs`, `src/app_main_menu_shell_render.rs`, `src/ui/main_menu_shell/layout.rs`, `src/bin/bik-player.rs`, `src/bin/bik-survey.rs`.
