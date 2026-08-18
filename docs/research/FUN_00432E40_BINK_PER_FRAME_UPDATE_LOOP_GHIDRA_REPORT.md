# FUN_00432E40 Bink Per-Frame Update Loop - Ghidra Research Report

**Address(es):** `0x00432E40` primary update loop; direct wrappers/callers `0x005C0580`, `0x00433040`, `0x00432C70`; helper/slot context `0x00432C50`, `0x00432BD0`, `0x007EE154`
**Investigation Mode:** exhaustive-slice, downgraded to partial for fresh Ghidra-MCP coverage because no running Ghidra instance was exposed.
**Claimed Scope:** per-frame ordering and return semantics inside `FUN_00432E40`: `_BinkWait`, `_BinkDoFrame`, `_BinkCopyToBuffer`, `_BinkNextFrame`, pause/resume, volume update, and catch-up loop behavior.
**Non-Scope:** BIK header parser internals, Bink audio decoding, full DirectDraw surface vtable layout, archive lookup, explicit draw composition outside this function, and runtime Bink SDK timing internals.
**Confidence:** High for instruction-level ordering in `0x00432E40` from direct `gamemd.exe` disassembly and import-table decoding; Medium for YR caller liveness where this report cites prior Ghidra-backed reports instead of fresh MCP xrefs.
**Active in YR:** Yes - standard main-menu dialog `0xE2` child static `0x71A` installs a Bink movie handle, arms timer `0x65` at `0x22` ms, and dispatches vtable `+0x04` to this update path.

## 0. Working Notes

- Target question: What exact ordering and return semantics does `FUN_00432E40` use around `_BinkWait`, `_BinkDoFrame`, `_BinkCopyToBuffer`, `_BinkNextFrame`, pause/resume, volume update, and catch-up, and what must Rust `BinkMovieSurface::step` preserve or explicitly fail to preserve?
- Non-goals: Do not investigate archive lookup, parser byte layout, decoder math, full paint composition, explicit draw pixel format, audio-track presence, or loop/end vtable slots beyond the interactions needed for this update routine.
- Evidence needed to mark COMPLETE: direct instruction evidence for all branches in `0x00432E40`; import-table names for every Bink call target; caller/liveness evidence for the `0x71A` timer path; Rust source comparison; unresolved runtime-only Bink SDK internals explicitly deferred.
- Stop conditions: Stop after the update-loop branches, return values, and Rust handoff are classified; if Ghidra MCP is unavailable, use direct binary disassembly plus prior verified reports and mark fresh-MCP-only gaps as remaining uncertainty.

## 1. Overview

`FUN_00432E40` is the concrete Bink per-frame update/copy loop. The active timer path calls it through `BinkMovie_Update_005C0580 -> FUN_00433040`, passing the destination surface and stored x/y copy coordinates. It first synchronizes volume, applies pause/resume state, optionally rejects the tick when `_BinkWait` says the next frame is not ready, then decodes/copies one or more frames until `_BinkWait` becomes nonzero after `_BinkNextFrame`.

The important Rust-facing result is that gamemd does not accumulate elapsed wall-clock time and does not cap catch-up at four frames. It asks Bink's own timing gate each poll and loops until Bink says to stop. Active in YR: Yes, via the standard main-menu Bink path; evidence: prior `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` lines 35-43 and direct wrapper disassembly at `0x005C0580`/`0x00433040`.

## 2. Class Layout / Key Offsets

