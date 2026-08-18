# Bink Update Loop 0x00432E40 Fresh MCP Audit - Ghidra Report

**Address(es):** `0x00432E40`, wrappers `0x00433040` and `0x005C0580`, Bink vtable `0x007EE154`, owner-draw timer path `0x006153E0`
**Investigation Mode:** focused `/re-swarm` slot 4 fresh MCP audit
**Status:** COMPLETE
**Active in YR:** Yes. Standard owner-draw movie static stores a Bink-backed handle with vtable `0x007EE154`, arms timer `0x65`, and the timer dispatch calls vtable `+0x04 -> 0x005C0580 -> 0x00433040 -> 0x00432E40`.
**Confidence:** High for scoped update-loop ordering, return values, and timer/vtable liveness. Evidence is fresh Ghidra MCP decompile/disassembly plus vtable memory read.

## 0. Working Notes

- Target question: Does fresh live Ghidra MCP confirm `0x00432E40` update-loop ordering and return semantics: volume before pause/wait, `_BinkWait` no-op return, force flag `+0x2D` skip, `_BinkDoFrame` / `_BinkCopyToBuffer` / `_BinkNextFrame` order, no fixed catch-up cap, changed return on copy skip, and active owner-draw timer path?
- Non-goals: Do not investigate BIK/VQA resolution, header parsing, decoder math, finished predicate internals, restart/`BinkGoto` semantics, or DirectDraw surface ABI beyond the calls used by this loop.
- Evidence needed to mark COMPLETE: fresh MCP decompile/disassembly of `0x00432E40`; fresh wrapper/vtable evidence for `+0x04`; fresh owner-draw timer evidence for timer `0x65`; focused Rust source scan for affected surfaces.
- Stop conditions: Stop after proving this update-loop slice and Rust handoff; record external Bink SDK internals and loop/restart semantics as remaining uncertainty instead of expanding scope.

## 1. Summary

Fresh MCP confirms the prior partial static-disassembly report. `FUN_00432E40` is Bink-SDK-timed, not elapsed-accumulator-timed. It updates volume first, applies pause/resume state next, then checks `_BinkWait(handle)` unless force flag `BinkObject+0x2D` is set. A ready/forced tick calls `_BinkDoFrame`, optionally copies pixels with `_BinkCopyToBuffer`, writes the last-frame marker from `BinkHandle+0x0C` into `BinkObject+0x30`, calls `_BinkNextFrame`, then loops while the post-next `_BinkWait(handle)` returns zero.

There is no fixed catch-up counter in this function. The update routine returns `0` only on the initial no-ready wait path. Any path that reaches `_BinkDoFrame` returns changed (`1`) after advancing, even when the destination surface lock/query returned zero and `_BinkCopyToBuffer` was skipped.

## 2. Active Caller And Vtable Path

| Finding | Active in YR | Evidence |
|---|---|---|
| Bink-backed movie handles install vtable `0x007EE154`. | Yes | MCP disassembly `0x005C0895..0x005C089D`: after Bink object allocation, writes `dword ptr [EAX] = 0x007EE154`; xref to vtable from `0x005C0897`. |
| Vtable `+0x04` is update slot `0x005C0580`. | Yes | MCP `read_memory 0x007EE154 length 64`: DWORDs begin `0x005C0A30, 0x005C0580, ...`; `+0x04` is `0x005C0580`. |
| `0x005C0580` unwraps `BinkMovieHandle+0x10` then jumps to `0x00433040`. | Yes | MCP disassembly `0x005C0580..0x005C0583`: `mov ecx,[ecx+0x10]`; `jmp 0x00433040`. |
| `0x00433040` passes `BinkObject+0x0C`, `+0x10`, `+0x14` to `0x00432E40`. | Yes | MCP disassembly `0x00433040..0x0043304C`: loads `[ECX+0x14]`, `[ECX+0x10]`, `[ECX+0x0C]`, pushes them, calls `0x00432E40`. |
| Owner-draw timer `0x65` calls vtable `+0x04` and invalidates only on nonzero return. | Yes | MCP disassembly `0x00615B80..0x00615BA0`: compares timer id with `0x65`, loads movie handle from owner-draw record `+0x58`, calls `[vtable+0x04]`, tests `AL`, then `InvalidateRect` only if nonzero. |
| The same owner-draw path arms timer `0x65` at `0x22` ms after movie creation. | Yes | MCP disassembly `0x00616121..0x0061615E`: stores movie handle at `owner+0x58`, moves window to Bink width/height, then `SetTimer(hwnd, 0x65, 0x22, 0)`. |

