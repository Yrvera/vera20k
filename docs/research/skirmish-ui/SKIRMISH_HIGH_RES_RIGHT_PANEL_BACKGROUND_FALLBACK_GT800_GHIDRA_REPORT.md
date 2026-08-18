# Skirmish High-Res Right-Panel Background Fallback >800 - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x0072CF40`, `0x0072CF90`, `0x0060CF00`, `0x00621E90`, `0x0072E730`, `0x004AED70`, `0x0072E450`, `0x0072EC70`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard Yuri's Revenge offline Skirmish dialog `0x102` behavior for the non-640 parent/right-panel background pointer at widths greater than 800, especially 1024x768: `DAT_00B0FA18`, parent record `+0xE4`, `Background_Overlay`, `FUN_0072CF40`, cleanup, and the `CC_Draw_Shape` null gate.  
**Non-Scope:** Choose Map preview refresh, `OwnerDraw_ListBox_00618D40`, `ComboDropWin 0x0060D540`, trackbar disabled runtime enable flow, start-marker clipping, full screenshot capture, WOL/online shells, and full SHP blitter internals after the null gate.  
**Confidence:** High for normal fresh standard Skirmish lifecycle and null draw behavior; Medium for abnormal stale-pointer histories because no live watchpoint was used.  
**Active in YR:** Yes for offline Skirmish `0x102`; conditional on high-resolution video mode for the `>800` branch.

## Working Notes

- Target question: At widths greater than 800, especially 1024x768, does the Skirmish parent/right-panel background path draw a fallback/stale `+0xE4` asset, draw nothing, or crash when `+0xE4` is null?
- Non-goals: Do not investigate Choose Map preview refresh, listbox owner draw, combo popup internals, trackbar disabled flow, or start-marker clipping.
- Evidence needed to mark COMPLETE: decompile plus assembly context for `FUN_0072CF40`, `FUN_0072CF90`, `FUN_0060CF00`, `WM_PAINT_Handler`, `Background_Overlay`, and `CC_Draw_Shape`; xrefs for `DAT_00B0FA18`; current Rust surface scan; stale-doc replacement wording for Q9.
- Stop conditions: Stop once the normal standard-YR lifecycle proves the fresh `>800` player-visible result and all remaining stale cases are either disproven by static xrefs or explicitly scoped to runtime watchpoint/screenshot work.

## 1. Overview

Fresh standard offline Skirmish at `1024x768` does not draw `MnScrnLCoopGameSetup.shp` as a parent-background fallback. The loader only fills `DAT_00B0FA18` at exact width `800`; the standard cleanup clears that global after the modal exits; dialog `0x102` copies the current global into parent `+0xE4`; and `Background_Overlay` passes that pointer to `CC_Draw_Shape` for every non-640 width. With a fresh `>800` entry, the pointer is null and `CC_Draw_Shape` returns before frame lookup or blit.

Player-visible result: the parent background overlay call is a no-op at fresh `>800`; right-panel chrome, lower strip, preview, controls, and text are separate paths and are not removed by this null background decision.

## 2. Key Offsets and Globals

