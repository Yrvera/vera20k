# Skirmish >800 Background Pointer Lifecycle - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x0072CF40`, `0x0072CF90`, `0x0060CF00`, `0x00621E90`, `0x0072E730`, `0x004AED70`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard YR offline Skirmish dialog `0x102` lifecycle for the non-640 parent background pointer: loader guard, cleanup/reset, parent record `+0xE0/+0xE4`, `Background_Overlay`, and `CC_Draw_Shape` null-SHP behavior at widths greater than 800.  
**Non-Scope:** Runtime screenshot validation, nonstandard resolution switching while the modal dialog is already open, WOL/online dialogs, right-panel art geometry, and full SHP blitter internals after the null gate.  
**Confidence:** High for normal static lifecycle and null behavior; Medium for abnormal stale-pointer histories because no runtime watchpoint was used.  
**Active in YR:** Yes for the offline Skirmish setup path; conditional for `>800` because it requires a high-resolution video mode.

## 1. Overview

Fresh `>800` offline Skirmish entry does not load a large parent background SHP. `FUN_0072CF40` only writes `DAT_00B0FA18` when `g_ScreenWidth == 800`, while `Background_Overlay` selects the same alternate pointer for every non-640 width. In a normal Skirmish lifecycle, the paired cleanup `FUN_0072CF90` clears `DAT_00B0FA18`, so a subsequent fresh `>800` entry passes a null SHP pointer to `CC_Draw_Shape`, which returns immediately before frame lookup or blit.

The practical player-visible result for a fresh `>800` entry is no parent background overlay from `MnScrnLCoopGameSetup.shp`; right-panel chrome and other shell elements are separate draw paths and remain outside this pointer's null decision.

## 2. Key Offsets and Globals