## 3. Update-Loop Mechanics

### Volume And Pause Before Wait

Active in YR: Yes.

Fresh MCP decompile and disassembly show `0x00432E40` starts by comparing cached float `DAT_0089C490` against current float `DAT_00A8EB9C`. On mismatch it copies the current value, multiplies by the constant at `0x007E3A70`, converts through `0x007C5F00`, and calls `_BinkSetVolume@8` through IAT `0x007E15A0`.

Evidence: MCP disassembly `0x00432E40..0x00432E7A`; MCP xrefs to `0x007E15A0` include `0x00432E7A`.

Pause/resume follows volume and precedes initial wait. If global run state `DAT_00A8ED80` is zero and object byte `+0x2C` is one, the function sets `+0x2C = 0` and calls `_BinkPause(handle, 1)`. If global run state is nonzero and `+0x2C` is zero, it sets `+0x2C = 1`, calls `_BinkPause(handle, 0)`, and runs a transition helper with stored or client-origin-adjusted coordinates.

Evidence: MCP decompile `0x00432E40`; MCP disassembly `0x00432E80..0x00432F31`.

### Initial Wait And Force Flag

Active in YR: Yes for the no-ready branch; Conditional for the force flag, because the branch is present in the active routine but setter liveness was not in scope.

If `BinkObject+0x2D == 0`, the routine calls `_BinkWait@4(handle)` through IAT `0x007E15C0`. If `_BinkWait` returns nonzero, the function returns `0` immediately. If `+0x2D != 0`, it skips this initial wait gate and enters decode. The flag is cleared after `_BinkDoFrame`.

Evidence: MCP disassembly `0x00432F36..0x00432F58`; clear at `0x00432F6D`; MCP xrefs to `0x007E15C0` include `0x00432F41` and `0x00433005`.

### Decode / Copy / NextFrame Order

Active in YR: Yes for normal decode/copy/advance order; Conditional for the copy-skip branch when destination surface lock/query fails.

The ready/forced path calls `_BinkDoFrame@4(handle)` first. It then optionally runs the `BinkObject+0x20` helper path, then calls destination surface vtable `+0x5C` as a lock/query. If that returns nonzero, it reads surface values via vtable `+0x74` and `+0x80`, computes copy flags from `BinkObject+0x08` plus optional `0x80000000`, calls `_BinkCopyToBuffer@28`, then calls vtable `+0x60`. If lock/query returns zero, it skips `_BinkCopyToBuffer` but does not return.

After copy or copy-skip, the loop calls helper `0x00433270`, writes `BinkObject+0x30 = BinkHandle+0x0C`, and calls `_BinkNextFrame@4(handle)`.

Evidence: MCP decompile `0x00432E40`; MCP disassembly `_BinkDoFrame` at `0x00432F5E..0x00432F62`, copy path `0x00432F93..0x00432FE6`, copy-skip branch `0x00432FA4 -> 0x00432FE9`, marker/next at `0x00432FE9..0x00432FFB`.

### Catch-Up And Return Value

Active in YR: Yes.

After `_BinkNextFrame`, the function calls `_BinkWait(handle)` again. If it returns zero, execution jumps back to `_BinkDoFrame` at `0x00432F5E`. If it returns nonzero, the function returns `1`. There is no loop counter, no elapsed-seconds accumulator, and no fixed maximum of four frames.