| Field/global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00B0FA18` | Alternate/non-640 Skirmish parent background SHP pointer, loaded as `MnScrnLCoopGameSetup.shp` only at exact width 800 | xrefs: read `0x0060D2A8`, write `0x0072CF65`, read `0x0072CFA5`, write `0x0072CFCB` | Conditional: non-null only after exact-800 load in normal lifecycle |
| `DAT_00B0FCD8` | Ownership/free flag for the loaded alternate SHP | data arg at `0x0072CF5B`, read/clear in cleanup | Conditional |
| `DAT_00B0FCD9` | Loader guard for the Skirmish background/palette bundle | read/write `0x0072CF40`, `0x0072CF7F`, `0x0072CF90`, `0x0072CFFA` | Yes |
| `DAT_00B0FCDC` / `DAT_00B0FCE0` | Palette raw buffer and convert object for the parent background path | `0x0072CF70..0x0072CF7A`, cleanup `0x0072CFD5..0x0072CFF4` | Yes |
| Parent record `+0xE0` / `piVar[0x39]` | 640/small background pointer from `DAT_00B0FB50` | `FUN_0060CF00`; assembly `0x0060D29C..0x0060D2A2` | Yes |
| Parent record `+0xE4` / `piVar[0x3A]` | Non-640 alternate background pointer copied from `DAT_00B0FA18` | `FUN_0060CF00`; assembly `0x0060D2A8..0x0060D2AE`; read at `0x00622108` | Yes |
| `DAT_00B0FC1C` | Background destination rect used by `Background_Overlay` | `RightPanel__ComputeLayoutRects @ 0x0072EC70`; read in `Background_Overlay` | Yes |

## 3. Core Logic

### 3.1 Standard Skirmish owns the load/cleanup pair

`FUN_006AE2C0` is called from `Main_Game @ 0x0052E168`. It calls `FUN_0072CF40()` before creating dialog `0x102`, pumps the modal loop, tears down preview state, and then calls `FUN_0072CF90()` before returning.

Active in YR: Yes. Evidence: decompile of `0x006AE2C0`; xref from `Main_Game`.

### 3.2 Loader is exact-width 800 for the SHP, but not for the palette

`FUN_0072CF40` first checks `DAT_00B0FCD9`. If not already loaded, it compares `g_ScreenWidth` to `0x320` (`800`). Only the equal branch loads `MnScrnLCoopGameSetup.shp` and writes `DAT_00B0FA18`. The following palette/convert load for `MnScrnLCoopGameSetup.PAL` runs after the skipped branch and then sets the guard.

Active in YR: Yes. Evidence: decompile `0x0072CF40`; assembly context `0x0072CF40..0x0072CF86`, with exact compare at `0x0072CF49`, skip at `0x0072CF53`, SHP write at `0x0072CF65`, PAL path at `0x0072CF6A..0x0072CF7F`. String/data evidence: `0x00844D6C -> 0x00844FA8 "MnScrnLCoopGameSetup.shp"` and `0x00844D70 -> 0x00844F8C "MnScrnLCoopGameSetup.PAL"`.

At `1024`, this branch is skipped exactly like any other non-800 width.

### 3.3 Cleanup clears normal stale state

`FUN_0072CF90` is guarded by `DAT_00B0FCD9`. Inside the guarded body, it optionally frees the loaded SHP if `DAT_00B0FCD8` is set, then clears `DAT_00B0FA18` unconditionally, frees palette state, destroys the convert object, and clears the guard.

Active in YR: Yes. Evidence: decompile `0x0072CF90`; assembly context `0x0072CF90..0x0072D001`, pointer clear at `0x0072CFCB`, guard clear at `0x0072CFFA`; caller evidence `0x006AE2C0`.

Normal stale result: a completed standard Skirmish session cannot leak an exact-800 `DAT_00B0FA18` into a later fresh `>800` Skirmish entry.

### 3.4 Dialog `0x102` copies the current alternate pointer into `+0xE4`

`FUN_0060CF00` includes dialog id `0x102` in the shared branch with `0xBC`, `0xBD`, `0xC2`, and `0xC9`. That branch writes the convert pointer from `FUN_0072D030` to parent `+0x74`, `DAT_00B0FB50` to parent `+0xE0`, and `DAT_00B0FA18` to parent `+0xE4`.

Active in YR: Yes. Evidence: decompile `0x0060CF00`; assembly context `0x0060D294..0x0060D2AE`, including `MOV EDX,[0x00B0FA18]` and `MOV [ESI+0xE4],EDX`.

Fresh `>800` consequence: `+0xE4` is copied as zero because `FUN_0072CF40` did not populate the global.

### 3.5 Parent paint still selects `+0xE4` at `>800`

`WM_PAINT_Handler` in mode `+0xB0 == 1` calls `RightPanel__Draw`, then re-reads the parent background fields and calls `Background_Overlay`. It reads parent `+0xE4` at `0x00622108` and pushes it for the background call.

Active in YR: Yes, conditional on mode-1 shell paint and the right-panel-ready gate. Evidence: decompile `0x00621E90`; assembly context `0x00621FFE` right-panel call, `0x00622108` `+0xE4` read, `0x0062211B` background call.

`Background_Overlay @ 0x0072E730` clips the draw rect to a centered 800x600-ish bound when wider/taller than 800x600, reads the origin from `DAT_00B0FC1C`, chooses `+0xE0` only when `g_ScreenWidth == 640`, and chooses `+0xE4` for every other width.

Active in YR: Yes. Evidence: decompile `0x0072E730`; assembly `0x0072E74F..0x0072E791` clipping, `0x0072E7AD` width compare to `0x280`, 640 call at `0x0072E7DF`, non-640 call at `0x0072E815`.

### 3.6 `CC_Draw_Shape` null pointer behavior is no draw

`CC_Draw_Shape @ 0x004AED70` tests the selected SHP/frame pointer before lazy-load handling, frame rectangle lookup, frame data lookup, temporary surface creation, or blitter selection. If null, it jumps to the shared return. There is a second null test after lazy-load indirection.

Active in YR: Yes for all callers, including `Background_Overlay`. Evidence: decompile `0x004AED70`; assembly `0x004AED79..0x004AED8E` first null gate, `0x004AEDAB..0x004AEDAD` second null gate; call-site evidence `0x0072E808..0x0072E815`.

Player-visible result at fresh `1024x768`: no parent background SHP draw for that call. It is not a crash, not a fallback to `MNSCRNS.SHP`, and not a stretched/tiled `MnScrnLCoopGameSetup.shp`.

### 3.7 Stale non-null behavior is conditional and not normal

If `DAT_00B0FA18` were somehow non-null when dialog `0x102` copied it, neither `Background_Overlay` nor `CC_Draw_Shape` has a special stale-pointer guard; the pointer would be drawn as the supplied SHP. Static xrefs for `DAT_00B0FA18` show only the setup write, cleanup read/clear, and parent-copy read in this slice, and the standard caller pairs load and cleanup.

Active in YR: Conditional. Evidence: direct pass-through in `0x0060CF00`/`0x0072E730`/`0x004AED70`; xrefs for `DAT_00B0FA18`; cleanup clear at `0x0072CFCB`. A runtime watchpoint would be needed only to prove abnormal process histories outside normal standard entry/exit.

## 4. INI Keys

No INI key controls this slice. Inputs are screen-size globals, fixed shell string-table entries, loader/cleanup globals, and parent metadata fields. `AllowHiResModes` and video setup may affect whether a `>800` mode is available, but not the draw branch once `g_ScreenWidth` is set.

## 5. Integration Points

| Integration point | Status | Evidence | Active in YR |
|---|---|---|---|
| `Main_Game -> FUN_006AE2C0` | verified | xref from `0x0052E168` | Yes |
| Skirmish entry -> `FUN_0072CF40` -> dialog create | verified | decompile `0x006AE2C0` | Yes |
| Skirmish exit -> `FUN_0072CF90` | verified | decompile `0x006AE2C0` | Yes |
| `FUN_0060CF00` parent `+0xE4` assignment | verified | decompile and assembly `0x0060D2A8..0x0060D2AE` | Yes |
| `WM_PAINT_Handler -> Background_Overlay` | verified | `0x00622108`, `0x0062211B` | Yes |
| `Background_Overlay -> CC_Draw_Shape` | verified | `0x0072E7DF`, `0x0072E815` | Yes |

## 6. Current Rust Implementation Status

Current Rust already matches the verified fresh `>800` parent-background policy:

- `src/app_skirmish_shell_render.rs::parent_background_role` returns `Some(Mnscrns640)` at width `640`, `Some(CoopGameSetup800)` at exact width `800`, and `None` above `800`.
- `parent_background_role_uses_only_verified_widths` asserts `compute_layout(1024, 768)` returns `None`.
- `semantic_draw_order_keeps_1024_parent_blank_but_large_lower_strip` asserts the 1024 draw-order model contains no `ParentBackgroundMnscrns640` or `ParentBackgroundCoopGameSetup800`.
- `src/render/skirmish_shell_chrome.rs` loads both verified parent-background assets, which is acceptable as asset availability; semantic selection must remain width-gated.

No Rust delta is required for this exact `>800` parent-background decision.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Standard offline Skirmish reachability | verified | `Main_Game @ 0x0052E168`, `0x006AE2C0` | none |
| `FUN_0072CF40` exact-800 SHP load | verified | decompile and assembly `0x0072CF40..0x0072CF86` | none |
| `FUN_0072CF90` cleanup/reset | verified | decompile and assembly `0x0072CF90..0x0072D001` | none for normal lifecycle |
| `DAT_00B0FA18` xrefs | verified | bulk xrefs: `0x0060D2A8`, `0x0072CF65`, `0x0072CFA5`, `0x0072CFCB` | runtime watchpoint only for abnormal histories |
| Dialog `0x102` `+0xE4` assignment | verified | `FUN_0060CF00`; `0x0060D2A8..0x0060D2AE` | none |
| `WM_PAINT_Handler` background call | verified | `0x00622108`, `0x0062211B` | none |
| `Background_Overlay` non-640 selection | verified | `0x0072E7AD..0x0072E815` | none |
| `CC_Draw_Shape` null gate | verified | `0x004AED79..0x004AED8E`, `0x004AEDAB..0x004AEDAD` | none |
| Exact retail screenshot at 1024x768 | deferred | not a static Ghidra artifact | full-composition screenshot trace if pixel aggregate is needed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does standard offline Skirmish call the scoped loader before dialog `0x102` creation? -> Yes.` (evidence: `0x006AE2C0`)
- `[RESOLVED] OQ-02 - Does standard offline Skirmish call paired cleanup on normal exit? -> Yes.` (evidence: `0x006AE2C0`, `0x0072CF90`)
- `[RESOLVED] OQ-03 - Is `DAT_00B0FA18` loaded at 1024 or any `>800` width? -> No; load is exact `g_ScreenWidth == 800`.` (evidence: `0x0072CF49..0x0072CF65`)
- `[RESOLVED] OQ-04 - Is the palette/convert still loaded at `>800`? -> Yes, the PAL path follows the skipped SHP branch.` (evidence: `0x0072CF6A..0x0072CF7F`)
- `[RESOLVED] OQ-05 - Does cleanup clear the alternate pointer even if the ownership byte is false? -> Yes, `DAT_00B0FA18=0` is after the ownership branch but inside the guarded body.` (evidence: `0x0072CFCB`)
- `[RESOLVED] OQ-06 - Which parent field receives the alternate pointer for dialog `0x102`? -> Parent `+0xE4` / `piVar[0x3A]`.` (evidence: `0x0060D2A8..0x0060D2AE`)
- `[RESOLVED] OQ-07 - Which background pointer does `Background_Overlay` use above 800? -> The non-640 alternate pointer, parent `+0xE4`.` (evidence: `0x0072E7AD..0x0072E815`)
- `[RESOLVED] OQ-08 - What happens when the selected pointer is null? -> `CC_Draw_Shape` returns before frame lookup/blit.` (evidence: `0x004AED79..0x004AED8E`)
- `[RESOLVED] OQ-09 - Can stale `DAT_00B0FA18` survive a normal completed Skirmish lifecycle? -> No, cleanup clears it and resets the guard.` (evidence: `0x006AE2C0`, `0x0072CFCB`, `0x0072CFFA`)
- `[RESOLVED] OQ-10 - What would a non-normal stale non-null pointer do if copied into `+0xE4`? -> It would be passed through and drawn as supplied; no special stale guard exists in `Background_Overlay`/`CC_Draw_Shape`.` (evidence: `0x0060D2A8`, `0x0072E815`, `0x004AED84`)
- `[DEFERRED] OQ-11 - Can an abnormal retail process history leave `DAT_00B0FA18` non-null without standard cleanup?` (category: `needs-runtime-debugger`; reason: static standard lifecycle and xrefs say no for normal entry/exit; next-step-if-pursued: watch `DAT_00B0FA18` across shell transitions)
- `[DEFERRED] OQ-12 - Exact aggregate pixels of the full 1024x768 shell.` (category: `needs-runtime-debugger`; reason: this report proves background-pointer behavior, not screenshot parity for every surrounding surface; next-step-if-pursued: capture retail 1024x768 Skirmish first paint and compare aggregate composition)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Fresh `>800` Skirmish does not draw a parent-background SHP from `+0xE4` because the pointer is null and `CC_Draw_Shape` no-ops | `0x0072CF49..0x0072CF65`, `0x0060D2A8`, `0x0072E815`, `0x004AED84..0x004AED8E` | none observed | `src/app_skirmish_shell_render.rs::parent_background_role`, semantic draw-order tests | Keep returning `None` for width `>800` | 1024x768 Skirmish shell emits no `ParentBackgroundMnscrns640` and no `ParentBackgroundCoopGameSetup800`; proposed test name: `skirmish_gt800_parent_background_role_is_none_after_fresh_entry` | Do not stretch, tile, center, or reuse the exact-800 background above 800 |
| Exact width `800` remains a distinct loaded background case | `0x0072CF49`, `0x0072CF65`, non-640 call path `0x0072E815` | none observed | `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render.rs` | Keep 800 separate from `>800` | 800x600 shell can emit `ParentBackgroundCoopGameSetup800`; proposed test name: `skirmish_exact800_parent_background_uses_coop_game_setup` | Do not collapse `800` and `>800` into a generic large-width bucket |
| Normal cleanup prevents an 800-loaded pointer from leaking into a later fresh `>800` entry | `0x006AE2C0`, `0x0072CF90..0x0072D001`, `0x0072CFCB` | no explicit state needed while Rust derives role from current width | future shell resource lifecycle/cache surfaces | If a future gamemd-like cache models background pointers, reset it on shell exit | Enter 800 shell, exit, then enter 1024 shell: no parent-background role; proposed test name: `skirmish_gt800_does_not_reuse_cached_exact800_background` | Do not let asset cache availability become semantic draw state |

