# Bink Loop/End Vtable +0x14/+0x1C - Ghidra Research Report

**Address(es):** `0x006153E0`, `0x007EE154`, `0x005C0570`, `0x00432C50`, `0x005C05D0`, `0x00432BD0`
**Investigation Mode:** exhaustive-slice, downgraded to partial because no Ghidra MCP instance was available for fresh read-only verification.
**Claimed Scope:** WM_TIMER `0x65` owner-draw static `0x71A` end check and loop restart handoff, based on existing verified Ghidra reports plus current Rust source.
**Non-Scope:** Bink open/init, archive lookup, per-frame copy internals, audio, general VQA movie handling, and retail runtime capture of the first visible loop frame.
**Confidence:** Medium overall. High for facts directly copied from prior verified Ghidra reports; low for any claim requiring new disassembly in this slot.
**Active in YR:** Yes - standard main-menu dialog `0xE2` sends `0x4E3` and `0x4E4` to child static `0x71A`, and `OwnerDraw_Static_006153E0` handles timer `0x65` for that live movie path.

## 0. Working Notes

- Target question: Verify the finished predicate and restart path used by the WM_TIMER `0x65` owner-draw static handler: vtable `+0x14` end check, loop flag from `0x4E3`, vtable `+0x1C` restart call, exact `BinkGoto` arguments, and Rust loop restart implications.
- Non-goals: Do not re-investigate Bink open/init, BIK-before-VQA lookup, full update/copy pipeline, audio tracks, pixel format, or Rust implementation patches.
- Evidence needed to mark COMPLETE: fresh read-only Ghidra confirmation of `0x007EE154` slot values, decompile/assembly for `0x00432C50` and `0x00432BD0`, xref/caller proof from `0x006153E0` timer branch, and current Rust line-level comparison.
- Stop conditions: Stop after the scoped slots and Rust handoff are classified; if Ghidra is unavailable, write a partial report that distinguishes prior verified evidence from unresolved fresh-verification gaps.

## 1. Overview

The main-menu RA2TS movie static uses a timer-driven generic movie handle. On each `WM_TIMER` id `0x65`, the owner-draw proc updates the movie, invalidates the static when a frame advanced, then calls the movie vtable `+0x14` finished/wrap predicate. If that predicate is true and the stored loop flag is nonzero, the handler calls vtable `+0x1C` with frame argument `1`; for Bink this resolves to `_BinkGoto(handle, 1, 1)` and clears the Bink object last-frame marker.

This slot could not connect to Ghidra (`list_instances` returned no running instances), so it did not add new binary evidence. It reconciles prior verified reports and names the exact remaining live checks needed to upgrade this report to COMPLETE.

## 2. Class Layout / Key Offsets

### Owner-draw static record

| Offset / index | Meaning | Active in YR | Evidence |
|---|---|---|---|
| `piVar11[0x16]` / byte `+0x58` | Generic movie handle pointer for kind-4 movie statics. | Yes - child `0x71A` on dialog `0xE2`. | `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` section 2 and 10.4. |
| `piVar11[0x17]` / byte `+0x5C` | Movie loop flag written by custom message `0x4E3`. | Yes - main menu sends `0x4E3` with `wParam=1`. | `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` sections 2 and 4; `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` sections 5.1 and 8. |

### Bink object fields reported by prior Ghidra docs

| Offset | Meaning | Active in YR | Evidence |
|---|---|---|---|
| Bink handle `+0x08` | Total frame count used by end/wrap test. | Yes - Bink movie handle for `ra2ts_s/l`. | `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 5. |
| Bink handle `+0x0C` | Current frame value used by end/wrap test. | Yes - same path. | `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 5. |
| Bink object `+0x30` | Last-frame marker recorded before `_BinkNextFrame`; cleared on goto. | Yes - same path. | `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 5. |

## 3. Core Logic

### Timer branch order

Prior verified reports agree on this live timer order:

1. `OwnerDraw_Static_006153E0` handles `WM_TIMER` `0x65`.
2. If movie handle is null, it returns.
3. It calls movie vtable `+0x04` update.
4. If update returns nonzero, it calls `InvalidateRect(hwnd, NULL, erase=0)`.
5. It calls movie vtable `+0x14` finished/wrap predicate.
6. If not finished, it returns.
7. If finished and loop flag `piVar11[0x17]` is nonzero, it calls movie vtable `+0x1C` with argument `1`, logs `"Looping movie"`, and returns.
8. If finished and not looping, it destroys the movie handle, clears `piVar11[0x16]`, kills timer `0x65`, and destroys secondary handle `piVar11[0x18]` if present.

Active in YR: Yes. Evidence: `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` section 5.1, `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 4.

### Finished predicate

