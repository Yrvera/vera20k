# Skirmish Shell Invalidation Globals DAT_00AC1CC8..DAT_00AC1DD0 - Ghidra Research Report

**Address(es):** `0x006107A0`, `0x00610810`, `0x00610950`, `0x00610B50`, `0x00610BF0`, common subclass thunk `0x00610CA0..0x006128D4`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Writer/reader roles for shell globals `DAT_00AC1CC8`, `DAT_00AC1DCC`, and `DAT_00AC1DD0`, limited to their common-shell/subclass-thunk behavior and whether standard offline Skirmish dialog `0x102` needs a Rust redraw/invalidation model for them.  
**Non-Scope:** Choose Map preview refresh, listbox paint, combo dropdown row behavior, trackbar disabled flow, start-marker clipping, full shell transition animation pixels, and unrelated owner-draw callbacks.  
**Confidence:** High for static binary roles and Rust handoff; Medium for runtime non-activation because this pass did not use a live breakpoint.  
**Active in YR:** Conditional. The reader/restore paths are in the active shell subclass thunk installed for dialog `0x102`; the activation writer `0x00610810` has no static code xref or function-pointer-table entry found in the retail binary sweep, so standard offline Skirmish has no evidence of enabling this overlay state.

## Working Notes

- Target question: What are `DAT_00AC1CC8`, `DAT_00AC1DCC`, and `DAT_00AC1DD0`, and does Rust need to model them as Skirmish invalidation state?
- Non-goals: Do not revisit settled Choose Map preview refresh, listbox paint, combo popup, trackbar disabled flow, or start-marker clipping.
- Evidence needed to mark COMPLETE: writer/readers for the three globals; active-YR path check from `0x102`; Rust handoff for redraw/caching behavior; stale-doc wording for OQ14.
- Stop conditions: If Ghidra lacks function boundaries, do not create them; use read-only disassembly and record the boundary limitation.

## 1. Overview

The three globals are part of a shell-global transient surface overlay, not a general dirty-rectangle or child invalidation queue. `DAT_00AC1DCC` is the active flag, `DAT_00AC1CC8` is a heap surface pointer containing a saved copy of a rectangular screen region, and `DAT_00AC1DD0` records whether that saved region has already been restored to the display surface.

The common subclass thunk reads these globals to restore the saved region before intersecting child redraw and to clean it up on focus/destroy-style messages. Standard Skirmish `0x102` reaches the reader paths because the thunk is installed on shell controls, but this investigation found no static caller that activates the overlay writer in normal retail YR.

## 2. Key Globals

| Global | Verified role | Active in YR | Evidence |
|---|---|---|---|
| `DAT_00AC1CC8` | Pointer to transient saved `BSurface`-like object; allocated by `0x00610950`, restored from via vtable `+8`, freed through vtable `+0` | Conditional: readers active in thunk; object non-null only if writer path activates | `0x00610980..0x006109EF`, `0x00611824..0x00611897`, `0x00611F58..0x00611FB0` |
| `DAT_00AC1DCC` | Overlay/region active flag; zero means all restore/draw helpers return or skip | Conditional; reset path exists, activation writer has no static caller found | reset `0x006107C1`, set `0x0061091F`, tested `0x0061180C`, `0x00611F48` |
| `DAT_00AC1DD0` | Saved-region restored flag; `0` after overlay redraw, `1` after saved surface is copied back | Conditional; readers active if overlay active | set/restored `0x00610894`, cleared `0x00610A32`, tested `0x0061181B`, `0x00611F50` |
| `DAT_00AC1CB8..DAT_00AC1CC4` | Rectangle copied from caller and later used as saved/restored bounds | Conditional | copied from caller `0x006108FF..0x00610925`; read `0x00611EEB..0x00611F1B` |
| `DAT_00AC1CCC` | Wide text buffer drawn by `0x00610950` after surface snapshot | Conditional | buffer setup `0x006108D0..0x006108FC`; draw call setup `0x00610AD5..0x00610AFB` |
| `DAT_00AC1DD4` | HWND/target identity used by thunk cleanup; cleanup only triggers when incoming HWND matches | Conditional | writer `0x0061092D`; compare to current HWND `0x006117DB..0x006117F0` |