| Offset | Field / role in this loop | Active in YR | Evidence |
|---|---|---|---|
| `BinkObject+0x04` | Bink SDK handle pointer; passed to all `_Bink*` imports. | Yes | Direct disassembly `0x00432E75`, `0x00432E9E`, `0x00432EC4`, `0x00432F3D`, `0x00432F5E`, `0x00432FD0`, `0x00432FF1`. |
| `BinkObject+0x08` | Bink copy format flags, ORed with `0x80000000` when helper overlay/copy flag `bl` is set. | Yes | Direct disassembly `0x00432FB9..0x00432FC6`. |
| `BinkObject+0x0C` | Destination surface pointer compared against primary and passed/called through by wrapper/update paths. | Yes | Direct disassembly `0x00432ED7..0x00432F2F`, `0x00433040..0x0043304C`. |
| `BinkObject+0x10/+0x14` | Stored x/y copy coordinates passed by wrapper `0x00433040`; adjusted by `ClientToScreen` when target is the primary surface. | Yes | Direct disassembly `0x00433040..0x0043304C`, `0x00432F08..0x00432F2F`. |
| `BinkObject+0x20` | Optional helper/subsurface object; if valid, drives `0x006C9C60`, optional pre-copy helper `0x00433330`, and flag bit `0x80000000`. | Conditional | Direct disassembly `0x00432F68..0x00432F93`; exact helper role deferred. |
| `BinkObject+0x24` | Integer timing multiplier used only by the optional helper path as `handle+0x0C * object+0x24`. | Conditional | Direct disassembly `0x00432F75..0x00432F80`; active when `+0x20` exists and helper accepts. |
| `BinkObject+0x2C` | Local playing flag toggled around `_BinkPause(handle, 1/0)`. | Yes | Direct disassembly `0x00432E87..0x00432ED1`; initialized as playing in surrounding fullscreen loop `0x00432CB0`. |
| `BinkObject+0x2D` | Force-frame flag. When nonzero, skips the initial `_BinkWait` gate and is cleared after `_BinkDoFrame`. | Conditional | Direct disassembly `0x00432F36..0x00432F58`, clear at `0x00432F6D`; setter helper `0x00433020..0x00433036`. |
| `BinkObject+0x30` | Last-frame marker written from `BinkHandle+0x0C` immediately before `_BinkNextFrame`. | Yes | Direct disassembly `0x00432FF1..0x00432FFB`; consumed by finished predicate report for `0x00432C50`. |
| `DAT_0089C490` | Cached last-sent Bink volume. | Yes | Direct disassembly `0x00432E40..0x00432E7A`; prior audio report section 5. |
| `DAT_00A8EB9C` | Current global Bink/audio volume compared as a float. | Yes | Direct disassembly `0x00432E46`, `0x00432E5A`; prior audio report section 5. |
| `DAT_00A8ED80` | Global run/pause gate for Bink pause state; nonzero path resumes, zero path pauses. | Yes | Direct disassembly `0x00432E80..0x00432ED1`; prior audio report identifies this as the game-running gate. |

## 3. Core Logic

### 3.1 Exact call order

Active in YR: Yes. Evidence: direct disassembly of `gamemd.exe` at the listed addresses; Bink import names decoded from the PE import table.

