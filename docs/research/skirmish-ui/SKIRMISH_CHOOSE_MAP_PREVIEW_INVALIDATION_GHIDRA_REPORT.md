# Skirmish Choose Map Preview Invalidation - Ghidra Research Report

**Address(es):** `0x006ACEE0` primary Choose Map command branch; `0x005E68A0`, `0x005E7160`, `0x005E74E0`, `0x006AE3F0`, `0x006406E0`, `0x006406F0`, `0x00641DB0`, `0x0069ADF0`, `0x0069AE70`, `0x00640710`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** successful `FUN_006ACEE0` Choose Map return through selected-map state, preview object replacement/invalidation, and dialog repaint; limited to the Choose Map branch and immediate preview invalidation/update calls.  
**Non-Scope:** full map chooser filtering/sorting, full PreviewPack channel-order proof, random-map generation internals, assigned-player marker overlays, start-position projection formulas.  
**Confidence:** High for the Choose Map return branch, selected-map state writes, preview replacement/invalidation order, and repaint handoff.  
**Active in YR:** Yes for standard offline Skirmish Choose Map; conditional subpaths are marked per finding.

## 1. Overview

Successful Choose Map selection in offline Skirmish does not paint the preview directly. `0x006ACEE0` rebuilds selected-map UI state, refreshes or replaces `DAT_00AC1154`, then invalidates the setup dialog so `WM_PAINT` later consumes the current preview object.

The invalidation is parent-dialog invalidation with `erase = FALSE`. The random-map path invalidates explicitly in `0x006ACEE0`; the normal stock-map path delegates invalidation to `0x005E74E0`.

## 2. Key Offsets And Globals

| Item | Verified purpose in this slice | Evidence | Active in YR |
|---|---|---|---|
| Dialog `0x102` | Offline Skirmish setup dialog that routes `WM_COMMAND`/`WM_PAINT`. | `0x006AE2C0`, `0x006AE3F0` | Yes: standard offline Skirmish setup. |
| Command `0x5AA` | Choose Map branch in `0x006ACEE0`. | branch at `0x006AD8E7` after command dispatch from `0x006AE443` | Yes. |
| `DAT_00A8B250` | Selected mode/category token saved before chooser and restored on failure/cancel. | `0x006AD8E7`, `0x006AD95B`, `0x006ADB52`; object identity corrected by `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md` | Yes. |
| `DAT_00A8B254` | Selected scenario index saved before chooser and rewritten on accepted selection. | `0x005E7160`, `0x006AD905`, `0x006ADA77`, `0x006ADB4B` | Yes. |
| `DAT_00A8B8E0` | Current selected map file path consumed by normal preview loader. | `0x006AD8ED`, `0x005E74E0` | Yes. |
| `DAT_00AC1154` | Global 4-byte preview wrapper; wrapper field `+0` is the inner preview surface pointer. | `0x006406E0`, `0x006406F0`, `0x006AE3F0` | Yes. |
| Record `+0x58` | Random-map sentinel field compared to `RandMap.Sed` by `0x0069ADF0`. | `0x0069ADF0` | Conditional: true only for random-map selection. |
| Record `+0x6A8` | Random-map sentinel field compared by common preview loader `0x0069AE70`. | `0x0069AE70` | Conditional: true only for random-map records. |
| Child `0x468` | Preview child used by setup `WM_PAINT` before drawing starts/preview. | `0x006AE45D`, `0x00640710` | Yes. |

## 3. Core Logic

### 3.1 Entry And Modal Return

`0x006AE3F0` routes `WM_COMMAND` to `0x006ACEE0` by passing the low word of `wParam` as the command id. `0x006ACEE0` enters the Choose Map branch for command `0x5AA`, saves `DAT_00A8B250`, saves `DAT_00A8B254`, copies `DAT_00A8B8E0` to a stack buffer and into `DAT_00A8B322`, calls `0x00608070`, hides the setup dialog with `ShowWindow(hwnd, 0)`, then calls `0x005E68A0`.