### Negative Facts / Do Not Do

- Do not draw `MnScrnLCoopGameSetup.shp` at widths greater than 800 for a fresh standard Skirmish entry. Active in YR: Yes. Evidence: exact-800 loader `0x0072CF49..0x0072CF65`, null no-op `0x004AED84..0x004AED8E`.
- Do not fallback from null `+0xE4` to `+0xE0`/`MNSCRNS.SHP` above 800. Active in YR: Yes. Evidence: `Background_Overlay` selects `+0xE0` only when `g_ScreenWidth == 640` at `0x0072E7AD`.
- Do not treat asset loading in Rust as proof that an asset should be drawn. Active in YR: Yes. Evidence: `DAT_00B0FA18` availability is separate from `DAT_00B0FA18` selection; current Rust asset atlas can contain assets while `parent_background_role` returns `None`.
- Do not model normal stale `+0xE4` leakage from an earlier exact-800 session. Active in YR: No for normal completed Skirmish lifecycle. Evidence: `FUN_006AE2C0` calls cleanup and `0x0072CFCB` clears the pointer.
- Do not use this report to change Choose Map preview, listbox, combo, trackbar, or marker clipping behavior. Active in YR: scope boundary. Evidence: non-scope and no traced code in those paths here.

### Remaining Uncertainty