1. Compare cached volume `DAT_0089C490` to current volume `DAT_00A8EB9C` using x87 `fcomp`; if equal, skip volume work. If unequal, copy the current float to the cache, multiply by `32768.0f` (`0x007E3A70`), convert through `0x007C5F00`, then call `_BinkSetVolume@8(handle, converted_volume)` at IAT `0x007E15A0`. Evidence: `0x00432E40..0x00432E7A`; constant read `0x007E3A70 = 32768.0f`.
2. Read `DAT_00A8ED80`, then read `BinkObject+0x2C`. If the global is zero and local flag is `1`, log the pause string, set `+0x2C = 0`, and call `_BinkPause@8(handle, 1)`. Evidence: `0x00432E80..0x00432EAB`, import `0x007E15B0`.
3. If the global is nonzero and local flag is `0`, log the resume string, set `+0x2C = 1`, and call `_BinkPause@8(handle, 0)`. Evidence: `0x00432EB6..0x00432ED1`.
4. Only on the resume-transition branch, if destination surface equals `DAT_00887308`, call `GetClientRect` and `ClientToScreen`, adjust x/y, and call helper `0x004331F0`; otherwise call the helper with stored x/y. Evidence: `0x00432ED7..0x00432F31`. This is transition-side catch-up/copy work, not the main decode loop.
5. If force flag `+0x2D` is zero, call `_BinkWait@4(handle)` at IAT `0x007E15C0`. If `_BinkWait` returns nonzero, return `0` immediately. If it returns zero, decode. Evidence: `0x00432F36..0x00432F55`.
6. If force flag `+0x2D` is nonzero, skip the initial `_BinkWait` and decode. Evidence: jump from `0x00432F3B` to `0x00432F58`; flag clear at `0x00432F6D`.
7. Call `_BinkDoFrame@4(handle)` at IAT `0x007E15BC`; clear force flag; optionally run helper path at `+0x20`. Evidence: `0x00432F5E..0x00432F93`.
8. Lock/query the destination surface via vtable `+0x5C`. If that call returns nonzero, gather vtable `+0x74` and `+0x80` values, compute copy flags from `BinkObject+0x08` with optional `0x80000000`, call `_BinkCopyToBuffer@28` at IAT `0x007E15B8`, then call surface vtable `+0x60`. Evidence: `0x00432F93..0x00432FE6`.
9. If the surface vtable `+0x5C` call returns zero, skip `_BinkCopyToBuffer` and the `+0x60` unlock/finish call, but continue to post-copy helper `0x00433270`, last-frame write, `_BinkNextFrame`, and the final return path. Evidence: branch `0x00432FA4 -> 0x00432FE9`.
10. Call helper `0x00433270`, write `BinkObject+0x30 = BinkHandle+0x0C`, then call `_BinkNextFrame@4(handle)` at IAT `0x007E15B4`. Evidence: `0x00432FE9..0x00432FFB`.
11. Call `_BinkWait@4(handle)` again. If it returns zero, loop back to `_BinkDoFrame` and decode/copy another frame. If nonzero, return `1`. Evidence: `0x00433001..0x0043301C`.

### 3.2 Return values

| Condition | Return | Active in YR | Evidence |
|---|---:|---|---|
| No force flag and initial `_BinkWait(handle)` returns nonzero. | `0` | Yes | `0x00432F3D..0x00432F55`: `sete al` makes `al=0` when wait is nonzero, then epilogue returns. |
| Force flag set, regardless of initial wait readiness. | `1` after at least one `_BinkDoFrame` / `_BinkNextFrame` pass | Conditional | Force branch skips wait at `0x00432F36..0x00432F58`; return-one epilogue at `0x00433013..0x0043301C`. |
| Initial `_BinkWait(handle)` returns zero. | `1` after one or more frame advances | Yes | Ready branch reaches `_BinkDoFrame`; final epilogue sets `al=1` at `0x00433016`. |
| Destination surface lock/query returns zero. | Still `1` after `_BinkDoFrame`, helper, last-frame marker write, `_BinkNextFrame`, and final wait. | Conditional | Copy skip branch `0x00432FA4 -> 0x00432FE9`; no alternate zero return after decode. |
| End of movie / loop boundary. | Not directly returned as a distinct value by `FUN_00432E40`. | Yes | This function only returns changed/no-change. Finished/loop is handled by owner-draw timer via vtable `+0x14/+0x1C`; see `BINK_LOOP_END_VTABLE_0X14_0X1C_GHIDRA_REPORT.md`. |

### 3.3 Catch-up behavior

The loop is BinkWait-driven and unbounded by a hard integer cap in gamemd. After each `_BinkNextFrame`, the function immediately calls `_BinkWait`; if Bink returns zero, execution jumps back to the decode point at `0x00432F5E`. There is no `for max 4` equivalent, no elapsed-seconds accumulator, and no Rust-side frame interval subtraction.

Active in YR: Yes. Evidence: direct disassembly `0x00433001..0x0043300D` (`_BinkWait`, `test eax,eax`, `je 0x00432F5E`) and timer liveness from `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` lines 35-43.

### 3.4 Pause and volume ordering

Volume update always precedes pause/resume checks, and both precede the initial `_BinkWait` gate. Pause/resume can happen on a tick that ultimately returns `0` if `_BinkWait` then says no frame is ready. This matters for Rust because pause/volume state changes are not tied to successful frame upload.

Active in YR: Yes. Evidence: direct disassembly order `0x00432E40..0x00432E7A` volume, `0x00432E80..0x00432ED1` pause/resume, `0x00432F36..0x00432F55` initial wait return.