Active in YR: Yes. Evidence: `0x006AE2C0` creates/pumps the offline Skirmish dialog, `0x006AE3F0` calls `0x006ACEE0` at `0x006AE443`, and `0x006ACEE0` has the `0x5AA` branch at `0x006AD8E7`.

`0x005E68A0` creates the modal map dialog from resource `0x6B` with proc `0x005E6920`, shows it, runs modal pump `0x007759E0`, and returns the modal result to `0x006ACEE0`.

Active in YR: Yes. Evidence: direct call from Choose Map branch at `0x006AD947`.

### 3.2 Accepted Return State Rebuild

The parent treats return value `2` as restore/cancel. The successful/accepted path is the `return != 2` path at `0x006ADA21`.

On accepted return, the branch:

1. Computes selected-map player capacity from current `DAT_00A8B254` with `0x005E6520`.
2. Optionally clamps capacity through the selected `MPModes` mode/category object's vtable `+0x04` and `DAT_00A8B230+0x11E4`.
3. Rebuilds player option state via `0x004E4FC0`, `0x004E5310`, and `0x004E5D60`.
4. Calls `0x006ADDF0(setup, old_index, DAT_00A8B254)`.
5. Shows the setup dialog with `ShowWindow(hwnd, 5)`.
6. Calls `0x005E7BF0(DAT_00A8B254)`.
7. If `0x005E7BF0` fails, restores saved `DAT_00A8B254` and `DAT_00A8B250` and returns without invalidating.
8. If load succeeds, updates derived map text (`0x005D5F30`, `0x005E2EF0`, `0x005E2F60`), updates dependent controls via `0x006ACD60`, then refreshes the preview.

Active in YR: Yes. Evidence: contiguous accepted branch at `0x006ADA21..0x006ADB45`; no TS-only gate found. The vtable/session clamp is conditional on the vtable result, but the branch itself is live in the offline Skirmish path.

### 3.3 Selected-Map Writes From The Modal Dialog

`0x005E7160` accepts the map dialog selection. It reads list selection from child `0x553`, finds the matching entry in `DAT_00A8B8CC`, may read mode/category selection from child `0x6EB`, and writes selected state:

- If the selected mode/category pointer changes, `DAT_00A8B250 = selected_mode[10]`, `DAT_00A8B23C = selected_mode`, and `DAT_00A8B254 = matched_index`. The selected map record itself remains `DAT_00A8B8CC[DAT_00A8B254]`.
- It then writes `DAT_00A8B254 = matched_index` again even if the session did not change.
- It closes the modal dialog through `0x007757E0`.
- It updates chooser-dialog child text (`0x6EC`, `0x5A8`) before returning.

Active in YR: Yes. Evidence: `0x005E6920` command branch calls `0x005E7160` at `0x005E6B67`; `0x005E7160` has `g_GameMode == 5` handling through `0x006ACCA0`.

## 4. Preview Replacement And Invalidation

### 4.1 Random-Map Accepted Path

After selected-map load succeeds, `0x006ACEE0` checks the selected record with `0x0069ADF0(record)`, which compares `record+0x58` with string `RandMap.Sed`.

If true:

1. Reads old `DAT_00AC1154`.
2. If old wrapper is non-null, calls `0x006406F0(old)` and then frees the wrapper with `0x007C8B3D(old)`.
3. Allocates exactly 4 bytes.
4. If allocation succeeds, calls `0x006406E0(wrapper)`, which only writes wrapper field `+0 = 0`.
5. Stores the new wrapper in `DAT_00AC1154`.
6. Calls `0x00641DB0(DAT_00AC1154, "RandMap.img")`.
7. If the wrapper's inner pointer is still null after that load, calls fallback `0x005E74E0(setup)`.
8. Calls `InvalidateRect(setup, NULL, FALSE)` once at `0x006ADB19..0x006ADB1E`.

Active in YR: Conditional. Evidence: branch is live in standard offline Skirmish, but it only executes when selected record `+0x58` equals `RandMap.Sed` (`0x0069ADF0`; string `RandMap.Sed @ 0x0082BC30`; `RandMap.img @ 0x00829ABC`).

Tiny details:

- Destroy-before-free is explicit: `0x006406F0` destroys the inner surface through its vtable and clears wrapper field `+0`; caller then frees the wrapper allocation.
- Allocation failure sets the new wrapper value to zero. The later `*DAT_00AC1154` check is not guarded in the accepted random branch, so this path assumes allocation succeeds or would be unsafe.
- The new wrapper is zero-initialized before `RandMap.img` load; `0x006406E0` does not populate a preview.
- The explicit invalidation uses `erase = FALSE` (`Push 0`, `Push 0`, `Push hwnd` before `InvalidateRect` import at `0x007E149C`).

### 4.2 Normal Stock-Map Accepted Path

If `0x0069ADF0(record)` is false, `0x006ACEE0` does not invalidate directly. It calls `0x005E74E0(setup)` and returns.

`0x005E74E0` starts by destroying and freeing any existing `DAT_00AC1154`, then sets the global to zero. In the default offline path, it opens `DAT_00A8B8E0`, allocates a 4-byte wrapper if the file opens, initializes it with `0x006406E0`, and, when `DAT_00AC1154` is non-null, checks `0x0069AE70`. If the selected record is not `RandMap.Sed`, it calls the `.map` preview decode path (`0x00641EE0` / `0x00641B00`), then invalidates the setup dialog with `InvalidateRect(setup, NULL, FALSE)`.

Active in YR: Yes for normal selected stock-map previews. Evidence: accepted non-random branch at `0x006ADB31..0x006ADB33`; loader body at `0x005E74E0`; default branch runs for `g_GameMode` values outside network/special cases, including offline Skirmish.

Tiny details:

- The old preview is cleared before any new file open/load attempt. If load fails, paint sees no preview object.
- `0x005E74E0` invalidates only if `DAT_00AC1154 != 0` after the load attempt.
- The common loader has a special branch for random-map records via `0x0069AE70(record+0x6A8)`, separate from the parent branch's `record+0x58` check.

### 4.3 Cancel/Restore Contrast

Return value `2` restores saved `DAT_00A8B250` and `DAT_00A8B254`, calls `0x005E7BF0`, calls `0x005E74E0`, shows the setup dialog, then rechecks random-map status. On the restore/random path, explicit invalidation occurs twice after `RandMap.img` load/fallback.

Active in YR: Conditional. Evidence: branch at `0x006AD94C..0x006ADA1E`; it is active only when modal result equals `2`, so it is not the successful accepted path requested here.

## 5. Repaint Handoff

The Choose Map handler does not call `DrawStartPositions` directly. It invalidates the setup dialog, and `0x006AE3F0` handles the later paint path.

For `WM_PAINT`, `0x006AE3F0` checks `DAT_00AC1154 != 0`, gets child `0x468`, calls `0x006067A0(child)`, and only when that returns false calls `DrawStartPositions @ 0x00640710` with `DAT_00AC1154` and the setup HWND. It then validates the parent rect.

`DrawStartPositions` validates the dialog rect at entry and only draws if wrapper field `+0` is non-null. Thus a non-null wrapper with null inner surface does not draw the preview/start positions.

Active in YR: Yes. Evidence: `0x006AE3F0` `WM_PAINT` branch at `0x006AE454..0x006AE483`; `DrawStartPositions @ 0x00640710` checks `*param_1`.

## 6. Current Rust Implementation Status

