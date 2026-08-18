# Bink Restart / BinkGoto Helper 0x00432BD0 Ghidra Report

**Date:** 2026-05-27  
**Target:** `0x00432BD0`, Bink vtable `0x007EE154 + 0x1C`, owner-draw timer loop caller  
**Status:** COMPLETE  
**Evidence mode:** Fresh read-only Ghidra MCP against `gamemd.exe`; no Ghidra mutations.

## Working Notes

- Target question: What exactly does Bink restart helper `0x00432BD0` pass to `_BinkGoto`, what state does it clear, and how does the standard owner-draw timer call it with frame `1`?
- Non-goals: Do not re-investigate finished predicate internals, update-loop cadence, surface copy format, BIK/VQA resolution, Bink SDK internals, or VQA playback.
- Evidence needed to mark COMPLETE: fresh MCP evidence for helper decompile plus assembly, `_BinkGoto` import target, vtable `+0x1C` thunk mapping, `0x65` timer owner-draw caller with pushed frame `1`, and liveness through Bink handle construction.
- Stop conditions: stop after proving the helper/caller path or if Ghidra MCP cannot provide fresh decompile/disassembly/xrefs; do not infer Bink SDK frame-to-packet mapping from gamemd bytes alone.

## Summary

The Bink restart helper is a tiny `thiscall` wrapper. It reads the caller's frame argument from `[esp+4]`, calls `_BinkGoto@12` with `(object+0x04 Bink handle, caller_frame, 1)`, then clears `object+0x30` to zero and returns `void` with `ret 4`.

For the owner-draw movie timer path, `WM_TIMER` id `0x65` first updates the movie through vtable `+0x04`, then checks finished state through vtable `+0x14`. If finished and the owner-draw loop flag at record `+0x5C` is nonzero, it calls movie vtable `+0x1C` with a single argument `1`. For Bink movie handles, vtable `0x007EE154 + 0x1C` points to thunk `0x005C05D0`, which forwards that `1` to `0x00432BD0`.

Binary evidence proves gamemd asks BINKW32 to go to external frame argument `1` with wait flag `1`. Binary evidence does not prove that Bink external frame `1` is equivalent to Rust `BinkFile::video_packet(0)`, because that mapping lives inside the Bink SDK/container decode semantics rather than in gamemd wrapper code.

## Verified Findings

### 1. Helper argument order is `_BinkGoto(handle, frame, 1)`

**Active in YR:** Yes.

Fresh MCP decompile at `0x00432BD0`:

- `FUN_00432bd0(int this, undefined4 frame)` calls `_BinkGoto_12(*(this + 4), frame, 1)`.
- It then stores zero to `*(this + 0x30)`.

Fresh MCP assembly at `0x00432BD0..0x00432BEC` confirms the stack order:

- `00432BD0`: load frame argument from `[ESP+0x4]`.
- `00432BD7`: `PUSH 0x1`.
- `00432BD9`: `PUSH EAX` where `EAX` is the caller frame.
- `00432BDA`: load `ECX = [ESI+0x4]`.
- `00432BDD`: push handle.
- `00432BDE`: call through `[0x007E15AC]`.
- `00432BE4`: store `0` to `[ESI+0x30]`.
- `00432BEC`: `RET 0x4`.

The import indirection at `0x007E15AC` contains `0x00410BEC`; `list_external_locations` identifies `0x00410BEC` as `BINKW32.DLL::_BinkGoto@12`.

### 2. The helper clears object offset `+0x30` after `_BinkGoto`

**Active in YR:** Yes.

Evidence is the same helper body: the clear occurs after the external call, not before it. The offset write is `MOV dword ptr [ESI + 0x30], 0x0` at `0x00432BE4`.

This proves the cleared field is a Bink-object wrapper field, not a Rust-style elapsed accumulator. Slot 2 owns the predicate meaning of the field; this slot proves the restart write ordering and value.

### 3. Bink vtable `+0x1C` maps to thunk `0x005C05D0`, which forwards the frame argument

**Active in YR:** Yes for Bink movie handles.

Fresh MCP `read_memory(0x007EE154, 64)` shows the Bink vtable sequence. At `0x007EE154 + 0x1C = 0x007EE170`, the dword is `0x005C05D0`.

Fresh MCP xrefs:

- `get_xrefs_to(0x005C05D0)` returns data xref from `0x007EE170`.
- `get_xrefs_to(0x00432BD0)` returns call xref from `0x005C05D8`.

Thunk bytes at `0x005C05D0` decode as:

- load caller argument from `[ESP+4]`;
- load underlying Bink object from movie handle `+0x10`;
- push the same caller argument;
- call `0x00432BD0`;
- `ret 4`.

Thus the vtable slot accepts one frame argument and forwards it unchanged to the helper.

### 4. Standard owner-draw timer calls vtable `+0x1C` with frame `1`

**Active in YR:** Yes.

Fresh MCP decompile of `OwnerDraw_Static_006153E0` shows the `WM_TIMER` branch for timer id `0x65`:

- movie object is read from owner-draw record `+0x58` (`piVar11[0x16]`);
- vtable `+0x04` update is called;
- vtable `+0x14` finished predicate is called;
- if finished and loop flag `piVar11[0x17]` is nonzero, vtable `+0x1C` is called with argument `1`.

Fresh MCP assembly pinpoints the path:

- `0x00615B80..0x00615B93`: branch for `WM_TIMER` id `0x65`, require movie handle at `[ESI+0x58]`.
- `0x00615B99..0x00615B9E`: call vtable `+0x04`.
- `0x00615BB2..0x00615BBC`: call vtable `+0x14`; return if false.
- `0x00615BC2..0x00615BC5`: check loop flag `[ESI+0x5C]`.
- `0x00615BC7..0x00615BD0`: load movie handle, `PUSH 0x1`, call `[vtable + 0x1C]`.
- `0x00615BD3..0x00615BDD`: register "Looping movie" diagnostic string, then return `0`.

This is the active main-menu owner-draw static path previously reached by static child `0x71A` and timer `0x65`.

### 5. Bink movie handles install vtable `0x007EE154`

**Active in YR:** Yes for resolved `.BIK` movie handles.

Fresh MCP decompile/disassembly of `0x005C07D0` shows the BIK branch allocating a 0x14-byte generic movie handle and writing `0x007EE154` at `0x005C0897`. The same branch stores the Bink object pointer at handle `+0x10` and copies Bink width/height from the Bink object handle fields into generic handle `+0x08/+0x0C`.

This proves that the owner-draw timer's vtable `+0x1C` call reaches the Bink thunk for BIK-backed movies, not the VQA vtable.

## Implementation Handoff

| Verified behavior | Evidence | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|---|
| Loop restart calls `_BinkGoto(handle, frame, 1)` with owner-draw frame argument `1`, then clears object `+0x30`. | `0x00432BD0..0x00432BEC`, import `[0x007E15AC] -> _BinkGoto@12`, caller `0x00615BC7..0x00615BD0`. | Rust `restart_at_original_frame_one` flushes decoder, decodes `video_packet(0)`, sets `current_frame = 1`, clears `accumulator_secs`; it does not model Bink `wait=1` or the `+0x30` marker explicitly. | `src/render/bink_movie.rs::restart_at_original_frame_one`; any future Bink SDK/seek abstraction. | Looping movie restart should encode "external Bink frame 1 + wait flag 1 + marker clear" as the parity behavior, with packet-0 decode only behind a verified adapter. | `bink_restart_uses_external_frame_one_wait_one_and_clears_marker` | Treating Rust packet index `0` as the mechanism can hide a first-frame/loop-frame drift. |
| Owner-draw timer calls update, then finished predicate, then restart with frame `1` only when loop flag is nonzero. | `0x00615B99..0x00615BD0`; decompile of `OwnerDraw_Static_006153E0`. | Rust loop behavior is embedded in `BinkMovieSurface::step`, so restart can happen as part of stepping rather than as a separate owner-draw decision after finished predicate. | `src/render/bink_movie.rs::step`; main-menu movie caller. | If Rust keeps a self-contained movie surface, tests must preserve the same visible ordering: update result first, finished test second, loop restart third. | `bink_ownerdraw_loop_restarts_after_finished_check_with_frame_one` | Restarting before displaying or invalidating the final changed frame would drift from owner-draw timer order. |
| Binary proves Bink external frame `1`; it does not prove Rust `video_packet(0)` equivalence. | gamemd only calls external `_BinkGoto@12`; Rust search shows `video_packet(0)` at `src/render/bink_movie.rs:137`. | Current Rust assumes the equivalence by directly decoding packet index `0`. | `src/assets/bink_file.rs::video_packet`; `src/render/bink_movie.rs::restart_at_original_frame_one`. | Add fixture/runtime proof before documenting packet-0 restart as exact; otherwise name it as an adapter assumption. | `bink_goto_frame_one_matches_first_decoded_packet_for_ra2ts_fixture` | Without a Bink SDK/runtime comparison, a 1-frame loop discontinuity can be missed. |