## 3. Core Logic

### 3.1 Reset / Initialization

Active in YR: Yes as shell-global init/reset, but not Skirmish-specific. `0x006107A0` zeros `DAT_00AC1CC8`, `DAT_00AC1DCC`, `DAT_00AC1DD0`, the adjacent rectangle globals, the text buffer prefix, and `DAT_00AC1DD4`. Retail byte search found `0x006107A0` as a function pointer table entry at file offset `0x414378`, surrounded by other shell init/helper pointers.

Evidence: read-only disassembly `0x006107A0..0x006107D0`; retail `gamemd.exe` little-endian pointer sweep found only pointer-table hit for `0x006107A0`.

### 3.2 Activation Writer

Active in YR: Conditional / not proven for standard Skirmish. `0x00610810` consumes `ECX` as a four-int rect pointer and `EDX` as optional wide text. If an old overlay is active, it first restores/frees it, then copies text into `DAT_00AC1CCC`, copies the rect into `DAT_00AC1CB8..C4`, sets `DAT_00AC1DCC = 1`, writes `DAT_00AC1DD4`, and calls `0x00610950` with `CL = 1`.

The activation entry itself has no static code xref and no immediate pointer-table hit in the retail byte sweep. That means the reader code is live in the shell thunk, but this pass found no evidence that normal offline Skirmish creates this overlay state.

Evidence: disassembly `0x00610810..0x0061093C`; `get_function_xrefs 0x00610810` reports no function references because Ghidra has no boundary; retail code scan found no direct `CALL rel32` or immediate pointer to `0x00610810`.

### 3.3 Draw / Snapshot Helper

Active in YR: Conditional. `0x00610950` is the main draw/snapshot helper. It exits immediately if `DAT_00AC1DCC == 0`. When called with `CL != 0`, it frees any old `DAT_00AC1CC8`, allocates `0x20` bytes plus a pixel buffer sized from the global width/height (`width * height * 2`), stores a `BSurface` vtable, snapshots from `DAT_00887308` using vtable slot `+8`, and stores the new object in `DAT_00AC1CC8`.

Then it clears `DAT_00AC1DD0 = 0`, draws a filled/outlined rectangle and the text in `DAT_00AC1CCC`, and returns `AL = 1`. The only verified callsites are from the activation writer and from the common thunk end-of-paint path at `0x006128CF`.

Evidence: disassembly `0x00610950..0x00610B43`; Ghidra xrefs to `0x00610950` from `0x00610933` and `0x006128CF`.

### 3.4 Restore-Only And Restore-And-Clear Helpers

Active in YR: Conditional. `0x00610B50` restores the saved surface only if active, not already restored (`DAT_00AC1DD0 != 1`), and `DAT_00AC1CC8 != 0`; after the vtable `+8` copy it sets `DAT_00AC1DD0 = 1` and leaves the overlay active. `0x00610BF0` performs the same restore if needed, then frees `DAT_00AC1CC8` and zeros `DAT_00AC1CC8`, `DAT_00AC1DCC`, and `DAT_00AC1DD0`.

No static callers of these standalone helpers were found; the common thunk contains inlined equivalent restore/clear code.

Evidence: disassembly `0x00610B50..0x00610BE6`, `0x00610BF0..0x00610C9C`; retail code scan found no direct `CALL rel32` to either entry.

### 3.5 Common Thunk Readers

Active in YR: Yes for the reader branches, conditional for nonzero global state. Standard offline Skirmish installs common thunk `0x00610CA0` via `FUN_0060F9A0` during `FUN_00622B50` `WM_INITDIALOG`; the thunk body is therefore on the `0x102` control path.

There are two material reader blocks:

- `0x006117DB..0x006118A9`: after the thunk's temporary HWND list walk, if the incoming HWND matches `DAT_00AC1DD4` and the message is `0x82`, `0x18`, or `0x08`, it restores the saved surface if needed, frees `DAT_00AC1CC8`, and zeros all three globals.
- `0x00611EEB..0x00611FB6`: when descendant/paint processing is active and the current child invalidation rect intersects the overlay rect, the thunk restores the saved surface first and sets `DAT_00AC1DD0 = 1`; it also marks a local flag so `0x006128CF` later calls `0x00610950` to redraw the overlay after child processing.

Evidence: disassembly `0x006117DB..0x006118A9`, `0x00611EEB..0x00611FB6`, `0x006126A3..0x006128CF`; setup path from prior and spot-checked decompile `FUN_0060F9A0 @ 0x0060F9A0`.

## 4. INI Keys

No INI key drives this slice. Active in YR: Yes/Conditional as binary shell infrastructure, not content data. Evidence: all verified inputs are HWND/message state, display surfaces, and global shell records; no `rulesmd.ini`, `artmd.ini`, or CSF key controls the three globals.

## 5. Integration Points

| Integration point | Active in YR | Evidence | Skirmish implication |
|---|---|---|---|
| `FUN_0060F9A0` installs common thunk | Yes | decompile `0x0060F9A0`; prior thunk reports | Reader code can run for `0x102` controls |
| Common thunk descendant redraw aggregation | Yes, when messages enter thunk | `0x00611EEB..0x006128CF` | Rust may redraw directly; no HWND list model required |
| Overlay activation writer `0x00610810` | Conditional / no standard `0x102` proof | no static caller found in retail sweep | Do not add Skirmish state solely for this |
| `0x00610950` redraw helper | Conditional | xrefs from `0x00610933`, `0x006128CF` | Only relevant if overlay active |

## 6. Current Rust Implementation Status

Current Rust redraws the Skirmish shell in a direct render pass and stores control state explicitly:

- `src/ui/skirmish_shell/state.rs` has direct checkbox, trackbar, combo dropdown, and button state.
- `src/app_skirmish_shell_render.rs` renders Skirmish shell content every frame and lazily caches only the selected map preview texture.
- No Rust surface models a Win32 invalidation aggregation list, a transient `DAT_00AC1CC8` saved surface, or a `DAT_00AC1DD0` restored flag.