| Field/global | Purpose in this slice | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00B0FA18` | Alternate/non-640 Skirmish parent background SHP pointer, normally `MnScrnLCoopGameSetup.shp` at exact 800 width | writes at `0x0072CF65`, clears at `0x0072CFCB`, parent copy at `0x0060D2A8` | Yes; non-null only when loaded at exact width 800 in normal lifecycle |
| `DAT_00B0FCD8` | Ownership/free flag for `DAT_00B0FA18` fallback allocation | passed as `EDX=0x00B0FCD8` to loader at `0x0072CF5B`, checked/cleared at `0x0072CF9C..0x0072CFBF` | Conditional; only matters when the loaded SHP buffer is owned/freed |
| `DAT_00B0FCD9` | Loader guard for the Skirmish parent background/palette bundle | read/write at `0x0072CF40`, `0x0072CF7F`, `0x0072CF90`, `0x0072CFFA` | Yes |
| `DAT_00B0FCDC` | Raw parent background palette buffer | written by `0x0072ADE0` call at `0x0072CF75`, freed at `0x0072CFD5` | Yes |
| `DAT_00B0FCE0` | Convert/palette object returned by `FUN_0072D030` and stored into parent record `+0x74` | written by `0x0072ADE0`, returned at `0x0072D030`, destroyed at `0x0072CFEE..0x0072CFF4` | Yes |
| Parent record `+0xE0` / `piVar2[0x39]` | 640/small background pointer from `DAT_00B0FB50` | branch for dialog `0x102` in `FUN_0060CF00`, write at `0x0060D29C..0x0060D2A2` | Yes |
| Parent record `+0xE4` / `piVar2[0x3A]` | non-640 alternate background pointer copied from `DAT_00B0FA18` | write at `0x0060D2A8..0x0060D2AE`, read before `Background_Overlay` call in `WM_PAINT_Handler` | Yes |

## 3. Core Logic

### 3.1 Standard Skirmish owns the loader and cleanup pair

`FUN_006AE2C0` calls `FUN_0072CF40` before creating dialog `0x102`, then after the modal loop and preview cleanup calls `FUN_0072CF90` before returning the Start/Back result.

Active in YR: Yes. Evidence: `FUN_006AE2C0` decompile shows `FUN_0072CF40()` before `FUN_00622650(0)` and `FUN_0072CF90()` after `DAT_00AC1154` preview cleanup, before the final return.

### 3.2 Loader only populates the alternate SHP at exact width 800

`FUN_0072CF40` first checks `DAT_00B0FCD9`; if the guard is already set, it returns without reloading. On a fresh guarded load, it compares `g_ScreenWidth` to `0x320` and only then calls the SHP loader for `MnScrnLCoopGameSetup.shp`, storing the result into `DAT_00B0FA18`. It always proceeds to load/convert `MnScrnLCoopGameSetup.PAL` and then sets `DAT_00B0FCD9 = 1`.

Active in YR: Yes. Evidence: `0x0072CF40..0x0072CF86`; exact-width compare at `0x0072CF49`, SHP load/store at `0x0072CF55..0x0072CF65`, PAL load/convert at `0x0072CF6A..0x0072CF7A`, guard write at `0x0072CF7F`. String evidence: `0x00844FA8 = "MnScrnLCoopGameSetup.shp"`, `0x00844F8C = "MnScrnLCoopGameSetup.PAL"`.

For `g_ScreenWidth > 800`, the `JNZ` at `0x0072CF53` skips the SHP load exactly like any non-800 width. There is no `>= 800` branch in this loader.

### 3.3 Cleanup clears stale state in the normal lifecycle

`FUN_0072CF90` checks the same guard. If the guard is set, it optionally frees the owned `DAT_00B0FA18` buffer when `DAT_00B0FCD8 != 0`, then writes `DAT_00B0FA18 = 0` unconditionally inside the guarded cleanup body. It also frees `DAT_00B0FCDC`, destroys `DAT_00B0FCE0`, and finally clears `DAT_00B0FCD9`.

Active in YR: Yes. Evidence: `0x0072CF90..0x0072D001`; owned-SHP cleanup at `0x0072CF9C..0x0072CFBF`, unconditional pointer clear at `0x0072CFCB`, palette free at `0x0072CFD5..0x0072CFDE`, convert destroy at `0x0072CFEE..0x0072CFF4`, guard clear at `0x0072CFFA`.

Normal stale survival answer: no. Since standard offline Skirmish calls `FUN_0072CF90` after the modal loop, `DAT_00B0FA18` cannot survive a normal completed Skirmish setup lifecycle.

### 3.4 Dialog `0x102` copies whatever the global currently holds

During parent metadata setup, `FUN_0060CF00` includes dialog id `0x102` in the branch that stores `FUN_0072D030()` into parent `+0x74`, `DAT_00B0FB50` into parent `+0xE0`, and `DAT_00B0FA18` into parent `+0xE4`.

Active in YR: Yes. Evidence: `FUN_0060CF00` branch for `0x102 || 0xBC || 0xBD || 0xC2 || 0xC9`; writes visible in decompile as `piVar2[0x1E] = FUN_0072D030()`, `piVar2[0x39] = DAT_00B0FB50`, `piVar2[0x3A] = DAT_00B0FA18`; assembly xrefs to `DAT_00B0FA18` at `0x0060D2A8`.

At a fresh `>800` entry after normal cleanup, this means parent `+0xE4` is copied as zero.

### 3.5 `Background_Overlay` selects `+0xE4` for all non-640 widths

`Background_Overlay @ 0x0072E730` first clips the right/bottom of the destination rect toward an 800x600-centered bound when the incoming rect exceeds 800x600. Then it reads the common shell origin from `DAT_00B0FC1C`. It selects parent `+0xE0` only when `g_ScreenWidth == 640`; otherwise it calls `CC_Draw_Shape` with parent `+0xE4`.

Active in YR: Yes. Evidence: `0x0072E74F..0x0072E775` clamps width when `rect.w > 800`, `0x0072E779..0x0072E791` clamps height when `rect.h > 600`, `0x0072E7AD` compares width to `0x280`, `0x0072E7DF` calls `CC_Draw_Shape` on the 640 path, and `0x0072E815` calls it on the non-640 path.

This selection does not distinguish 800 from `>800`; the distinction is entirely in `FUN_0072CF40`'s loader.

### 3.6 Null SHP passed to `CC_Draw_Shape` is an early no-op

At `CC_Draw_Shape @ 0x004AED70`, the chosen SHP pointer is the first stack argument at entry. The function moves it to `EDI` and tests it before frame-wrapper handling, clipping, frame rect lookup, frame data lookup, temporary surface construction, or blitter selection. If it is null, the function jumps to the shared return path.

Active in YR: Yes. Evidence: `0x004AED79` loads first stack argument into `EDI`, `0x004AED84..0x004AED8E` tests it and jumps to return at `0x004AF289` when zero. `Background_Overlay` pushes parent `+0xE0` or `+0xE4` as that argument immediately before its `CC_Draw_Shape` call (`0x0072E7D3..0x0072E7DF`, `0x0072E808..0x0072E815`).

Fresh `>800` draw answer: parent background overlay is a no-op, not a crash and not stale art, under the normal lifecycle.

## 4. INI Keys

No INI key participates in this pointer lifecycle. Inputs are screen-size globals, fixed shell resource string tables, parent metadata fields, and static loader guard/cleanup globals.

## 5. Integration Points

| Integration point | Status | Evidence | Active in YR |
|---|---|---|---|
| Offline Skirmish entry calls loader before dialog creation | verified | `FUN_006AE2C0` call order | Yes |
| Offline Skirmish exit calls cleanup after modal loop | verified | `FUN_006AE2C0` call to `FUN_0072CF90` | Yes |
| Loader writes alternate SHP only at exact 800 | verified | `0x0072CF49..0x0072CF65` | Yes; conditional on width 800 |
| Parent setup copies global `DAT_00B0FA18` into parent `+0xE4` | verified | `FUN_0060CF00`, xref `0x0060D2A8` | Yes |
| Parent paint selects `+0xE4` for `>800` | verified | `Background_Overlay @ 0x0072E730` | Yes; conditional on high-res width |
| `CC_Draw_Shape` null SHP early return | verified | `0x004AED79..0x004AED8E` | Yes |

## 6. Current Rust Implementation Status

Rust already has the key `>800` parent-background policy isolated in `src/app_skirmish_shell_render.rs`: `parent_background_role` returns `None` for widths above 800 and logs that Ghidra verifies no fresh `>800` parent substitution. That matches this report for the normal fresh lifecycle.

Rust also loads `MNSCRNS.SHP` and `MnScrnLCoopGameSetup.shp` as optional parent backgrounds in `src/render/skirmish_shell_chrome.rs`. This is acceptable as asset availability; the runtime selection must keep skipping the parent background at `>800`.

Separate non-target delta: the semantic draw-order helper currently pushes parent background before right-panel roles for 640/800, while prior paint-order research says gamemd draws right-panel chrome before `Background_Overlay`. That is outside this pointer-lifecycle slice but remains an implementation parity issue for standard widths.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_006AE2C0` loader/cleanup lifecycle | verified | decompile call order | none |
| `FUN_0072CF40` guard and exact-800 SHP load | verified | `0x0072CF40..0x0072CF86` | none |
| `FUN_0072CF90` cleanup/reset | verified | `0x0072CF90..0x0072D001` | none for normal lifecycle |
| `DAT_00B0FA18` xrefs | verified | xrefs: read `0x0060D2A8`, write `0x0072CF65`, read `0x0072CFA5`, write `0x0072CFCB` | none |
| Dialog `0x102` parent `+0xE0/+0xE4` assignment | verified | `FUN_0060CF00` | none |
| `Background_Overlay` non-640 selection | verified | `0x0072E7AD..0x0072E815` | none |
| `CC_Draw_Shape` null-SHP gate | verified | `0x004AED79..0x004AED8E` | none |
| Abnormal stale non-null history without normal cleanup | deferred | static normal lifecycle clears it | runtime watchpoint or fault-injection session only if needed |