## 4. INI Keys

No INI key controls the scoped update loop. Frame cadence is delegated to the Bink SDK and the BIK header; the shell poll cadence is hardcoded by the owner-draw timer.

| Key | Status | Active in YR | Evidence |
|---|---|---|---|
| None | No INI surface found for `FUN_00432E40` update-loop ordering. | Yes | Direct disassembly contains no INI reads; prior reports route activation through messages/timer, not INI. |

## 5. Integration Points

| Integration | Role | Active in YR | Evidence |
|---|---|---|---|
| `0x007EE154 + 0x04 = 0x005C0580` | Bink movie vtable update slot. | Yes | Direct vtable read from `gamemd.exe`; wrapper disassembly `0x005C0580: mov ecx,[ecx+0x10]; jmp 0x00433040`. |
| `FUN_00433040 @ 0x00433040` | Thin wrapper passes `BinkObject+0x0C`, `+0x10`, `+0x14` as three stack args to `FUN_00432E40`. | Yes | Direct disassembly `0x00433040..0x00433051`. |
| `OwnerDraw_Static_006153E0` timer `0x65` | Calls vtable `+0x04`; invalidates only if update returns nonzero. | Yes | Prior verified Ghidra report `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` lines 35-43. |
| `0x00432C70` fullscreen/message loop | Has two direct calls to `FUN_00432E40` at `0x00432D74` and `0x00432D89`. | Conditional | Direct disassembly; not the standard `0x71A` timer path, but relevant because the helper shares the same update routine. |
| `0x00432C50` finished predicate | Reads the `+0x30` last-frame marker written by this loop. | Yes | Direct disassembly confirms predicate shape; prior loop/end report ties it to owner-draw timer vtable `+0x14`. |
| `0x00432BD0` restart | `_BinkGoto(handle, frame, 1)` and clears `+0x30`; adjacent to this loop because restart resets the marker written here. | Yes when looping | Direct disassembly; prior loop/end report ties it to owner-draw timer vtable `+0x1C`. |

## 6. Current Rust Implementation Status