That is acceptable for this slice. A future implementation should only add a UI/render dirty-resource marker for real Rust caches, such as preview textures or layout-dependent GPU resources, not for these shell globals.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x006107A0` global reset | verified | disassembly `0x006107A0..0x006107D0`; pointer-table byte hit | exact owning init dispatcher name not needed |
| `0x00610810` activation writer | verified for behavior; touched-not-exhausted for reachability | disassembly `0x00610810..0x0061093C`; no static refs found | live runtime breakpoint could prove never-called in a session |
| `0x00610950` draw/snapshot helper | verified | disassembly `0x00610950..0x00610B43`; xrefs `0x00610933`, `0x006128CF` | exact helper names for display vtable slots out of scope |
| `0x00610B50` restore-only helper | verified behavior; no caller found | disassembly `0x00610B50..0x00610BE6`; retail call scan | none for Skirmish handoff |
| `0x00610BF0` restore-clear helper | verified behavior; no caller found | disassembly `0x00610BF0..0x00610C9C`; retail call scan | none for Skirmish handoff |
| Thunk cleanup reader | verified | `0x006117DB..0x006118A9` | none |
| Thunk intersecting-child reader | verified | `0x00611EEB..0x00611FB6`, `0x006128CF` | runtime overlay active case not captured |
| Rust redraw/caching comparison | verified | Rust source scan | no code change in this research slot |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - What is the investigation mode and exact slice? -> exhaustive-slice for the three shell globals and Rust redraw implication only.` (evidence: user scope)
- `[RESOLVED] OQ2 - Is the common thunk active in standard offline Skirmish 0x102? -> Yes; prior and spot-checked setup path installs `0x00610CA0` through `FUN_0060F9A0` from common shell init.` (evidence: `0x0060F9A0`, prior `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ3 - What writes `DAT_00AC1CC8`? -> `0x00610950` writes a newly allocated saved surface, restore/clear paths zero it, and no other direct writer was found in this slice.` (evidence: `0x006109EF`, `0x00611897`, `0x00610C84`)
- `[RESOLVED] OQ4 - What writes `DAT_00AC1DCC`? -> reset/clear writes zero; activation writer sets it to one after copying rect/text.` (evidence: `0x006107C1`, `0x0061091F`, `0x0061189D`)
- `[RESOLVED] OQ5 - What writes `DAT_00AC1DD0`? -> draw helper clears it to zero; restore paths set it to one; cleanup paths zero it.` (evidence: `0x00610A32`, `0x00611FB0`, `0x006118A3`)
- `[RESOLVED] OQ6 - Is this a general dirty-rectangle accumulator? -> No; it is a single saved-surface overlay rect plus active/restored flags.` (evidence: single pointer and rect globals `0x006108FF..0x00610925`, `0x00611EEB..0x00611FB6`)
- `[RESOLVED] OQ7 - Does standard Skirmish need a Rust model of these globals? -> No direct model; direct redraw covers the visible restore/redraw effect unless a future cached overlay is introduced.` (evidence: Rust render/state scan; no active writer proof)
- `[RESOLVED] OQ8 - What about null pointer edge cases? -> Restore branches skip if `DAT_00AC1CC8 == 0`; clear still zeros flags.` (evidence: `0x00611824..0x006118A3`, `0x00610C74..0x00610C90`)
- `[RESOLVED] OQ9 - What about already-restored edge cases? -> If `DAT_00AC1DD0 == 1`, restore copy is skipped, but free/clear may still happen on cleanup.` (evidence: `0x0061181B..0x00611897`, `0x00610C06..0x00610C84`)
- `[RESOLVED] OQ10 - What about non-intersecting child redraw? -> The thunk only restores before child processing if the overlay rect intersects the current child rect.` (evidence: `IntersectRect` call at `0x00611F3E` before global tests)
- `[RESOLVED] OQ11 - What about INI/default gates? -> None; no INI participates.` (evidence: binary-only shell path)
- `[DEFERRED] OQ12 - Can live runtime prove `0x00610810` is never reached in a full retail Skirmish session?` (category: `needs-runtime-debugger`; reason: static code and pointer sweeps found no caller, but no breakpoint run was performed; next-step-if-pursued: set an execute breakpoint on `0x00610810` while browsing Skirmish)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| The three globals are a single transient saved-surface/status-overlay state, not a Skirmish dirty-rectangle queue. | `0x006108FF..0x00610925`, `0x00610950..0x00610B43`, `0x00611EEB..0x00611FB6` | none observed for steady-state renderer | `src/app_skirmish_shell_render.rs`; `src/ui/skirmish_shell/state.rs` | Keep direct redraw; do not introduce Win32-style invalidation globals for `0x102`. | Toggle checkbox, open/close combo, drag trackbar, choose/cancel modal; next render reflects state without a separate invalidation aggregator. Proposed test: `skirmish_shell_state_changes_render_without_win32_invalidation_globals` | Adding a fake global dirty queue can create stale/ordering bugs not present in Rust's direct render model. |
| If the overlay is active and intersects child redraw, gamemd restores the saved surface before child work and redraws the overlay afterward. | `0x00611F2C..0x00611FB6`, `0x006128C7..0x006128CF` | unchecked because Rust has no equivalent overlay | future UI overlay/debug/help-text surface only, not current Skirmish controls | If a future shell help/status overlay is implemented, model it as a render-layer resource with explicit snapshot/redraw ordering. | Show a shell overlay overlapping a combo dropdown; child redraw does not permanently erase the overlay after the frame. Proposed test: `shell_overlay_redraws_after_overlapping_child_redraw` | Do not apply this ordering to map preview, owner-draw listboxes, or combo dropdown rows. |
| Cleanup messages matching `DAT_00AC1DD4` restore/free the saved surface and zero all three globals. | `0x006117DB..0x006118A9` | no current Rust overlay resource to clean | future shell overlay resource lifecycle | Future overlay state must be cleared on shell exit/focus-destroy equivalent; current preview/cache lifecycle should remain separate. | Enter Skirmish, create a future transient overlay, exit/re-enter; no stale overlay texture appears. Proposed test: `shell_transient_overlay_clears_on_dialog_exit` | Do not clear selected-map preview texture just because this transient overlay cleanup exists. |

### Negative Facts / Do Not Do

- Do not model `DAT_00AC1CC8..DAT_00AC1DD0` as Skirmish preview invalidation. Evidence: preview paint uses parent `WM_PAINT`/`DAT_00AC1154`; these globals snapshot a generic display rectangle and text buffer at `0x00610950`.
- Do not add these globals to `sim/`. Evidence: all reads/writes are shell HWND/display-surface paths before gameplay launch.
- Do not treat the common thunk reader as proof that standard `0x102` activates the overlay. Evidence: activation writer `0x00610810` has no static caller or pointer hit found, while thunk readers are conditional on `DAT_00AC1DCC != 0`.
- Do not implement a general dirty-rectangle accumulator from this evidence. Evidence: there is one saved surface pointer and one rect, not a list; the list at `DAT_00AC1DE8` is separate child HWND aggregation.
- Do not re-open settled Choose Map/listbox/combo/trackbar/start-marker questions for this target. Evidence: no material finding here changes those sibling reports.

### Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`: replace OQ14 with: "`[RESOLVED] OQ14 - `DAT_00AC1CC8`, `DAT_00AC1DCC`, and `DAT_00AC1DD0` are a conditional shell-global transient saved-surface/status-overlay state: `DAT_00AC1CC8` is the saved surface pointer, `DAT_00AC1DCC` is the overlay-active flag, and `DAT_00AC1DD0` is the saved-region-restored flag. The active common thunk can restore/redraw this overlay around child redraw, but standard offline Skirmish `0x102` has no static evidence that the activation writer `0x00610810` is called. Rust should not add a Win32 invalidation aggregation model for these globals; direct redraw is sufficient unless a future shell overlay feature is implemented. See `SKIRMISH_SHELL_INVALIDATION_GLOBALS_DAT_00AC1CC8_00AC1DD0_GHIDRA_REPORT.md`.`"
- Same doc: replace the Section 3.7 sentence "The thunk maintains a temporary HWND list around message `0x4A9` and selected paint/input messages, invalidates intersecting descendants..." with: "The thunk maintains a temporary HWND list for descendant/control processing separately from `DAT_00AC1CC8..DAT_00AC1DD0`; those three globals are not the list, but a conditional transient saved-surface overlay restored before intersecting child redraw and redrawn afterward if active."

### Remaining Uncertainty

- A live runtime breakpoint on `0x00610810` would be needed to prove the activation writer is never reached during a full standard Skirmish browse/session; static evidence found no caller.
- Exact user-facing feature name for the overlay text in `DAT_00AC1CCC` remains unnamed; it behaves like shell status/help overlay text, but no visible standard `0x102` activation was found.

## Sources

- Ghidra read-only disassembly: `0x006107A0..0x00610C9C`, `0x006117DB..0x006118A9`, `0x00611EEB..0x00611FB6`, `0x006126A3..0x006128CF`.
- Ghidra decompile/spot checks: `FUN_0060F9A0 @ 0x0060F9A0`; prior common thunk setup reports for `0x00622B50` and `0x006AE3F0`.
- Ghidra xrefs: `get_function_xrefs 0x00610950` returned calls from `0x00610933` and `0x006128CF`; no Ghidra function boundary exists for `0x00610810`, `0x00610B50`, or `0x00610BF0`.
- Retail `gamemd.exe` byte sweeps: direct `CALL rel32` scan found no calls to `0x00610810`, `0x00610B50`, or `0x00610BF0`; little-endian immediate pointer scan found no pointer to `0x00610810` and one pointer-table hit for reset `0x006107A0`.
- Prior docs checked: `docs/research/skirmish-ui/SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md`.
- Rust source scanned: `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/app.rs`.
