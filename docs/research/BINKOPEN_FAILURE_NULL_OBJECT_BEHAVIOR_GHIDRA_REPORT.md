# BinkOpen Failure Null Object Behavior - Ghidra Research Report

**Address(es):** `0x00432750`, `0x00432690`, `0x004326C0`, `0x00432C70`, `0x005C07D0`, `0x005BED40`, `0x006153E0`, `0x005BF390`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** What active `gamemd.exe` code does after resolver/source selection succeeds but `_BinkOpen` returns null.
**Non-Scope:** Missing-file resolver failure, VQA decoder internals, corrupt-file runtime debugger observation, BINKW32 internal validation, and normal per-frame playback parity.
**Confidence:** High for static/live Ghidra branch behavior; Medium for user-visible owner-draw outcome because exact OS crash/dialog presentation needs runtime proof.
**Active in YR:** Yes/Conditional. The code paths are active infrastructure in standard YR; `_BinkOpen` null behavior is conditional on a corrupt/unsupported/source-open-failing BIK after candidate selection.

## 0. Working Notes Required Before Investigation

- **Target question:** When a BIK candidate/source has been selected but `_BinkOpen` returns null, does `gamemd.exe` return a null movie object, keep a wrapper with a null inner object, arm timers, clean up, log only, or fault?
- **Non-goals:** Do not restudy missing BIK/VQA fallback, VQA playback, Bink frame timing, DirectDraw copy format, or BINKW32 codec parsing.
- **Evidence needed to mark COMPLETE:** live read-only Ghidra decompile and assembly for `0x00432750`, constructors, fullscreen loop guard, owner-draw wrapper construction/callers, and Rust surfaces.
- **Stop conditions:** Stop if the branch requires runtime-only corrupt-file behavior beyond static proof; mark that part as uncertainty rather than guessing.

## 1. Overview

`FUN_00432750` is the concrete Bink open/init routine. On `_BinkOpen` failure it logs `Bink Error: %s\n`, returns `0`, and leaves the Bink object allocated but with `object+0x04 == 0`.

The two major callers do not handle that return the same way. Fullscreen blocking movie playback reaches `0x00432C70`, which checks `object+0x04` and returns early. Owner-draw/generic wrapper construction at `0x005C07D0` ignores the open return and dereferences `object+0x04` while copying width/height, so a null `_BinkOpen` result is not a clean "no playback" return on that path.

## 2. Class Layout / Key Offsets

| Offset / field | Behavior on `_BinkOpen` failure | Active in YR | Evidence |
|---:|---|---|---|
| Bink object `+0x04` | Receives `_BinkOpen` result. Null means open failed. | Yes | `0x00432849..0x00432855` |
| Bink object `+0x0C` | Destination surface pointer. Constructor `0x004326C0` seeds it before open; `0x00432690` leaves it zero before open. | Yes/Conditional | `0x004326C0..0x004326D8`, `0x00432690..0x004326A5` |
| Bink object `+0x20` | BSurface pointer. Remains zero on open failure because success setup is skipped. | Yes | `0x00432852..0x00432874`, success begins `0x00432877` |
| Bink object `+0x28` | Win32 handle. If file-handle mode opened a handle and `_BinkOpen` fails, the handle remains until destructor/cleanup. | Yes | handle set before `_BinkOpen` at `0x00432824..0x00432849`; cleanup `0x00432700` |
| Bink object `+0x2C` | Constructor initializes playing/unpaused flag to `1`; failure path does not rewrite it. | Yes | `0x004326AF`, `0x004326E2` |
| Bink object `+0x2D` | Constructor initializes force-frame flag to `0`; failure path does not rewrite it. | Yes | `0x00432693..0x004326B3`, `0x004326C7..0x004326E9` |
| Bink object `+0x30` | Cleared before open attempt. | Yes | `0x0043279C` |
| Generic movie wrapper `+0x10` | Stores inner Bink object pointer in BIK branch, even if that object has null `+0x04`. | Yes | `0x005C0897..0x005C08A0` |
| Generic movie wrapper `+0x08/+0x0C` | Width/height copied from `*(object+0x04)+0/+4`; unsafe when handle is null. | Yes | `0x005C08A6..0x005C08B4` |

