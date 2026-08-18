# Bink vtable `0x007EE154` Slot Map - Ghidra Research Report

**Address(es):** `0x007EE154`, `0x005C0580`, `0x005C0570`, `0x005C05A0`, `0x005C05D0`, `0x005C05F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** direct slot map for Bink movie handle vtable byte offsets `+0x04`, `+0x14`, `+0x18`, `+0x1C`, and `+0x28`, with tiny thunk identity checks only.  
**Non-Scope:** internals of `0x00432E40`, `0x00432C50`, `0x00432BD0`, `0x00432AB0`, and `0x00433060`; VQA playback; Bink SDK internals; DirectDraw color-format proof.  
**Confidence:** High for the slot table; Medium for role labels that rely on the target callees' existing names/decompile shape.  
**Active in YR:** Yes. The `OwnerDraw_Static_006153E0` movie static calls these virtual slots on the active main-menu Bink-backed handle.

## 0. Investigation Gate

**Target question:** What are the fresh Ghidra MCP-verified vtable entries at `0x007EE154 + {0x04,0x14,0x18,0x1C,0x28}`, and which prior report is stale?

**Non-goals:** Do not audit full Bink update, finished, restart, clip, or explicit-draw bodies. Do not implement Rust. Do not investigate VQA.

**Evidence needed to mark COMPLETE:**

- Direct Ghidra MCP memory read of `0x007EE154` covering all requested offsets.
- Thunk identity check for each requested slot from the pointed address to the concrete target.
- Active caller proof that the owner-draw timer/paint path uses these slots in YR.
- Stale-doc reconciliation for the conflicting old/new tables.

**Stop conditions:** Stop after the five requested slots are mapped and the stale table is identified; defer target function internals to the other focused retry slots.

## 1. Overview

The Bink movie handle vtable at `0x007EE154` is the active virtual table installed for `.bik` movie handles. Fresh MCP memory read proves the newer `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` slot map is correct for the contested loop/end slots: `+0x14` is `0x005C0570 -> 0x00432C50`, and `+0x1C` is `0x005C05D0 -> 0x00432BD0`.

The older `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` table is stale for `+0x14/+0x1C`: it lists `+0x14 = 0x005C0550` and `+0x1C = 0x005C0570`, but fresh memory reads show those addresses belong to `+0x10` and `+0x14` respectively.

## 2. Slot Table

Fresh Ghidra MCP `read_memory(program="gamemd.exe", address="0x007EE154", length=64)` returned these little-endian dwords:

| Vtable byte offset | Dword target | Thunk identity | Role label | Active in YR? | Evidence |
|---:|---:|---|---|---|---|
| `+0x04` | `0x005C0580` | `MOV ECX,[ECX+0x10]; JMP 0x00433040`; `0x00433040` calls `0x00432E40(surface,x,y)` | timer update / changed-frame poll | Yes | vtable bytes at `0x007EE158`; thunk assembly `0x005C0580..0x005C0587`; owner-draw timer call `0x00615B8E..0x00615B9B` |
| `+0x14` | `0x005C0570` | `MOV ECX,[ECX+0x10]; JMP 0x00432C50` | finished/end-or-wrap predicate | Yes | vtable bytes at `0x007EE168`; thunk assembly `0x005C0570..0x005C0577`; owner-draw timer call `0x00615BB2..0x00615BBA` |
| `+0x18` | `0x005C05A0` | loads stack args, `MOV ECX,[ECX+0x10]`, calls `0x00432AB0`, returns `8` | target/clip setup | Yes | vtable bytes at `0x007EE16C`; Ghidra disassembly `0x005C05A0..0x005C05B2`; owner-draw setup calls `+0x18` at `0x00616136` and `0x006161FB` |
| `+0x1C` | `0x005C05D0` | loads one stack arg, `MOV ECX,[ECX+0x10]`, calls `0x00432BD0`, returns `4` | restart/goto wrapper | Yes | vtable bytes at `0x007EE170`; thunk assembly `0x005C05D0..0x005C05DD`; owner-draw loop call `0x00615BC7..0x00615BD0` |
| `+0x28` | `0x005C05F0` | `MOV ECX,[ECX+0x10]; JMP 0x00433060` | explicit draw/copy | Yes | vtable bytes at `0x007EE17C`; Ghidra disassembly `0x005C05F0..0x005C05F7`; owner-draw `0x4F0` call `0x00616270..0x00616275` |

Additional adjacent values from the same read explain the stale-doc shift:

| Vtable byte offset | Dword target | Tiny identity | Why it matters |
|---:|---:|---|---|
| `+0x10` | `0x005C0550` | stores `ECX` to `DAT_00ABF3F8`, calls `0x00432C70`, clears the global, returns | This is not `+0x14`; the old table put this address in the finished-predicate row. |
| `+0x0C` | `0x005C0540` | calls `0x00432C30(arg)` | Confirms the old table was not uniformly shifted, only the contested rows were wrong. |

## 3. Core Logic

This report only maps slots; it does not claim full callee semantics. The relevant virtual dispatch order in the active owner-draw static is:

1. `WM_TIMER`, id `0x65`, loads movie handle at owner-draw record `+0x58`, calls vtable `+0x04`, and invalidates the static only if `AL != 0`. Active in YR: Yes. Evidence: `OwnerDraw_Static_006153E0` decompile and assembly `0x00615B80..0x00615BAC`.
2. The same timer then calls vtable `+0x14`. If `AL == 0`, it returns. Active in YR: Yes. Evidence: assembly `0x00615BB2..0x00615BBC`.
3. If `+0x14` reports ended/wrapped and owner-draw loop flag at record `+0x5C` is nonzero, the timer calls vtable `+0x1C` with pushed argument `1`. Active in YR: Yes. Evidence: assembly `0x00615BC2..0x00615BD0`.
4. Custom message `0x4F0` calls vtable `+0x28` on the same movie handle. Active in YR: Yes. Evidence: `OwnerDraw_Static_006153E0` decompile and assembly `0x00616270..0x00616275`.
5. Setup calls vtable `+0x18` to configure target/clip/rect fields before moving/sizing the window. Active in YR: Yes. Evidence: assembly `0x00616136` and `0x006161FB`.

## 4. INI Keys

No INI keys are read by this vtable table slice. Movie asset choice and archive priority are outside this report.

## 5. Integration Points

`VQMovieHandle__Constructor` installs `&vtable__BinkMovieHandle` (`0x007EE154`) when the resolved movie suffix is `.bik`; Ghidra xrefs to `0x007EE154` include `0x005C0897` and constructors at `0x005C0607` / `0x005C0A37`. Active in YR: Yes, conditional on the resolved file being `.bik`; standard RA2TS main-menu movies are `.bik`.

The active consumer is `OwnerDraw_Static_006153E0`: message `0x4E4` constructs the movie handle and starts timer `0x65` at `0x22` ms; timer `0x65` uses `+0x04`, `+0x14`, and `+0x1C`; custom `0x4F0` uses `+0x28`.

## 6. Current Rust Implementation Status

Rust currently has a concrete Bink surface rather than a virtual movie-handle table:

| Rust surface | Current logic relevant to this slot map |
|---|---|
| `src/render/bink_movie.rs::BinkMovieSurface::step` | Accumulator-driven update loop, returns `FrameUploaded`/`Ended`, and loops by `restart_at_original_frame_one`; no distinct finished predicate slot equivalent. |
| `src/render/bink_movie.rs::restart_at_original_frame_one` | Decodes `video_packet(0)`, sets `current_frame = 1`, and clears accumulator; this is not yet proven equivalent to native vtable `+0x1C` / `_BinkGoto(handle, 1, 1)`. |
| `src/assets/bink_file.rs::video_packet` | Zero-based packet lookup surface used by Rust restart. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Direct `0x007EE154` requested slot table | verified | MCP `read_memory 0x007EE154 len 64` | none |
| `+0x04` thunk | verified | `0x005C0580..0x005C0587`; decompile of `BinkMovie_Update_005C0580` and `0x00433040` | full `0x00432E40` internals belong to slot 4 |
| `+0x14` thunk | verified | `0x005C0570..0x005C0577`; decompile of `0x00432C50` | full predicate audit belongs to slot 2 |
| `+0x18` thunk | verified | `0x005C05A0..0x005C05B2`; decompile of `0x00432AB0` touched only for identity | exact clipping edge cases out-of-scope |
| `+0x1C` thunk | verified | `0x005C05D0..0x005C05DD`; decompile of `0x00432BD0` | full restart/goto semantics belong to slot 3 |
| `+0x28` thunk | verified | `0x005C05F0..0x005C05F7`; decompile of `0x00433060` | full surface/color audit belongs to slot 5 |
| Old/new doc conflict | verified | fresh table plus prior doc lines in `BINK_0x4F0...` and `MAIN_MENU_RA2TS...` | update stale doc wording if/when published docs are patched |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-VT-001 - What is the target at vtable +0x04? -> 0x005C0580, thunk to 0x00433040.` (evidence: `0x007EE158`, `0x005C0580..0x005C0587`)
- `[RESOLVED] OQ-VT-002 - What is the target at vtable +0x14? -> 0x005C0570, thunk to 0x00432C50.` (evidence: `0x007EE168`, `0x005C0570..0x005C0577`)
- `[RESOLVED] OQ-VT-003 - What is the target at vtable +0x18? -> 0x005C05A0, thunk to 0x00432AB0.` (evidence: `0x007EE16C`, `0x005C05A0..0x005C05B2`)
- `[RESOLVED] OQ-VT-004 - What is the target at vtable +0x1C? -> 0x005C05D0, thunk to 0x00432BD0.` (evidence: `0x007EE170`, `0x005C05D0..0x005C05DD`)
- `[RESOLVED] OQ-VT-005 - What is the target at vtable +0x28? -> 0x005C05F0, thunk to 0x00433060.` (evidence: `0x007EE17C`, `0x005C05F0..0x005C05F7`)
- `[RESOLVED] OQ-VT-006 - Which prior report is stale on +0x14/+0x1C? -> BINK_0x4F0 table is stale; MAIN_MENU_RA2TS table matches fresh MCP evidence.` (evidence: fresh vtable read and the two cited docs)
- `[RESOLVED] OQ-VT-007 - Is this table active in standard YR main-menu movie playback? -> Yes, conditional on resolved movie suffix .bik; OwnerDraw static uses the handle's slots.` (evidence: `VQMovieHandle__Constructor` xref `0x005C0897`; owner-draw calls `0x00615B9B`, `0x00615BB7`, `0x00615BD0`, `0x00616275`)
- `[DEFERRED] OQ-VT-008 - What are exact internals of the update loop?` (category: out-of-scope; reason: slot 4 owns `0x00432E40`; next-step-if-pursued: read `BINK_UPDATE_LOOP_0X00432E40_FRESH_MCP_AUDIT_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-VT-009 - Does Bink frame 1 equal Rust packet 0?` (category: out-of-scope; reason: slot 3 owns `_BinkGoto`/SDK frame-index semantics; next-step-if-pursued: compare `0x00432BD0` with parser packet traces)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native loop/end dispatch uses distinct slots: update `+0x04`, finished `+0x14`, restart `+0x1C(1)`. | `0x00615B8E..0x00615BD0`; vtable entries `0x007EE158`, `0x007EE168`, `0x007EE170` | Rust currently combines step, end, and loop restart in `BinkMovieSurface::step`. | `src/render/bink_movie.rs::step`, `restart_at_original_frame_one` | Preserve the native ordering when parity mode is implemented: update first, invalidate if changed, then finished predicate, then loop restart with argument `1`. | Main-menu timer tick with final ready frame uploads/invalidates before loop restart decision. Proposed test: `bink_timer_dispatch_order_update_then_finished_then_restart`. | Do not use the old table that maps `+0x14` to `0x005C0550` or `+0x1C` to `0x005C0570`. |
| Native explicit draw dispatch is vtable `+0x28 -> 0x005C05F0 -> 0x00433060`. | `0x007EE17C`; `0x005C05F0..0x005C05F7`; `0x00616270..0x00616275` | Rust draws Bink as a GPU texture through the shell renderer rather than a virtual explicit-copy call. | `src/render/bink_movie.rs`, `src/app_main_menu_shell_render.rs` | Keep explicit draw as a separate render/copy phase in the parity model, not part of the update slot. | A `0x4F0`/paint-equivalent path copies the current decoded frame without advancing decode state. Proposed test: `bink_explicit_draw_does_not_step_frame`. | Do not tie `+0x28` to the timer update return path. |
| Setup/clip dispatch is vtable `+0x18 -> 0x005C05A0 -> 0x00432AB0`, not restart. | `0x007EE16C`; `0x005C05A0..0x005C05B2`; owner-draw setup calls `0x00616136`, `0x006161FB` | Rust layout uses direct movie dimensions and draw rects; no virtual clip helper equivalent. | `src/ui/main_menu_shell/layout.rs`, `src/app_main_menu_shell_render.rs` | Keep setup/clip concerns distinct from restart and draw when modeling native movie-handle behavior. | Movie setup stores/uses rect independently of timer loop restart. Proposed test: `bink_movie_setup_clip_slot_is_not_loop_restart`. | Do not shift table rows and accidentally call restart behavior for `+0x18`. |

### Stale Docs / Follow-up Docs

- `docs/research/BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`: replace section 3 rows for `+0x14/+0x1C` with:

  > Fresh Ghidra MCP read of BinkMovieHandle vtable `0x007EE154` resolves `+0x14` to `0x005C0570 -> 0x00432C50` (finished/end-or-wrap predicate) and `+0x1C` to `0x005C05D0 -> 0x00432BD0` (`_BinkGoto(handle, frame, 1)` wrapper). The earlier `+0x14 = 0x005C0550` and `+0x1C = 0x005C0570` rows were stale; `0x005C0550` is actually vtable `+0x10`.

- `docs/research/MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`: no replacement needed for the requested rows; its `+0x14/+0x18/+0x1C/+0x28` table matches the fresh MCP read.

## 10. Negative Facts / Do Not Do

- Do not use `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` section 3 as authority for `+0x14/+0x1C`.
- Do not identify `0x005C0550` as the finished predicate; fresh table puts it at `+0x10`, not `+0x14`.
- Do not identify `0x005C0570` as the loop/restart slot; it is `+0x14`, not `+0x1C`.
- Do not collapse update, finished, restart, setup/clip, and explicit draw into one native virtual slot.
- Do not treat Bink vtable `0x007EE154` as VQA-only; the `.bik` branch installs it.

## Sources

- Live Ghidra MCP instance: `gamemd.exe`, image base `0x00400000`, `read_memory 0x007EE154 len 64`.
- Ghidra MCP `get_bulk_xrefs` for `0x005C0580`, `0x005C0570`, `0x005C05A0`, `0x005C05D0`, `0x005C05F0`, `0x00433040`, `0x00432C50`, `0x00432BD0`, `0x00433060`.
- Ghidra MCP decompile: `OwnerDraw_Static_006153E0`, `VQMovieHandle__Constructor`, `BinkMovie_Update_005C0580`, `0x00433040`, `0x00432C50`, `0x00432BD0`, `0x00432AB0`, `BinkMovie_ExplicitDraw_005C05F0`, `0x00433060`.
- Ghidra MCP disassembly / direct read-backed thunk bytes: `0x005C0540..0x005C063F`, owner-draw dispatch ranges `0x00615B80..0x00615BD0` and `0x00616270..0x00616275`.
- Prior docs compared: `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`; `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`.