| Rust area | Status vs this slice | Evidence |
|---|---|---|
| Choose Map action | Not parity-equivalent: current action increments `selected_map_idx` in-place instead of opening a modal chooser with accepted/cancel restore semantics. | `src/ui/skirmish_shell/state.rs:165` |
| App routing | `ChooseMap` is swallowed as a no-transition action after state handling. | `src/app.rs:557` |
| Preview cache invalidation | Current renderer lazily rebuilds preview texture when cached `selected_map_idx` differs; it clears texture when decode fails. This is conceptually similar for normal selection but lacks modal accepted/cancel ordering and random-map `RandMap.img` branch. | `src/app_skirmish_shell_render.rs:736`, `src/app_skirmish_shell_render.rs:765` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline Skirmish dialog creation/pump | verified | `0x006AE2C0` | none for this slice |
| `WM_COMMAND` to `0x006ACEE0` | verified | `0x006AE3F0`, xref `0x006AE443` | none |
| `0x006ACEE0` Choose Map accepted path | verified | `0x006AD8E7..0x006ADB45` | none |
| Modal chooser wrapper | verified for return integration | `0x005E68A0` | chooser list internals out-of-scope |
| Modal accept selected-map writes | verified | `0x005E7160`, `0x005E6B67` | validation prompts outside immediate return path |
| Random-map preview replacement | verified | `0x006ADAC3..0x006ADB1E`, `0x006406E0`, `0x006406F0`, `0x00641DB0` | none for invalidation |
| Normal stock-map preview refresh | verified for invalidation/update handoff | `0x006ADB31`, `0x005E74E0` | full PreviewPack channel order out-of-scope |
| Paint after invalidation | verified | `0x006AE454..0x006AE483`, `0x00640710` | start marker projection formulas out-of-scope |
| Cancel/restore branch | touched-not-exhausted | `0x006AD94C..0x006ADA1E` | non-success branch, out-of-scope |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is the `0x5AA` Choose Map branch active in standard YR? -> Yes, offline Skirmish dialog `0x102` routes `WM_COMMAND` into `0x006ACEE0`, which branches on `0x5AA`.` (evidence: `0x006AE2C0`, `0x006AE3F0`, `0x006AD8E7`)
- `[RESOLVED] OQ-2 - Where does successful modal selection write selected-map globals? -> `0x005E7160` writes `DAT_00A8B250` from selected session `[10]` and `DAT_00A8B254` from the matched list index, then closes the modal dialog.` (evidence: `0x005E7160`, `0x005E6B67`)
- `[RESOLVED] OQ-3 - Does accepted Choose Map restore saved state before preview refresh? -> No; accepted return rebuilds from current selected globals and restores only if `0x005E7BF0` fails.` (evidence: `0x006ADA21..0x006ADB52`)
- `[RESOLVED] OQ-4 - Is preview replacement before repaint invalidation? -> Yes; random path destroys/allocates/loads/fallbacks before `InvalidateRect`, while normal path delegates both load and invalidation to `0x005E74E0`.` (evidence: `0x006ADAC3..0x006ADB1E`, `0x005E74E0`)
- `[RESOLVED] OQ-5 - Does Choose Map paint directly? -> No, it invalidates; `0x006AE3F0` later handles `WM_PAINT` and calls `DrawStartPositions`.` (evidence: `0x006ADB19`, `0x006AE454`, `0x00640710`)
- `[DEFERRED] OQ-6 - Exact PreviewPack channel order in the normal loader.` (category: out-of-scope; reason: assigned to PreviewPack channel-order work; next step: inspect `0x00641B00` with surface pixel format evidence)
- `[DEFERRED] OQ-7 - Full chooser list filter/sort behavior before `0x005E7160`.` (category: out-of-scope; reason: this slot starts at successful Choose Map return effects; next step: full `0x005E6920` dialog investigation)

## Sources

- Fresh Ghidra decompiles/assembly context: `0x006ACEE0`, `0x006AE2C0`, `0x006AE3F0`, `0x005E68A0`, `0x005E7160`, `0x005E74E0`, `0x006406E0`, `0x006406F0`, `0x00641DB0`, `0x0069ADF0`, `0x0069AE70`, `0x00640710`, `0x00641EE0`, `0x00641B00`.
- String evidence: `RandMap.img @ 0x00829ABC`, `RandMap.Sed @ 0x0082BC30`.
- Prior docs checked: `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`, `SKIRMISH_PREVIEW_OBJECT_LIFECYCLE_DAT_00AC1154_GHIDRA_REPORT.md`, `SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md`.
- Rust comparison references: `src/ui/skirmish_shell/state.rs:165`, `src/app.rs:557`, `src/app_skirmish_shell_render.rs:736`, `src/app_skirmish_shell_render.rs:765`.