Evidence: MCP disassembly `0x00433001..0x0043301C`: call `_BinkWait`, `test eax,eax`, `jz 0x00432F5E`, return `AL=1`.

Changed return is tied to reaching decode/advance, not copy success. The copy-skip branch still flows through marker write, `_BinkNextFrame`, final wait, and the return-one epilogue.

Evidence: MCP disassembly branch `0x00432FA4 -> 0x00432FE9`; return-one epilogue `0x00433013..0x0043301C`.

Finished/restart handling is external to this routine. The owner-draw timer calls vtable `+0x14` after update, and may call `+0x1C` for looping. `0x00432E40` itself returns only changed/no-change.

Evidence: MCP disassembly `0x00615B99..0x00615BD0`: timer calls `[vtable+0x04]`, then `[vtable+0x14]`, and only if finished/looping calls `[vtable+0x1C]`.

## 4. Rust Scan

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/render/bink_movie.rs:84..110` `BinkMovieSurface::step` | Uses caller-provided `elapsed_secs`, `frames_due`, and uploads once after the loop. | Drift: gamemd polls `_BinkWait` before decode and after each `_BinkNextFrame`; upload/copy is inside each decoded-frame pass. |
| `src/render/bink_movie.rs:90` | Uses `frames_due(..., 4)`. | Drift: no hard catch-up cap exists in `0x00432E40`. |
| `src/render/bink_movie.rs:91..97` | Returns `Ended` from the step loop and handles looping before decoding. | Ownership drift: finished/loop is external to `0x00432E40`, checked by owner-draw after update. |
| `src/render/bink_movie.rs:107..109` | Reports `FrameUploaded` only when upload occurs. | Future surface-failure drift risk: gamemd returns changed after decode/advance even when `_BinkCopyToBuffer` is skipped. |
| `src/render/bink_movie.rs:171..183` | `frames_due` is an elapsed accumulator with a `max` parameter. | Not gamemd mechanism; it is a Rust timing policy. |

## 5. Implementation Handoff

- `_BinkWait(handle)==0` gates decode before `_BinkDoFrame`, unless force flag `+0x2D` is set -> Rust uses elapsed seconds and `frames_due` as the readiness gate -> affected surface `src/render/bink_movie.rs::BinkMovieSurface::step` and possible playback-clock abstraction -> acceptance scenario: a poll with wait-nonzero returns unchanged without advancing or consuming a frame, while a forced poll bypasses initial wait -> proposed test name `bink_step_wait_nonzero_returns_unchanged_without_advancing` -> risk: treating “less than frame_dt elapsed” as equivalent to Bink not-ready is unproven under pause/load/SDK timing.
- Catch-up repeats after `_BinkNextFrame` while post-next `_BinkWait(handle)==0`, with no fixed counter cap -> Rust caps catch-up at four frames via `frames_due(..., 4)` -> affected surface `src/render/bink_movie.rs:90` and tests around `frames_due` -> acceptance scenario: repeated ready results decode until the ready gate stops, with no gamemd-derived max-4 cap -> proposed test name `bink_step_decodes_until_wait_becomes_nonzero_without_fixed_cap` -> risk: preserving max-4 as parity hides a real mechanism drift.
- Once `_BinkDoFrame` runs, update returns changed after advancing even if destination lock/query skips `_BinkCopyToBuffer` -> Rust currently has no separate surface-lock failure path and equates changed with successful decode/upload -> affected surface `src/render/bink_movie.rs::step` and any future DirectDraw-like surface abstraction -> acceptance scenario: decode/advance reports changed even when copy/upload target is unavailable -> proposed test name `bink_step_reports_changed_after_decode_even_when_copy_surface_unavailable` -> risk: making copy/upload failure imply no frame advancement will desynchronize loop/end marker behavior.

## 6. Negative Facts / Do Not Do

- Do not implement main-menu Bink stepping as a wall-clock accumulator and claim exact parity. Evidence: MCP disassembly calls `_BinkWait` at `0x00432F41` and `0x00433005`; no elapsed accumulator exists in `0x00432E40`.
- Do not keep the Rust `max 4` catch-up cap as a gamemd-derived behavior. Evidence: MCP disassembly `0x00433001..0x0043300D` has only `_BinkWait`, `test`, and `jz 0x00432F5E`; no counter compare.
- Do not return or handle “ended” inside the update-loop equivalent. Evidence: MCP owner-draw timer `0x00615B99..0x00615BD0` checks finish/restart after vtable `+0x04`; `0x00432E40` returns only `0` or `1`.
- Do not skip volume or pause updates just because no frame is ready. Evidence: MCP disassembly order is volume `0x00432E40..0x00432E7A`, pause/resume `0x00432E80..0x00432F31`, then initial wait `0x00432F36..0x00432F55`.
- Do not equate `_BinkCopyToBuffer` success with the update return value. Evidence: MCP branch `0x00432FA4 -> 0x00432FE9` skips copy but still reaches `_BinkNextFrame` and return-one epilogue.

## 7. Remaining Uncertainty

- Exact Bink SDK internals for `_BinkWait`, `_BinkPause`, `_BinkDoFrame`, and `_BinkNextFrame` remain external to `gamemd.exe`; this audit proves call order and branch use, not SDK implementation.
- Force flag `+0x2D` behavior inside the loop is proven, but complete standard-YR caller liveness for the setter was outside scope.
- Exact DirectDraw surface vtable method names for `+0x5C/+0x60/+0x74/+0x80` are role-inferred from call placement; full surface ABI belongs to the surface-format slot.
- Loop finished/restart details are intentionally external and owned by the `+0x14/+0x1C` slots.

## 8. Stale-Doc Replacement Wording

`C:/Users/enok/Documents/ra2-rust-game/docs/research/traces/MAIN_MENU_RA2TS_BACKGROUND_MOVIE_TRACE.md`

Replace:

> Catch-up cap | Bink internal (loops while `BinkWait==0`) | max 4 frames per `step` call

With:

> Catch-up behavior | gamemd `FUN_00432E40` loops with no hard counter cap while post-`_BinkNextFrame` `_BinkWait(handle)==0`; current Rust caps `frames_due(..., 4)`, which is a Rust safety policy and a mechanism drift.

Replace:

> Under CPU pressure, our accumulator can advance 2-3 frames in one step call where gamemd would advance 1 per `BinkWait` poll.

With:

> Under CPU pressure, current Rust advances according to elapsed wall-clock with a fixed max-4 cap. gamemd advances according to repeated `_BinkWait` results inside each timer poll, so the number of frames per poll is Bink-SDK-gated rather than elapsed-accumulator-gated.

`C:/Users/enok/Documents/ra2-rust-game/docs/research/BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`

Replace:

> `FUN_00432e40` also calls `_BinkWait_4` and loops `while (_BinkWait_4 == 0)`

With:

> `FUN_00432E40` first returns `0` when initial `_BinkWait(handle) != 0` and force flag `+0x2D` is clear; after a decode/next-frame pass it loops back while the post-`_BinkNextFrame` `_BinkWait(handle) == 0`, then returns `1`.

## 9. Sources

- Ghidra MCP `list_instances`: connected to `gamemd.exe`, image base `0x00400000`.
- Ghidra MCP `decompile_function`: `0x00432E40`, `0x00433040`, `0x005C0580`, `0x006153E0`, `0x005C07D0`.
- Ghidra MCP `disassemble_function`: `0x00432E40`, `0x00433040`, `0x005C0580`, `0x006153E0`, `0x005C07D0`.
- Ghidra MCP `read_memory`: `0x007EE154` vtable bytes.
- Ghidra MCP `get_bulk_xrefs`: `0x00432E40`, `0x00433040`, `0x005C0580`, `0x007EE154`.
- Rust source scan: `C:/Users/enok/Documents/ra2-rust-game/src/render/bink_movie.rs`.