## Negative Facts / Do Not Do

- Do not implement restart as "jump to frame 0" in gamemd terms. Evidence: owner-draw pushes `1` at `0x00615BCA`, vtable thunk forwards it, helper passes it to `_BinkGoto`.
- Do not treat `_BinkGoto` wait flag as caller-controlled in this path. Evidence: helper hardcodes `PUSH 0x1` at `0x00432BD7`; the owner-draw caller supplies only the frame argument.
- Do not clear the restart marker before calling `_BinkGoto`. Evidence: the clear at `0x00432BE4` follows the external call at `0x00432BDE`.
- Do not claim binary evidence proves `BinkGoto(1)` equals Rust `video_packet(0)`. Evidence: gamemd passes an external Bink API frame number and never indexes the Rust parser's BIK frame table.
- Do not route the owner-draw loop call through the VQA vtable for BIK files. Evidence: BIK construction writes vtable `0x007EE154` at `0x005C0897`; `+0x1C` at `0x007EE170` points to Bink thunk `0x005C05D0`.

## Remaining Uncertainty

- Exact Bink SDK semantics after `_BinkGoto(handle, 1, 1)` remain external to gamemd: whether it immediately decodes/seeks timing state and which compressed packet it will expose next must be proven by runtime capture, SDK documentation, or a fixture comparison.
- Whether `object+0x30` should be represented in Rust as a last-frame marker, current-frame marker, or other Bink wrapper state is owned by the finished-predicate slot; this slot only proves restart clears it after `_BinkGoto`.

## Stale-Doc Replacement Wording

### `docs/plans/2026-05-17-initial-main-menu-dialog-0xe2-plan.md`

Replace claims that say "`BinkGoto(1)` is treated as Rust decoder index `0`" or that implementation "matches `BinkGoto(1)` assumption" with:

> gamemd loops by calling `_BinkGoto(handle, frame=1, wait=1)` through the Bink movie vtable and then clearing the Bink wrapper marker at object `+0x30`. Rust currently restarts by decoding `BinkFile::video_packet(0)`, but the equivalence between Bink's external frame `1` and Rust's zero-based packet `0` is not proven by gamemd binary evidence and needs a targeted runtime/fixture proof.

### `docs/plans/2026-05-17-initial-main-menu-dialog-0xe2-design.md`

Replace:

> `BinkGoto(1)` in gamemd maps to Rust decoder frame index `0`; this should be verified with a targeted playback test during implementation.

With:

> gamemd calls `_BinkGoto(handle, frame=1, wait=1)` and clears the Bink wrapper marker at object `+0x30`. Treat mapping to Rust decoder packet index `0` as an open parity question until a targeted playback test or runtime capture proves it for RA2TS BIK files.

### `docs/research/traces/MAIN_MENU_RA2TS_BACKGROUND_MOVIE_TRACE.md`

Replace any wording that says Bink frame `1` is or is not the same as BIK frame/packet `0` as a settled fact with:

> The binary-proven restart mechanism is `_BinkGoto(handle, frame=1, wait=1)`, followed by clearing the wrapper marker at object `+0x30`. The mapping from that Bink API frame to Rust's `BinkFile::video_packet(0)` remains a runtime/fixture question.

## Open Questions

- `[DEFERRED] OQ-1 - Does BINKW32 `_BinkGoto(handle, 1, 1)` make the next visible copied frame byte-identical to Rust `video_packet(0)` for `ra2ts_l.bik` and `ra2ts_s.bik`?` Category: needs-runtime-fixture.
- `[DEFERRED] OQ-2 - Does `_BinkGoto(handle, 1, 1)` perform decode/copy side effects immediately or only reposition Bink timing state until the next `_BinkDoFrame`/`_BinkCopyToBuffer`?` Category: needs-runtime-debugger or SDK documentation.