- Full 1024x768 screenshot parity remains outside this static report; the remaining work is aggregate composition validation, not parent-background draw selection.
- Abnormal runtime histories that skip standard cleanup would require a debugger watchpoint on `DAT_00B0FA18`; static standard-YR entry/exit evidence does not show such a path.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md` Q9 replacement wording: "For a fresh normal `>800` Skirmish entry, `DAT_00B0FA18` is zero because `FUN_0072CF40` only loads the alternate background SHP at exact width 800 and `FUN_0072CF90` clears it during standard cleanup. `Background_Overlay` still selects parent `+0xE4` for non-640 widths, but `CC_Draw_Shape` receives a null SHP pointer and returns without drawing. Remaining 1024x768 work is full-composition screenshot parity or abnormal stale-pointer watchpointing, not deciding whether to reuse/stretch the 800 parent background."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md` coverage ledger replacement for `Background_Overlay @ 0x0072E730`: "`>800` fresh null behavior verified by `SKIRMISH_HIGH_RES_RIGHT_PANEL_BACKGROUND_FALLBACK_GT800_GHIDRA_REPORT.md`; abnormal stale non-null runtime history deferred to watchpoint only."

## Sources

- Ghidra decompile/disassembly/context: `0x006AE2C0`, `0x0072CF40`, `0x0072CF90`, `0x0060CF00`, `0x00621E90`, `0x0072E730`, `0x004AED70`, `0x0072E450`, `0x0072EC70`.
- Ghidra xrefs/data: `DAT_00B0FA18`, `DAT_00B0FCD8`, `DAT_00B0FCD9`, `DAT_00B0FCDC`, `DAT_00B0FCE0`, `0x00844D6C`, `0x00844D70`, `0x00844FA8`, `0x00844F8C`.
- Prior docs cross-checked: `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_GT800_BACKGROUND_POINTER_LIFECYCLE_GHIDRA_REPORT.md`, `SKIRMISH_HIGH_RES_SHELL_HOSTING_AND_GT800_BACKGROUND_GHIDRA_REPORT.md`, `SKIRMISH_GT800_BACKGROUND_TARGETED_TRACE_RECONCILIATION.md`, `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`.
- Rust scan: `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