## 3. Core Logic

### 3.1 `_BinkOpen` failure inside `0x00432750`

Active in YR: Yes, conditionally when BINKW32 fails to open the selected source.

The open function writes the `_BinkOpen` return to `object+0x04`, tests it, and branches to success only if non-null:

- `_BinkOpen` call through import pointer: `0x00432849`.
- Store handle: `MOV [ESI+0x04], EAX` at `0x0043284F`.
- Test against zero: `CMP [ESI+0x04], EBP` and `JNZ 0x00432877` at `0x00432852..0x00432855`.
- Failure path calls `_BinkGetError`, pushes its result plus string `0x00818B2C`, logs through `0x004068E0`, sets `AL=0`, and returns at `0x00432857..0x00432874`.

Success setup is skipped entirely. That means no open-time `_BinkSetVolume`, no `object+0x24` frame-tick computation, no BSurface allocation, no clipped rect, and no `_BinkDDSurfaceType` field setup.

### 3.2 Constructors ignore the boolean open result

Active in YR: Yes for all Bink users that use these constructors.

Both constructors initialize fields, call `0x00432750`, then return `this` in `EAX` without testing the boolean result:

- `0x00432690`: call at `0x004326B3`, then `MOV EAX, ESI`, `RET 0x4` at `0x004326B8..0x004326BB`.
- `0x004326C0`: call at `0x004326E9`, then `MOV EAX, ESI`, `RET 0x8` at `0x004326EE..0x004326F1`.

So `_BinkOpen` failure does not make the concrete Bink object pointer null. It makes the concrete object's handle field null.

### 3.3 Fullscreen blocking movie path handles null handle gracefully

Active in YR: Conditional. Reached by `FUN_005BED40` BIK branch for Movies/Sneak Preview/fullscreen movie playback in shell mode.

`FUN_005BED40` constructs a stack Bink object with `0x00432690`, then runs display/audio setup and calls `0x00432C70`:

- Stack object construction: `LEA ECX,[ESP+0x20]`, `CALL 0x00432690` at `0x005BEDE8..0x005BEDEC`.
- No test of the constructor/open result appears before audio/display setup.
- Blocking loop call: `LEA ECX,[ESP+0x1C]`, `CALL 0x00432C70` at `0x005BEE54..0x005BEE58`.

`0x00432C70` immediately guards `object+0x04`:

- Load and test handle: `MOV EAX,[ESI+0x04]`, `TEST EAX,EAX` at `0x00432C76..0x00432C79`.
- If non-null, jump into playback at `0x00432C96`.
- If null, call `_BinkGetError`, log `Bink Error: %s\n`, restore stack, and return at `0x00432C7D..0x00432C95`.

After that return, `FUN_005BED40` continues through its normal restore/cleanup sequence, including display/audio restoration and `0x00432700` destruction. Therefore this path is "log and no movie playback", not wrapper null and not timer-armed playback.

### 3.4 Owner-draw/generic wrapper path does not handle null handle safely

Active in YR: Yes for owner-draw shell BIK wrapper construction, including main-menu RA2TS when the selected BIK opens normally. The failure branch is conditional on `_BinkOpen` returning null.

`VQMovieHandle__Constructor @ 0x005C07D0` allocates a concrete Bink object, calls `0x004326C0`, then allocates the generic wrapper:

- Allocate concrete Bink object size `0x34`: `0x005C0862..0x005C086C`.
- Call `0x004326C0` with concrete object in `ECX`: `0x005C0876..0x005C087D`.
- Store returned object pointer in `ESI`.
- Allocate generic wrapper size `0x14`: `0x005C0883..0x005C088D`.
- Write Bink vtable and inner pointer: `MOV [EAX],0x007EE154`, `MOV [EAX+0x10],ESI` at `0x005C0897..0x005C089D`.
- Test only `ESI`, the object pointer, not `ESI+0x04`: `JZ 0x005C0966` at `0x005C08A0`.

If object allocation succeeded but `_BinkOpen` failed, `ESI` is non-null and `ESI+0x04` is zero. The constructor then performs:

- `MOV ECX,[ESI+0x04]` at `0x005C08A6`.
- `MOV EDX,[ECX]` at `0x005C08A9`.
- `MOV [EAX+0x08],EDX` at `0x005C08AB`.
- `MOV ECX,[ESI+0x04]` at `0x005C08AE`.
- `MOV EDX,[ECX+0x04]` at `0x005C08B1`.
- `MOV [EAX+0x0C],EDX` at `0x005C08B4`.

There is no null-handle guard between the failed open and these width/height reads. Static evidence therefore proves owner-draw BIK open failure is not represented as a clean null wrapper/no-timer outcome. It attempts to read through the null Bink handle during construction.

### 3.5 Owner-draw callers only see clean null on constructor allocation/resolver failure

Active in YR: Yes for owner-draw shell movie controls and trigger movie queue path.

`OwnerDraw_Static_006153E0` stores the constructor result and tests only the wrapper pointer:

- Cases `0x4DF` and `0x4E4` call `VQMovieHandle__Constructor`.
- If returned wrapper pointer is non-null, caller sets clip/position, calls `MoveWindow` with wrapper `+0x08/+0x0C`, and arms timer `0x65` at `0x22` ms.
- If constructor returns null, caller kills timer `0x65` and clears related state.

`FUN_005BF390` similarly calls `VQMovieHandle__Constructor`, then only proceeds if the returned wrapper pointer is non-null before setting position/callback and enqueueing it.

Because `_BinkOpen` failure faults or otherwise stops inside constructor before a clean return on the width/height copy, those caller-level null paths do not describe corrupt/unsupported BIK after successful source selection. They describe null input, missing resolver candidate, VQA/BIK construction allocation failure, or VQA constructor failure.

## 4. INI Keys

No INI key is read by the open-failure code itself. Movie tokens and availability gates are caller/content surfaces:

| INI / content surface | Role | Active in YR | Evidence |
|---|---|---|---|
| `artmd.ini [Movies]` / `art.ini [Movies]` | Supplies movie tokens selected upstream. | Yes for Movies picker; conditional by selected row. | Prior Movies/Credits report |
| `battlemd.ini FinalMovie=` | Supplies optional `[Movies]` index upstream. Stock repo values are blank. | Conditional | Prior FinalMovie report |

## 5. Integration Points

| Function/path | Relationship to `_BinkOpen` failure | Active in YR | Evidence |
|---|---|---|---|
| `0x00432750` | Detects null handle, logs Bink error, returns boolean `0`, leaves object allocated. | Conditional | `0x00432849..0x00432874` |
| `0x00432690` | Fullscreen/stack constructor ignores boolean return and returns object pointer. | Conditional | `0x004326B3..0x004326BB` |
| `0x004326C0` | Surface-aware constructor ignores boolean return and returns object pointer. | Yes/Conditional | `0x004326E9..0x004326F1` |
| `0x00432C70` | Fullscreen loop has null-handle guard and returns after logging. | Conditional | `0x00432C76..0x00432C95` |
| `0x005BED40` | Fullscreen BIK path constructs object, performs display/audio setup, calls guarded loop, then restores/cleans up. | Conditional | `0x005BEDE8..0x005BEE58` |
| `0x005C07D0` | Owner-draw generic constructor dereferences handle after checking only object pointer. | Yes/Conditional | `0x005C0878..0x005C08B4` |
| `0x006153E0` | Owner-draw caller arms timer only after constructor returns non-null; corrupt BIK likely never reaches this branch cleanly. | Yes/Conditional | decompile of cases `0x4DF`/`0x4E4` |
| `0x005BF390` | Trigger/queued movie path uses same constructor and wrapper-pointer-only gate. | Conditional | decompile `0x005BF390` |

## 6. Current Rust Implementation Status

Rust does not call BINKW32. It parses and decodes BIK data directly:

- `src/render/bink_movie.rs:29..46` returns `Result<Self, AssetError>` from `BinkMovieSurface::from_bytes`; parsing or first-frame decode failure returns an error and no movie surface.
- `src/app_main_menu_shell_render.rs:300..344` logs missing or failed main-menu RA2TS movie load and returns `Fallback`, leaving `state.main_menu_movie` unset.
- `src/app_main_menu_shell_render.rs:359..366` logs step/decode errors during playback.
- `src/assets/bink_file.rs:346..359` parses BIK data into `BinkFile`; `src/assets/bink_file.rs:149..182` has Rust-side validation for frame count, dimensions, fps, and audio-track count.