| Surface | Current behavior | Delta vs verified gamemd loop |
|---|---|---|
| `src/render/bink_movie.rs:80` `BinkMovieSurface::step` | Takes caller-provided `elapsed_secs`, computes `frames_due`, loops over due frames, then uploads once if any frame changed. | Mismatch: gamemd asks `_BinkWait` before decode and after each `_BinkNextFrame`; no elapsed-time accumulator exists in `FUN_00432E40`. |
| `src/render/bink_movie.rs:90` | Caps catch-up with `frames_due(..., max=4)`. | Mismatch: gamemd has no hard cap; it loops while `_BinkWait(handle)==0`. |
| `src/render/bink_movie.rs:91..97` | Checks end before decoding the next Rust frame and can return `Ended`. | Mismatch by ownership: gamemd update returns only changed/no-change; end/loop/destroy is checked by owner-draw after update returns. |
| `src/render/bink_movie.rs:107..109` | Uploads once after all catch-up frames and returns `FrameUploaded` or `Unchanged`. | Partial match at visible abstraction level only; gamemd calls `_BinkCopyToBuffer` inside each decoded frame loop and returns `1` even if copy lock failed after decode. |
| `src/render/bink_movie.rs:135` restart helper | Flushes decoder, decodes packet index `0`, clears accumulator. | Adjacent mismatch from loop/end reports: gamemd calls `_BinkGoto(handle, 1, 1)` and clears `+0x30`; exact frame-index equivalence is still unproven. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Ghidra MCP availability | deferred | `list_instances` returned no running instances. | Re-run with Ghidra open if fresh decompiler text/caller xrefs are required. |
| PE import table for Bink calls | verified | Direct PE import parse: `_BinkSetVolume@8=0x007E15A0`, `_BinkPause@8=0x007E15B0`, `_BinkNextFrame@4=0x007E15B4`, `_BinkCopyToBuffer@28=0x007E15B8`, `_BinkDoFrame@4=0x007E15BC`, `_BinkWait@4=0x007E15C0`. | None for import identity. |
| Volume update branch | verified | Direct disassembly `0x00432E40..0x00432E7A`; constant `0x007E3A70 = 32768.0f`. | Runtime value of `DAT_00A8EB9C` not needed for order; deferred elsewhere for exact volume setting. |
| Pause/resume branch | verified | Direct disassembly `0x00432E80..0x00432ED1`; prior audio report identifies global gate. | Exact string names at `0x00818B50/0x00818B3C` not needed. |
| Initial wait/no-op return | verified | Direct disassembly `0x00432F36..0x00432F55`. | None. |
| Force-frame skip-wait path | verified | Direct disassembly `0x00432F36..0x00432F58`, setter `0x00433020..0x00433036`. | Runtime caller of setter not traced in this slot. |
| Decode/copy/next loop | verified | Direct disassembly `0x00432F5E..0x0043301C`. | Surface vtable method names for `+0x5C/+0x60/+0x74/+0x80` remain role-inferred from call placement. |
| Owner-draw timer liveness | verified-from-prior-doc | `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` lines 35-43. | Fresh Ghidra xref/caller proof unavailable in this slot. |
| Rust `BinkMovieSurface::step` comparison | verified-source | Codegraph and direct source scan of `src/render/bink_movie.rs:80..110`. | Future patch/tests. |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-001 - What is the update-loop target slice?` -> `FUN_00432E40` ordering/returns only; archive lookup, codec math, and explicit draw are out of scope. (evidence: user scope)
- `[RESOLVED] OQ-002 - Is the standard YR main-menu path live?` -> Yes; owner-draw timer `0x65` calls vtable `+0x04` and invalidates on nonzero return. (evidence: `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` lines 35-43)
- `[RESOLVED] OQ-003 - Which imports does the loop call?` -> `_BinkSetVolume`, `_BinkPause`, `_BinkWait`, `_BinkDoFrame`, `_BinkCopyToBuffer`, `_BinkNextFrame`. (evidence: PE import table plus direct call sites `0x00432E7A`, `0x00432EAB`, `0x00432ED1`, `0x00432F41`, `0x00432F62`, `0x00432FDC`, `0x00432FFB`, `0x00433005`)
- `[RESOLVED] OQ-004 - Does volume update occur before wait/decode?` -> Yes, before pause/resume and before initial `_BinkWait`. (evidence: `0x00432E40..0x00432E7A`)
- `[RESOLVED] OQ-005 - Does pause/resume occur before wait/decode?` -> Yes, and may happen even on a tick returning no frame. (evidence: `0x00432E80..0x00432F55`)
- `[RESOLVED] OQ-006 - What means no-change?` -> Initial `_BinkWait(handle) != 0` with force flag clear returns `0`. (evidence: `0x00432F3D..0x00432F55`)
- `[RESOLVED] OQ-007 - What means changed?` -> Any path that reaches `_BinkDoFrame` returns `1` after one or more `_BinkNextFrame` calls. (evidence: `0x00432F5E..0x0043301C`)
- `[RESOLVED] OQ-008 - Can copy failure produce no-change?` -> No; lock/query failure skips `_BinkCopyToBuffer` but still advances and returns `1`. (evidence: `0x00432FA4 -> 0x00432FE9`, `0x00433016`)
- `[RESOLVED] OQ-009 - Is catch-up capped?` -> No hard cap in this loop; it repeats while post-next `_BinkWait(handle)==0`. (evidence: `0x00433001..0x0043300D`)
- `[RESOLVED] OQ-010 - Is end/loop returned by this function?` -> No distinct ended return; owner-draw checks finished after update. (evidence: no branch to an ended code in `0x00432E40`; prior loop/end report)
- `[RESOLVED] OQ-011 - Does Rust currently use an elapsed accumulator?` -> Yes. (evidence: `src/render/bink_movie.rs:80..110`, `:171..183`)
- `[DEFERRED] OQ-012 - What exact semantics do Bink SDK `_BinkWait` and `_BinkPause` use internally?` (category: `needs-runtime-debugger`; reason: SDK internals are external to `gamemd.exe`; next-step-if-pursued: runtime trace or Bink SDK documentation/test harness)
- `[DEFERRED] OQ-013 - What exact surface methods are vtable `+0x5C/+0x60/+0x74/+0x80`?` (category: `requires-different-system-context`; reason: enough to prove update-loop ordering, but exact surface ABI belongs to the DirectDraw surface pipeline slot; next-step-if-pursued: investigate DSurface/BSurface vtables)
- `[DEFERRED] OQ-014 - Who calls force-frame setter `0x00433020` in standard YR?` (category: `bounded-cost-too-high`; reason: setter behavior is proved, but complete caller liveness is outside this narrow update-loop target; next-step-if-pursued: xref scan with Ghidra MCP)

## 9. Visual/UI Composition Ledger

This report covers a visual update routine, not full composition. The scoped composition effect is when Bink frame pixels are copied and when the caller knows to invalidate.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `OwnerDraw_Static_006153E0` timer `0x65` | timer id `0x65`; movie handle non-null | `ra2ts_s/l.bik` current Bink frame | static `0x71A` | Bink SDK | yes | poll/update owner |
| 2 | `0x005C0580 -> 0x00433040 -> 0x00432E40` | vtable `+0x04` | current Bink frame(s) | `BinkObject+0x10/+0x14` | `_BinkCopyToBuffer` | yes | decode/copy loop |
| 3 | `_BinkCopyToBuffer@28` | surface lock/query return nonzero | decoded frame | passed x/y args; target surface from wrapper | flags from `BinkObject+0x08`, optionally `0x80000000` | conditional | pixel copy |
| 4 | caller invalidation | update returns nonzero | last copied frame | whole static invalidated | Win32 repaint | yes | schedule repaint |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `ra2ts_s.bik` | yes at width `640` | yes | yes | content | no | no | no | no | Parent-settled fact; prior RA2TS playback reports. |
| `ra2ts_l.bik` | yes otherwise | yes | yes | content | no | no | no | no | Parent-settled fact; prior RA2TS playback reports. |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Frame readiness is `_BinkWait(handle)==0` before decode, unless force flag `+0x2D` is set; no-change is `_BinkWait!=0` and returns `0`. | Direct disassembly `0x00432F36..0x00432F55`; import `0x007E15C0`. | Rust uses `elapsed_secs` and `frames_due` instead of an explicit wait/ready abstraction. | `src/render/bink_movie.rs` `BinkMovieSurface::step`; possible playback-clock abstraction. | Preserve a Bink-shaped ready gate: one poll can return unchanged without consuming elapsed accumulator, and force-ready bypass can be represented or proven unreachable. Test proposal: `bink_step_wait_nonzero_returns_unchanged_without_advancing`. | Do not treat "less than frame_dt elapsed" as equivalent proof of Bink not ready under load/pause conditions. |
| Catch-up repeats after `_BinkNextFrame` while `_BinkWait(handle)==0`, with no hard cap in `FUN_00432E40`. | Direct disassembly `0x00432FFB..0x0043300D`. | Rust caps catch-up at four frames via `frames_due(..., 4)`. | `src/render/bink_movie.rs:90`; tests around `frames_due`. | Either remove the hard cap for parity mode or document an intentional non-parity cap; repeated ready frames must decode until the ready gate stops. Test proposal: `bink_step_decodes_until_wait_becomes_nonzero_without_fixed_cap`. | Do not preserve the max-4 cap as parity; it is a Rust safety policy, not a gamemd mechanism. |
| Update returns changed (`1`) once `_BinkDoFrame` runs, even if `_BinkCopyToBuffer` is skipped because the destination surface lock/query returned zero. | Direct disassembly `0x00432FA4 -> 0x00432FE9`, return-one epilogue `0x00433013..0x0043301C`. | Rust equates changed with successful decode/upload; no surface-lock-failure distinction exists. | `src/render/bink_movie.rs` `step`; future render-surface failure handling if modeled. | Separate decode advancement from upload success if the render path gains lock/failure states; timer invalidation should follow binary changed semantics. Test proposal: `bink_step_reports_changed_after_decode_even_when_copy_surface_unavailable`. | Do not make failed copy imply no frame advancement once decode happened. |
| Volume update and pause/resume happen before frame readiness, and can occur on a tick that returns no new frame. | Direct disassembly `0x00432E40..0x00432F55`; prior audio report section 5. | Rust `BinkMovieSurface::step` has no volume or pause state and only advances video. | `src/render/bink_movie.rs`; future audio/pause integration. | Model volume/pause as per-poll side effects, not as frame-upload side effects. Test proposal: `bink_step_applies_pause_and_volume_before_wait_noop`. | Do not tie BinkPause/BinkSetVolume to successful frame upload. |

### Negative Facts / Do Not Do

- Do not implement `BinkMovieSurface::step` as a wall-clock accumulator and claim exact parity. Evidence: gamemd gates on `_BinkWait` at `0x00432F3D` and `0x00433005`, not elapsed seconds.
- Do not keep `max 4` catch-up as a gamemd-derived behavior. Evidence: loop branch `0x0043300D -> 0x00432F5E` has no counter compare.
- Do not return "ended" from the update routine itself. Evidence: `FUN_00432E40` has only no-change `0` and changed `1`; finished/loop checks are outside in the owner-draw timer path.
- Do not skip volume/pause updates just because no frame is ready. Evidence: those branches run before initial `_BinkWait`.
- Do not equate `_BinkCopyToBuffer` success with update return value. Evidence: lock/query zero skips copy but still reaches `_BinkNextFrame` and returns `1`.

### Stale Docs / Follow-up Docs

- `docs/research/traces/MAIN_MENU_RA2TS_BACKGROUND_MOVIE_TRACE.md`
  - Replace: "Catch-up cap | Bink internal (loops while `BinkWait==0`) | max 4 frames per `step` call"
  - With: "Catch-up behavior | gamemd `FUN_00432E40` loops with no hard counter cap while post-`_BinkNextFrame` `_BinkWait(handle)==0`; current Rust caps `frames_due(..., 4)`, which is a Rust safety policy and a mechanism drift."
- `docs/research/traces/MAIN_MENU_RA2TS_BACKGROUND_MOVIE_TRACE.md`
  - Replace: "Under CPU pressure, our accumulator can advance 2-3 frames in one step call where gamemd would advance 1 per `BinkWait` poll."
  - With: "Under CPU pressure, current Rust advances according to elapsed wall-clock with a fixed max-4 cap. gamemd advances according to repeated `_BinkWait` results inside each timer poll, so the number of frames per poll is Bink-SDK-gated rather than elapsed-accumulator-gated."
- `docs/research/BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`
  - Replace: "`FUN_00432e40` also calls `_BinkWait_4` and loops `while (_BinkWait_4 == 0)`"
  - With: "`FUN_00432E40` first returns `0` when initial `_BinkWait(handle) != 0` and force flag `+0x2D` is clear; after a decode/next-frame pass it loops back while the post-`_BinkNextFrame` `_BinkWait(handle) == 0`, then returns `1`."

## Sources

- Direct binary evidence generated in this slot:
  - PE import table parse of `<ra2-install>/gamemd.exe`
  - Direct disassembly of `0x00432E40..0x0043301C`, `0x00433020..0x00433051`, `0x00432C50`, `0x00432BD0`, `0x00432C70`, `0x005C0580`, and Bink vtable `0x007EE154`
- Prior Ghidra-backed reports referenced:
  - `docs/research/BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`
  - `docs/research/RA2TS_BINK_AUDIO_ENABLE_GHIDRA_REPORT.md`
  - `docs/research/BINK_LOOP_END_VTABLE_0X14_0X1C_GHIDRA_REPORT.md`
  - `docs/research/traces/MAIN_MENU_RA2TS_BACKGROUND_MOVIE_TRACE.md`
  - `docs/research/SHELL_PARENT_BSURFACE_COMPOSITION_AND_FLIP_GHIDRA_REPORT.md`
- Rust source inspected:
  - `src/render/bink_movie.rs`
- Tooling note:
  - Ghidra MCP `list_instances` returned no running instances; no mutating Ghidra tools were called.