The currently best-supported predicate is from the newer RA2TS playback report:

```text
finished = current_frame >= total_frames OR current_frame < last_frame_marker
```

where current/total are read from the Bink handle and last-frame marker is the object `+0x30` value written by the update loop before `_BinkNextFrame`.

Active in YR: Yes for main-menu Bink playback. Evidence: `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 3 maps vtable `+0x14` to thunk `0x005C0570` -> `0x00432C50`; section 5 states the handle `+0x0C`, handle `+0x08`, and object `+0x30` comparison. Fresh assembly was not available in this slot.

### Restart path

The currently best-supported restart path is:

```text
vtable+0x1C(frame=1)
  -> Bink goto thunk 0x005C05D0
  -> 0x00432BD0
  -> _BinkGoto(handle, frame, 1)
  -> object+0x30 = 0
```

For the main-menu loop, the owner-draw timer passes frame argument `1`, so Bink receives `_BinkGoto(handle, 1, 1)`.

Active in YR: Yes. Evidence: `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` sections 3, 4, and 5; `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` section 5.1. Fresh argument-order assembly was not available in this slot.

## 4. INI Keys

No INI key controls this loop/end branch. The path is driven by shell code sending custom messages to the owner-draw static and by Bink handle state.

| Key | Status | Active in YR | Evidence |
|---|---|---|---|
| None | No loop/end INI surface identified for static `0x71A`. | Yes - absence applies to standard main-menu path. | Prior owner-draw and RA2TS playback docs; no relevant INI surface in current Rust/doc scan. |

## 5. Integration Points

| Integration | Role | Active in YR | Evidence |
|---|---|---|---|
| `FUN_00531CC0` / `FUN_0052B9B0` | Main menu setup sends `0x4E3` before `0x4E4`; `0x4E3` passes loop flag `1`. | Yes - standard main menu open path. | `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` sections 2 and 4; `MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md` also records the sequence. |
| `OwnerDraw_Static_006153E0 @ 0x006153E0` | Owns custom messages and timer `0x65`; checks finished and restarts/destroys movie. | Yes - subclassed live static proc. | `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` sections 1, 5.1, 8, and 10.4. |
| `vtable__BinkMovieHandle @ 0x007EE154` | Concrete Bink movie vtable installed for `.bik` handles. | Yes - `ra2ts_s/l` are BIK files. | `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` sections 3 and Sources. |
| `0x00432E40` update loop | Writes object `+0x30` before `_BinkNextFrame`; finished predicate depends on this marker. | Yes. | `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 5. |

## 6. Current Rust Implementation Status

Current Rust source scanned:

| Surface | Current behavior | Delta vs best-known gamemd behavior |
|---|---|---|
| `src/render/bink_movie.rs` `BinkMovieSurface::step` | Uses `current_frame >= frame_count()` as the loop/end trigger. On loop, calls `restart_at_original_frame_one()`, marks changed, breaks, then uploads RGBA. | Missing the binary finished predicate's wrap leg (`current_frame < last_frame_marker`) and does not model Bink handle current-frame values directly. |
| `src/render/bink_movie.rs` `restart_at_original_frame_one` | Flushes decoder, decodes `video_packet(0)`, sets RGBA, sets `current_frame = 1`, clears accumulator. | It does not model `_BinkGoto(handle, 1, 1)` directly, does not clear a last-frame marker equivalent, and currently assumes Bink frame argument `1` maps to parser packet index `0`. |
| `src/assets/bink_file.rs` `BinkFile::video_packet` | Takes zero-based packet index `i` into `frame_index`. | Correct for parser indexing, but not itself evidence for Bink SDK's external frame numbering. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Ghidra MCP availability | deferred | `list_instances` returned no running instances in this slot. | Re-run with Ghidra instance connected. |
| `OwnerDraw_Static_006153E0` timer `0x65` branch | verified-from-prior-doc | `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` section 5.1; `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 4. | Fresh decompile/assembly spot-check. |
| Loop flag from `0x4E3` at owner-draw record `+0x5C` | verified-from-prior-doc | `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` sections 2 and 8. | Fresh decompile spot-check. |
| Vtable `+0x14` slot identity | touched-not-exhausted | Newer report says `0x005C0570 -> 0x00432C50`; older 0x4F0 cadence report table says `0x005C0550`, creating conflict. | Fresh `read_memory(0x007EE154)` and thunk disassembly. |
| Finished predicate internals | verified-from-prior-doc | `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 5. | Fresh decompile plus assembly for `0x00432C50`. |
| Vtable `+0x1C` slot identity | touched-not-exhausted | Newer report says `0x005C05D0 -> 0x00432BD0`; older 0x4F0 cadence report table says `0x005C0570`, creating conflict. | Fresh `read_memory(0x007EE154)` and thunk disassembly. |
| Restart arguments | verified-from-prior-doc | `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` sections 3 and 5; `RA2TS_BINK_AUDIO_ENABLE_GHIDRA_REPORT.md` import table. | Fresh argument-order assembly for `_BinkGoto(handle, frame, 1)`. |
| Rust `BinkMovieSurface::step` | verified-source | `src/render/bink_movie.rs` source scan; Codegraph context found `BinkFile`, `video_packet`, and `FrameIndexEntry`. | Future implementation patch and focused tests. |
| Rust parser indexing | verified-source | `src/assets/bink_file.rs` `video_packet(i)` indexes `frame_index.get(i)`. | Runtime/video fixture proving Bink frame-1 external mapping. |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is the target path active in standard YR?` -> Yes; dialog `0xE2` child `0x71A` receives `0x4E3`/`0x4E4` and timer `0x65` runs the movie path. (evidence: `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` sections 5.1, 8, 10.4)
- `[RESOLVED] OQ-2 - Where is the loop flag stored?` -> Owner-draw record `piVar11[0x17]` / byte `+0x5C`, written by message `0x4E3`. (evidence: `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` section 2 and 8)
- `[RESOLVED] OQ-3 - Does main menu set looping?` -> Yes, prior reports record main menu passes `wParam=1` via `0x4E3` before loading the movie. (evidence: `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 4)
- `[RESOLVED] OQ-4 - What does the finished predicate check according to current best evidence?` -> `current_frame >= total_frames OR current_frame < last_frame_marker`. (evidence: `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 5)
- `[RESOLVED] OQ-5 - What does the loop restart call pass according to current best evidence?` -> `_BinkGoto(handle, 1, 1)` for the main-menu loop. (evidence: `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` sections 3 and 5)
- `[RESOLVED] OQ-6 - Does Rust currently use zero-based parser packets?` -> Yes, `BinkFile::video_packet(i)` indexes `frame_index.get(i)`, and `restart_at_original_frame_one` passes `0`. (evidence: `src/assets/bink_file.rs`; `src/render/bink_movie.rs`)
- `[DEFERRED] OQ-7 - Which exact function pointers are present at vtable `+0x14` and `+0x1C` today?` (category: `needs-runtime-debugger`; reason: no running Ghidra instance, and prior docs conflict on the older table; next-step-if-pursued: read `0x007EE154` and disassemble the two thunks)
- `[DEFERRED] OQ-8 - Does Bink SDK frame argument `1` always correspond to Rust packet index `0` for these RA2TS files?` (category: `needs-runtime-debugger`; reason: prior docs establish the binary passes `1`, but not the decoded-frame equivalence; next-step-if-pursued: trace/display first frame after loop or build a BinkGoto-vs-decoder fixture)
- `[DEFERRED] OQ-9 - Does `_BinkGoto(handle, 1, 1)` decode/copy a frame immediately or only reposition timing state?` (category: `needs-runtime-debugger`; reason: prior docs prove the call and `+0x30` clear, but not immediate visible copy side effects; next-step-if-pursued: trace calls following `0x00432BD0` and observe next `0x00432E40`/`0x4F0` sequence)

## 9. Visual/UI Composition Ledger

This report does not re-open full visual composition. The scoped visual consequence is loop-point frame selection.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `OwnerDraw_Static_006153E0` timer `0x65` | Movie handle non-null; update returns changed; then finished check. | `ra2ts_s/l.bik`, current Bink frame | Static `0x71A` rect | Bink copy path, details out-of-scope | yes | decode/invalidate driver |
| 2 | `vtable+0x1C` restart | Finished true and loop flag `+0x5C != 0`. | Bink external frame `1` | same movie | Bink SDK timing state | yes | loop restart |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `ra2ts_s.bik` | yes at width `640` | yes | yes | yes | no | no | no | no | Parent-supplied settled fact; `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`. |
| `ra2ts_l.bik` | yes at widths other than `640` | yes | yes | yes | no | no | no | no | Parent-supplied settled fact; `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`. |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Finished means current Bink frame is at/after total frames OR has wrapped below the last recorded frame marker. | Prior verified Ghidra: `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` section 5 (`0x00432C50`). | Rust only checks `current_frame >= frame_count()`. | `src/render/bink_movie.rs` `BinkMovieSurface::step`. | Preserve a last-frame marker equivalent or otherwise prove wrap detection is impossible under Rust's decoder model. | Test proposal: `bink_movie_loop_end_detects_wrap_below_last_marker`. | Do not model the end check as total-frame-only unless a later binary/runtime proof shows Rust cannot observe wrap. |
| Looping main-menu Bink restart calls `_BinkGoto(handle, 1, 1)` and clears object `+0x30`. | Prior verified Ghidra: `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` sections 3 and 5 (`0x00432BD0`). | Rust flushes decoder, decodes `video_packet(0)`, sets `current_frame=1`, and clears accumulator. | `src/render/bink_movie.rs` `restart_at_original_frame_one`; possible decoder seek/reset helper. | Future Rust should explicitly encode the external Bink frame-1 semantics and reset the last-frame marker/timing state to match `wait=1`, not merely decode parser packet 0 by assumption. | Test proposal: `bink_movie_restart_frame_one_maps_bink_goto_one_to_expected_packet`. | Do not silently equate Bink external frame `1` with parser packet index `0` without a fixture or runtime proof. |
| Timer branch updates/invalidate first, then checks finished and restarts/destroys. | Prior verified Ghidra: `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md` section 5.1. | Rust checks end inside frame-advance loop and uploads after restart; no separate owner-draw invalidation model. | `src/render/bink_movie.rs` `step`; `src/app_main_menu_shell_render.rs` caller cadence. | Preserve observable ordering: final ready frame may be copied/uploaded before the loop decision; restart should not skip the last displayed frame. | Test proposal: `bink_movie_step_uploads_final_ready_frame_before_loop_restart`. | Do not restart before consuming/displaying the last frame unless a later direct binary check contradicts the prior timer-order docs. |

### Negative Facts / Do Not Do

- Do not treat the older `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` vtable table as authoritative for `+0x14/+0x1C`; it conflicts with the newer RA2TS playback report and should be rechecked before use. Evidence: older doc section 3 says `+0x14 = 0x005C0550`, `+0x1C = 0x005C0570`; newer doc section 3 says `+0x14 = 0x005C0570 -> 0x00432C50`, `+0x1C = 0x005C05D0 -> 0x00432BD0`.
- Do not implement loop restart as parser-index semantics alone. Evidence: gamemd-facing call is `_BinkGoto(handle, 1, 1)` per `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`; Rust parser `video_packet(i)` is zero-based.
- Do not ignore the wrap leg of the finished predicate. Evidence: prior report records `current_frame < last_frame_marker` as part of `0x00432C50`.
- Do not advance/destroy the movie based only on a render-loop frame counter. Evidence: owner-draw timer uses Bink update and Bink finished predicate, not Rust's wall-clock accumulator.
- Do not edit the old plan wording to assert the `BinkGoto(1) -> packet 0` mapping as proven. It remains unresolved until runtime/fixture proof.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/plans/2026-05-17-initial-main-menu-dialog-0xe2-plan.md`
  - Replace: "`BinkGoto(1)` is treated as Rust decoder index `0`; a loop unit test verifies this intended mapping in the Rust abstraction."
  - With: "gamemd loops with `_BinkGoto(handle, frame=1, wait=1)` and clears the Bink last-frame marker. Rust currently decodes parser packet index `0` on restart, but the equivalence between Bink's external frame `1` and Rust's zero-based packet `0` remains unverified and needs a targeted loop-frame fixture or runtime capture."
- `C:/Users/enok/Documents/ra2-rust-game/docs/plans/2026-05-17-initial-main-menu-dialog-0xe2-design.md`
  - Replace: "`BinkGoto(1)` in gamemd maps to Rust decoder frame index `0`; this should be verified with a targeted playback test during implementation."
  - With: "gamemd loops with `_BinkGoto(handle, frame=1, wait=1)`. Treat mapping to Rust decoder packet index `0` as an open parity question until a targeted playback test or runtime capture proves it for RA2TS BIK files."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`
  - Replace section 3 rows for `+0x14/+0x1C` with: "`+0x14` and `+0x1C` were later re-mapped by `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` as `+0x14 = 0x005C0570 -> 0x00432C50` and `+0x1C = 0x005C05D0 -> 0x00432BD0`; re-read `0x007EE154` before using this older slot table."

## Sources

- Ghidra availability check in this slot:
  - `list_instances` returned no running instances, so no fresh decompile/read-memory calls could be made.
- Prior verified Ghidra reports referenced:
  - `C:/Users/enok/Documents/ra2-rust-game/docs/research/MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game/docs/research/OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game/docs/research/BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game/docs/research/traces/MAIN_MENU_RA2TS_BACKGROUND_MOVIE_TRACE.md`
  - `C:/Users/enok/Documents/ra2-rust-game/docs/research/RA2TS_BINK_AUDIO_ENABLE_GHIDRA_REPORT.md`
- Rust source inspected:
  - `C:/Users/enok/Documents/ra2-rust-game/src/render/bink_movie.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/assets/bink_file.rs`
- Research index:
  - `python tools/research_index/brief.py "Bink loop end vtable 0x14 0x1C BinkGoto" --limit 8`