Current Rust behavior is more graceful than the owner-draw native corrupt-BIK branch: it logs and falls back instead of reproducing the constructor null-handle dereference. That may be acceptable as an application robustness choice only if explicitly documented as non-parity for corrupt/unsupported BIK; it is not exact gamemd behavior.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `_BinkOpen` null branch in `0x00432750` | verified | live decompile and assembly `0x00432849..0x00432874` | none |
| Constructor return behavior `0x00432690` | verified | live decompile and assembly `0x004326B3..0x004326BB` | none |
| Constructor return behavior `0x004326C0` | verified | live decompile and assembly `0x004326E9..0x004326F1` | none |
| Fullscreen null-handle guard `0x00432C70` | verified | live decompile and assembly `0x00432C76..0x00432C95` | none |
| Fullscreen caller restore/cleanup after null handle | verified | live decompile `0x005BED40`; assembly `0x005BEDE8..0x005BEE58` | exact visible frame/display state after fast return requires runtime capture |
| Owner-draw constructor null-handle dereference | verified | live decompile and assembly `0x005C0878..0x005C08B4` | exact OS/runtime crash presentation requires corrupt-file runtime |
| Owner-draw caller timer arming on clean wrapper return | verified | live decompile `0x006153E0` | none for caller gate |
| Trigger/queued movie same constructor use | verified | live decompile `0x005BF390` | broader trigger action content out of scope |
| Missing-file resolver null behavior | deferred | prior BIK/VQA resolver report | out of scope; already covered elsewhere |
| VQA constructor/open failure | deferred | prior BIK/VQA resolver report | separate VQA investigation |
| BINKW32 internal failure reasons | deferred | import boundary only | requires BINKW32/runtime oracle |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is this slice about missing candidates or selected-source open failure? -> Selected-source open failure only; missing candidate behavior is already covered by resolver reports.` (evidence: target prompt; prior `BIK_VQA_FALLBACK_AND_UNSUPPORTED_CONTRACT_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-02 - Does `0x00432750` detect `_BinkOpen` returning null? -> Yes, it tests `object+0x04` immediately after storing the `_BinkOpen` result.` (evidence: `0x00432849..0x00432855`)
- `[RESOLVED] OQ-03 - What side effects run on the open-failure branch? -> `_BinkGetError`, log `Bink Error: %s\n`, return `AL=0`; success setup is skipped.` (evidence: `0x00432857..0x00432874`)
- `[RESOLVED] OQ-04 - Does `0x00432750` close a newly opened Win32 handle when `_BinkOpen` fails? -> No immediate close is in the failure branch; cleanup/destructor closes `object+0x28` later if reached.` (evidence: `0x00432824..0x00432874`; `0x00432700`)
- `[RESOLVED] OQ-05 - Does `0x00432690` return null on failed open? -> No, it returns `this` after calling `0x00432750`.` (evidence: `0x004326B3..0x004326BB`)
- `[RESOLVED] OQ-06 - Does `0x004326C0` return null on failed open? -> No, it returns `this` after calling `0x00432750`.` (evidence: `0x004326E9..0x004326F1`)
- `[RESOLVED] OQ-07 - Does fullscreen blocking playback guard the null handle? -> Yes, `0x00432C70` returns early after logging if `object+0x04 == 0`.` (evidence: `0x00432C76..0x00432C95`)
- `[RESOLVED] OQ-08 - Does `FUN_005BED40` skip display/audio setup if open failed? -> No; it calls the constructor, then continues through display/audio setup before `0x00432C70` observes the null handle.` (evidence: `0x005BEDE8..0x005BEE58`)
- `[RESOLVED] OQ-09 - Does owner-draw wrapper constructor check the Bink handle before copying width/height? -> No; it checks only the concrete object pointer, then dereferences `object+0x04`.` (evidence: `0x005C0895..0x005C08B4`)
- `[RESOLVED] OQ-10 - Does owner-draw corrupt/unsupported BIK return a null wrapper to callers? -> Static evidence says no clean null-wrapper path for successful allocation plus null handle; the constructor dereferences the null handle before returning normally.` (evidence: `0x005C08A6..0x005C08B4`)
- `[RESOLVED] OQ-11 - Do owner-draw callers arm timer after a clean null constructor return? -> No; clean null constructor returns cause timer kill/no arming, but that path is not the `_BinkOpen` null-handle path after object allocation.` (evidence: `0x006153E0` cases `0x4DF`/`0x4E4`)
- `[RESOLVED] OQ-12 - Does `FUN_005BF390` use the same owner-draw wrapper constructor? -> Yes; it gates only on the returned wrapper pointer.` (evidence: `0x005BF390` decompile)
- `[RESOLVED] OQ-13 - Does current Rust distinguish missing, VQA-selected unsupported, and BIK open/decode failed? -> Not fully; main-menu load returns/logs `AssetError` for parse/decode and lacks the typed native movie resolver states.` (evidence: `src/render/bink_movie.rs:29..46`; `src/app_main_menu_shell_render.rs:300..344`)
- `[DEFERRED] OQ-14 - What exact Windows/runtime presentation occurs after the owner-draw null-handle dereference?` (category: `needs-runtime-debugger`; reason: static evidence proves the unsafe dereference, but not whether user sees a crash dialog, swallowed SEH, or launcher-specific handling; next-step-if-pursued: run retail with a corrupt RA2TS BIK and break on `0x005C08A6`)
- `[DEFERRED] OQ-15 - Which exact BINKW32 error strings occur for each corrupt/unsupported BIK variant?` (category: `needs-runtime-debugger`; reason: `_BinkGetError` content comes from BINKW32 runtime state; next-step-if-pursued: BINKW32/runtime oracle tests with controlled corrupt fixtures)
- `[DEFERRED] OQ-16 - How does VQA selected-but-open-failed behave?` (category: `requires-different-system-context`; reason: this slot is BIK/BinkOpen only; next-step-if-pursued: VQA fallback failure investigation)

## 9. Visual/UI Composition Ledger

This report does not claim full visual composition. It only covers what happens before or instead of playback after BIK open failure.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `0x00432750` | selected BIK source; `_BinkOpen` returns null | selected `.BIK`; no frame decoded | no success rect setup | no `_BinkDDSurfaceType` setup | Conditional | failure detection |
| 2 | `0x00432C70` | fullscreen object has null `+0x04` | selected `.BIK`; no frame decoded | no frame copy | no Bink copy | Conditional | graceful fullscreen no-playback |
| 3 | `0x005C07D0` | owner-draw object allocated, handle null | selected `.BIK`; no frame decoded | attempts width/height copy through null handle | none | Conditional | unsafe owner-draw construction |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| selected `.BIK` with failing `_BinkOpen` | Source selected; BINKW32 returns null | No proven successful frame draw | No movie frame from this failure path | Intended content | No | No | No | Conditional | `0x00432750`, `0x00432C70`, `0x005C07D0` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `_BinkOpen` null leaves concrete Bink object allocated with `+0x04 == 0`, logs Bink error, and returns boolean false from open/init. | `0x00432849..0x00432874`; constructors `0x004326B3..0x004326BB`, `0x004326E9..0x004326F1` | Rust returns `AssetError` from parse/first-frame decode and does not model a concrete object with null handle. | `src/assets/bink_file.rs`, `src/render/bink_movie.rs`, future movie resolver/error model | Represent BIK selected but open/decode failed distinctly from missing and VQA unsupported. | `movie_resolver_distinguishes_bik_selected_open_failed_from_missing`: corrupt BIK candidate yields `BikOpenFailed`/equivalent, not missing. | Do not collapse corrupt/unsupported selected BIK into missing-file fallback. |
| Fullscreen blocking BIK path logs and returns from `0x00432C70` when handle is null, then `FUN_005BED40` restores/cleans up normally. | `0x005BEDE8..0x005BEE58`; `0x00432C76..0x00432C95` | Rust has no blocking fullscreen movie wrapper yet. | Future fullscreen movie player; `src/app.rs` movie/credits/campaign states | For fullscreen playback, a selected corrupt BIK should result in no playback and cleanup of transient movie/audio/display state. | `fullscreen_bik_open_failed_logs_and_restores_without_timer`: corrupt selected BIK exits movie state and does not leave playback running. | Do not reuse owner-draw RA2TS timer semantics for fullscreen failure. |
| Owner-draw BIK wrapper construction does not cleanly return null on `_BinkOpen` failure after object allocation; it dereferences `object+0x04` while copying width/height. | `0x005C0878..0x005C08B4`; caller gates in `0x006153E0`, `0x005BF390` | Rust main-menu failure is more graceful: logs and returns `Fallback`. | `src/app_main_menu_shell_render.rs:300..344`, future parity/error-policy notes | Document this as intentional robustness drift if Rust keeps graceful fallback; do not claim native owner-draw corrupt-BIK behavior is no-playback. | `owner_draw_bik_open_failed_is_marked_non_parity_graceful_fallback`: docs/test assert Rust's graceful fallback is an explicit divergence for corrupt assets. | Do not state that `gamemd.exe` simply returns no owner-draw movie when `_BinkOpen` fails; that is only true for missing candidates or allocation failure, not null handle after allocation. |

### Negative Facts / Do Not Do

- Do not conflate "candidate missing" with "`_BinkOpen` returned null". Missing candidate returns null before Bink object construction; selected BIK open failure enters `0x00432750`.
- Do not claim both fullscreen and owner-draw failure are clean no-playback. Fullscreen `0x00432C70` guards the null handle; owner-draw `0x005C07D0` dereferences it.
- Do not treat constructor return value from `0x00432690` or `0x004326C0` as open success. Both return `this` regardless of `0x00432750` returning `AL=0`.
- Do not arm owner-draw timer `0x65` in a Rust model after BIK open/decode failure. Native would not reach normal timer arming cleanly on the null-handle branch.
- Do not hide BIK open/decode failure inside generic `missing asset`; future resolver state needs at least `Missing`, `VqaUnsupported`, and `BikOpenFailed`/decode-failed equivalents.

### Stale Docs / Follow-up Docs

- `FUN_00432750_BINK_OPEN_INIT_GHIDRA_REPORT.md` should replace "Corrupt BIK/open failure after wrapper allocation remains open" with: "Live Ghidra proof shows `0x00432750` returns `AL=0` with `object+0x04 == 0`; fullscreen `0x00432C70` logs and returns, but owner-draw `0x005C07D0` checks only the object pointer and then dereferences the null handle while copying width/height."
- `BIK_VQA_FALLBACK_AND_UNSUPPORTED_CONTRACT_GHIDRA_REPORT.md` should narrow "owner-draw callers store the returned pointer, test it, and if null do not arm the movie timer" to missing/allocation failure only. Add: "If `.BIK` is selected and concrete Bink object allocation succeeds but `_BinkOpen` returns null, the BIK branch does not cleanly return null; it dereferences the null Bink handle at `0x005C08A6..0x005C08B4`."

## Sources

- Live Ghidra MCP decompile: `0x00432750`, `0x00432690`, `0x004326C0`, `0x00432700`, `0x00432C70`, `0x005C07D0`, `0x005BED40`, `0x006153E0`, `0x005BF390`.
- Live Ghidra MCP assembly contexts: `0x00432849..0x00432874`, `0x004326B3..0x004326BB`, `0x004326E9..0x004326F1`, `0x00432C76..0x00432C95`, `0x005BEDE8..0x005BEE58`, `0x005C0878..0x005C08B4`.
- Prior docs checked: `FUN_00432750_BINK_OPEN_INIT_GHIDRA_REPORT.md`, `BIK_VQA_FALLBACK_AND_UNSUPPORTED_CONTRACT_GHIDRA_REPORT.md`, `MOVIES_CREDITS_DIALOG_PLAYBACK_FUN_005BED40_GHIDRA_REPORT.md`, `AUDIO_BEARING_BIK_PATH_AND_VOLUME_GHIDRA_REPORT.md`, `MSANIM_MSBINKANIM_CLASS_USAGE_SPLIT_GHIDRA_REPORT.md`.
- Rust files scanned read-only: `src/render/bink_movie.rs`, `src/app_main_menu_shell_render.rs`, `src/app.rs`, `src/assets/bink_file.rs`.