## 8. Open Questions - Final State

- [RESOLVED] Q1 - Does standard offline Skirmish call the scoped loader before dialog creation? -> Yes. (evidence: `FUN_006AE2C0`)
- [RESOLVED] Q2 - Does standard offline Skirmish call the paired cleanup on normal exit? -> Yes, after modal loop/preview cleanup. (evidence: `FUN_006AE2C0`)
- [RESOLVED] Q3 - Is `DAT_00B0FA18` loaded for `>800`? -> No; the loader uses exact `g_ScreenWidth == 800`. (evidence: `0x0072CF49..0x0072CF65`)
- [RESOLVED] Q4 - Is the PAL/convert still loaded for `>800` fresh entry? -> Yes, after the skipped SHP branch. (evidence: `0x0072CF6A..0x0072CF7F`)
- [RESOLVED] Q5 - Does cleanup clear the alternate SHP pointer even if the ownership flag is false? -> Yes, `DAT_00B0FA18 = 0` is after the ownership/free branch but inside the guard body. (evidence: `0x0072CFCB`)
- [RESOLVED] Q6 - Which parent field receives the alternate pointer? -> Parent `+0xE4` / `piVar2[0x3A]`. (evidence: `FUN_0060CF00`, `0x0060D2A8`)
- [RESOLVED] Q7 - Which pointer does `Background_Overlay` use at `>800`? -> The non-640 alternate pointer, parent `+0xE4`. (evidence: `0x0072E7AD..0x0072E815`)
- [RESOLVED] Q8 - What happens if that pointer is null? -> `CC_Draw_Shape` returns before frame lookup or blit. (evidence: `0x004AED79..0x004AED8E`)
- [RESOLVED] Q9 - Can stale `DAT_00B0FA18` survive normal completed Skirmish lifecycle? -> No, normal cleanup clears it and resets the guard. (evidence: `0x006AE2C0`, `0x0072CFCB`, `0x0072CFFA`)
- [DEFERRED] Q10 - Can abnormal process history leave stale non-null `DAT_00B0FA18` without the normal cleanup call? (category: needs-runtime-debugger; reason: static standard lifecycle has cleanup, but injected/broken lifecycle histories are not proven by static path alone; next-step-if-pursued: set a watchpoint on `DAT_00B0FA18` across shell transitions.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Fresh `>800` Skirmish entry does not draw `MnScrnLCoopGameSetup.shp` as parent background | `0x0072CF49..0x0072CF65`, `0x0072CFCB`, `0x004AED79..0x004AED8E` | none observed for current helper: `parent_background_role` skips `width > 800` | `src/app_skirmish_shell_render.rs` | Keep parent background role as `None` above 800 for fresh normal lifecycle | 1024x768 dev Skirmish shell has no parent-background sprite instance from `MnScrnLCoopGameSetup.shp` | Do not stretch or reuse the 800 background at `>800` as a convenience fallback |
| 800 width still uses `MnScrnLCoopGameSetup.shp` through the alternate pointer | `0x0072CF55..0x0072CF65`, `FUN_0060CF00`, `Background_Overlay` | none observed for asset presence | `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render.rs` | Keep exact 800 path distinct from `>800` | 800x600 shell can emit the 800 parent background role | Do not collapse 800 and `>800` into one "large" bucket |
| Normal cleanup clears stale alternate pointer before a later fresh entry | `FUN_006AE2C0`, `0x0072CF90..0x0072D001` | no explicit lifecycle state needed if Rust derives fresh role from current width | UI shell state/lifecycle if a future cache models gamemd globals | Any future cached parent-background pointer must be reset on shell exit | Enter 800 Skirmish, exit, then enter 1024 Skirmish: no parent background overlay | Do not let an 800 cached background leak into `>800` |

## Stale Docs / Follow-up Docs

- `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md` deferred the `>800` null/stale question. Replacement wording: "For a fresh normal `>800` Skirmish entry, `DAT_00B0FA18` is zero after paired cleanup and is not loaded by `FUN_0072CF40`; `CC_Draw_Shape` receives a null SHP pointer and returns immediately. Stale non-null survival is only an abnormal-history/runtime-watchpoint question."
- Older docs that recommend keeping `>800` parent background unresolved can now cite this report for static normal lifecycle behavior.

## Sources

- Ghidra decompile / assembly context: `0x006AE2C0`, `0x0072CF40`, `0x0072CF90`, `0x0060CF00`, `0x00621E90`, `0x0072E730`, `0x004AED70`, `0x004A38D0`, `0x0072ADE0`, `0x0072D030`.
- Ghidra xrefs: `DAT_00B0FA18`, `DAT_00B0FCD8`, `DAT_00B0FCD9`, `DAT_00B0FCDC`, `DAT_00B0FCE0`, string pointers `0x00844D6C`, `0x00844D70`.
- String search: `0x00844FA8 = "MnScrnLCoopGameSetup.shp"`, `0x00844F8C = "MnScrnLCoopGameSetup.PAL"`.
- Prior docs referenced: `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_HIGH_RES_SHELL_HOSTING_ORIGIN_GHIDRA_REPORT.md`, `SKIRMISH_HIGH_RES_SHELL_HOSTING_AND_GT800_BACKGROUND_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`.
- Rust scan: `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
