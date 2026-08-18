# Skirmish Choose Map Modal Flow - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x005E68A0`, `0x005E6920`, `0x005E7160`, `0x005E74E0`, `0x005E7BF0`, `0x006AE3F0`, `0x00640710`  
**Investigation Mode:** exhaustive-slice scoped to gaps and reconciliation  
**Claimed Scope:** offline Skirmish setup dialog `0x102` Choose Map control `0x5AA`: modal open/return contract, selected-map state update, setup label refresh, preview object replacement/refresh, invalidation/repaint handoff, and dialog continuation.  
**Non-Scope:** full map list population internals, full PreviewPack channel decode, map preview marker projection, random map generation internals, non-offline/WOL variants.  
**Confidence:** High for the offline Skirmish parent flow, modal return split, selected-map commit, selected-record loader, text refresh, preview refresh, and repaint continuation.  
**Active in YR:** Yes. Evidence: standard offline Skirmish creates/pumps dialog `0x102` through `0x006AE2C0`, routes `WM_COMMAND` in `0x006AE3F0`, and reaches the live `0x5AA` branch in `0x006ACEE0`.

## 1. Overview

Retail Yuri's Revenge does not treat `Choose Map` as an in-place next-map button. The setup dialog saves the current selected map/category state, copies the current map path into the display buffer, runs pre-modal shell handling, hides itself, opens a modal Choose Map dialog, then continues the same setup loop after the modal returns.

The modal return contract is:

- result `2`: cancel/back, restore saved selected globals, reload/restored preview state, show setup again, repaint;
- result other than `2`: accepted path, rebuild player/combo state from the newly committed selection, show setup, load selected-record fields, refresh map/mode text, refresh preview, invalidate for repaint;
- accepted load failure: restore old selected globals and return without the normal text/preview invalidation path.

The player-visible implementation consequence is that the setup screen disappears while the chooser is active, then returns with map label, game-type label, preview object, and dependent controls rebuilt from the chooser's selected record. It is not a silent one-click cycle.

## 2. Entry And Modal Lifecycle

`0x006AE3F0` handles `WM_COMMAND` by splitting `wParam`: the low word becomes the command id and the high word becomes the notification before calling `0x006ACEE0`.

- Active in YR: Yes.
- Evidence: `0x006AE432..0x006AE443` masks low word with `0xffff`, shifts high word by `0x10`, and calls `0x006ACEE0`.

For command `0x5AA`, `0x006ACEE0` starts the Choose Map branch by saving the current selected state and current path:

- saves `DAT_00A8B250` into a stack local;
- saves `DAT_00A8B254` in `EBX`;
- copies current selected file path `DAT_00A8B8E0` into a stack buffer, then copies it into `DAT_00A8B322`;
- calls `0x00608070(setup_hwnd)`;
- calls `ShowWindow(setup_hwnd, 0)`;
- calls `0x005E68A0`.

Active in YR: Yes. Evidence: `0x006AD8E7` reads `DAT_00A8B250`, `0x006AD8ED..0x006AD927` copies `DAT_00A8B8E0` through `DAT_00A8B322`, `0x006AD931` calls `0x00608070`, `0x006AD93C..0x006AD93F` hides the setup HWND, and `0x006AD947` calls `0x005E68A0`.

`0x005E68A0` is the modal wrapper. It creates dialog resource `0x6B` with callback `0x005E6920`, stores the chooser HWND in `DAT_00AC0D40`, sends message `0x4A9`, shows the chooser with `ShowWindow(..., 1)`, then runs `0x007759E0(0, 1, 1)` and returns the modal result.

- Active in YR: Yes.
- Evidence: `0x005E68BE` pushes callback `0x005E6920`, `0x005E68C4` calls `0x00775700`, `0x005E68D0` stores `DAT_00AC0D40`, `0x005E68E3..0x005E68E9` sends `0x4A9`, `0x005E68F5..0x005E68F8` shows the chooser, and `0x005E6909..0x005E690F` calls `0x007759E0`.

`0x007757E0` is the modal close helper that writes the result to `DAT_00B72F4C`; `0x007759E0` returns `DAT_00B72F4C` after the modal leaves the stack.

- Active in YR: Yes.
- Evidence: decompile of `0x007757E0` stores `param_2` into `DAT_00B72F4C`; decompile of `0x007759E0` returns `DAT_00B72F4C` once the target modal is gone.

## 3. Modal Return Codes And Selected-Map Commit

The chooser cancel/back command maps to result `2`. In the chooser proc, the `0x5C0` branch calls `0x007757E0(hwnd, 2)`.

- Active in YR: Yes.
- Evidence: `0x005E69D3..0x005E69EC`; specifically `0x005E69E7` loads `EDX = 2` before calling `0x007757E0`.

The chooser accept command maps to result `1`. The `0x6C5` branch calls `0x005E7160`, and that helper closes the modal with result `1`.

- Active in YR: Yes.
- Evidence: `0x005E69C2..0x005E69CD` identifies `0x6C5`, `0x005E6B63..0x005E6B67` calls `0x005E7160`, and `0x005E73A4..0x005E73AD` calls `0x007757E0` with `EDX = 1`.

`0x005E7160` commits selected map state by reading listbox `0x553`, resolving item data back to a scenario record pointer in `DAT_00A8B8CC`, reading the selected mode/category item data from control `0x6EB`, and writing selected globals.

- `DAT_00A8B23C = selected mode/category object`;
- `DAT_00A8B254 = matched scenario index`;
- `DAT_00A8B250 = selected_mode[10]`;
- `DAT_00A8B254 = matched scenario index` is written again after the category-change block.

Active in YR: Yes. Evidence: `0x005E7160` decompile; `0x005E7367` writes `DAT_00A8B23C`, `0x005E7370` writes `DAT_00A8B254`, `0x005E7376` writes `DAT_00A8B250`, and the sibling record/list reports verify `DAT_00A8B8CC[DAT_00A8B254]` is the selected scenario record.

`0x005E7160` also updates chooser/active-dialog text controls before/around close by sending message `0x4B2` to child `0x6EC` for mode/category text and child `0x5A8` for the selected map record pointer.

- Active in YR: Yes.
- Evidence: decompile of `0x005E7160` sends `0x4B2` to `0x6EC` after `FUN_007B7140`, then sends `0x4B2` to `0x5A8` with `DAT_00A8B8CC[DAT_00A8B254]` when the selected index is valid.

## 4. Parent Return Branches

After `0x005E68A0` returns, the parent compares `EAX` with `2`.

- Active in YR: Yes.
- Evidence: `0x006AD94C` compares `EAX, 2`; `0x006AD94F` jumps to the accepted path when not equal.

### Cancel/Restore Result `2`

When return is `2`, the parent restores the saved selected globals:

- `DAT_00A8B250 = saved token`;
- `DAT_00A8B254 = saved scenario index`;
- calls `0x005E7BF0` and `0x005E74E0`;
- shows setup with `ShowWindow(setup_hwnd, 5)`;
- handles random-map preview/invalidation special cases.

Active in YR: Conditional. The branch is active in the standard path only when the modal returns `2`. Evidence: `0x006AD94C..0x006AD978`; `0x006AD95B` restores `DAT_00A8B250`, `0x006AD961` restores `DAT_00A8B254`, `0x006AD967` calls `0x005E7BF0`, `0x006AD96E` calls `0x005E74E0`, and `0x006AD973..0x006AD976` shows the setup HWND.

### Accepted Result

When return is not `2`, the parent does not restore saved selected globals. It rebuilds setup state from the new selection:

1. `0x005E6520(DAT_00A8B254)` computes selected-map capacity.
2. selected mode/category vtable `+0x04` may clamp capacity through `DAT_00A8B230+0x11E4`.
3. `0x004E4FC0`, `0x004E5310`, and `0x004E5D60` rebuild player/combo state.
4. `0x006ADDF0(setup, old_index, DAT_00A8B254)` updates row visibility/control state.
5. `ShowWindow(setup_hwnd, 5)` restores setup before selected-record load.
6. `0x005E7BF0(DAT_00A8B254)` loads selected-record fields.
7. On loader failure, old `DAT_00A8B254` and `DAT_00A8B250` are restored and the branch returns.
8. On success, mode/map labels, dependent controls, and preview are refreshed.

Active in YR: Yes, with conditional clamp/load-failure subpaths. Evidence: accepted branch starts at `0x006ADA21`; `0x006ADA27` calls `0x005E6520`; `0x006ADA36` calls vtable `+0x04`; `0x006ADA6D` calls `0x006ADDF0`; `0x006ADA72..0x006ADA75` shows setup; `0x006ADA7D` calls `0x005E7BF0`; `0x006ADA82..0x006ADB52` tests failure and restores saved globals.

`0x005E7BF0` copies selected record fields into setup globals: display title to `DAT_00A8B322`, digest to `DAT_00A8BAE2`, file/path to `DAT_00A8B8E0`, path to `ScenarioClass+0x125C`, official flag to `DAT_00A8BB08`, player cap/mask to `DAT_00A8BB0C`, and file vtable `+0x2C` result to `DAT_00A8BB04`.

- Active in YR: Yes.
- Evidence: `0x005E7D2C..0x005E7D31` copies record `+0x00` into `DAT_00A8B322`; `0x005E7D87..0x005E7D99` copies record `+0x58` into `DAT_00A8B8E0` and `ScenarioClass+0x125C`; `SKIRMISH_CHOOSE_MAP_RECORD_DECODE_FIELDS_GHIDRA_REPORT.md` verifies the other selected-record fields.

After successful selected-record load, the parent refreshes setup labels:

- `0x005E2EF0` updates control `0x6EC` with current mode/category text;
- `0x005E2F60` updates control `0x5A8` with `DAT_00A8B322`.

Active in YR: Yes. Evidence: `0x006ADA90..0x006ADAB1` calls `0x005D5F30`, `0x005E2EF0`, and `0x005E2F60`; the record decode report verifies the target controls and message `0x4B2`.

## 5. Preview Refresh And Repaint Handoff

The parent does not directly paint the new preview. It refreshes `DAT_00AC1154` or delegates refresh, then invalidates the setup dialog so `WM_PAINT` consumes the new/current preview object.

For random-map records, the accepted path:

1. checks selected record with `0x0069ADF0`;
2. destroys and frees old `DAT_00AC1154` if present;
3. allocates a 4-byte wrapper;
4. zero-initializes it through `0x006406E0`;
5. loads `RandMap.img` through `0x00641DB0`;
6. calls `0x005E74E0(setup)` as fallback if the wrapper's inner pointer remains null;
7. calls `InvalidateRect(setup, NULL, FALSE)`.

Active in YR: Conditional. This subpath is active when the selected record is the random-map sentinel. Evidence: `0x006ADAC3..0x006ADB1E`; `0x006ADB02` calls `0x00641DB0` with `RandMap.img`; `0x006ADB19..0x006ADB1E` invalidates with `erase = FALSE`.

For normal stock-map records, the accepted path calls `0x005E74E0(setup)` and returns. `0x005E74E0` destroys/free-clears old `DAT_00AC1154`, opens `DAT_00A8B8E0`, allocates/initializes a 4-byte wrapper on successful file open, runs the non-random preview load/decode path, and invalidates the setup dialog if a preview wrapper exists.

- Active in YR: Yes for normal stock-map selection.
- Evidence: `0x006ADB31..0x006ADB33` calls `0x005E74E0`; decompile of `0x005E74E0` clears old `DAT_00AC1154`, opens `DAT_00A8B8E0`, allocates via `operator_new(4)`, initializes with `0x006406E0`, and reaches `InvalidateRect(setup, NULL, FALSE)` at `0x005E78FA..0x005E78FD`.

The later `WM_PAINT` path in `0x006AE3F0` checks `DAT_00AC1154`, finds child `0x468`, calls `0x006067A0`, and only if that returns false calls `DrawStartPositions @ 0x00640710`. It then validates the parent rect.

- Active in YR: Yes.
- Evidence: `0x006AE454` reads `DAT_00AC1154`, `0x006AE45D..0x006AE463` gets child `0x468`, `0x006AE46B` calls `0x006067A0`, `0x006AE472..0x006AE47B` conditionally calls `0x00640710`, and `DrawStartPositions` itself validates the parent and early-outs when wrapper field `+0` is null.

## 6. Rust Implementation Gaps

This is a contrast-only implementation note, not binary evidence.

Current Rust has the control identity but not the modal contract:

- `OwnerDrawButton::ChooseMap0x5aa` maps to `SkirmishShellAction::ChooseMap` (`src/ui/skirmish_shell/state.rs:100`).
- `ChooseMap` increments `selected_map_idx = (selected_map_idx + 1) % maps.len()` in-place (`src/ui/skirmish_shell/state.rs:165`).
- app routing swallows `ChooseMap` without a modal screen transition (`src/app.rs:557`).
- preview texture handling is keyed to `selected_map_idx` and rebuilt lazily (`src/app_skirmish_shell_render.rs:736`, `src/app_skirmish_shell_render.rs:765`), but it is not driven by a retail accept/cancel modal flow, selected-record loader failure semantics, or `RandMap.img` branch.

Implementation gaps implied by this report:

- no modal resource/screen lifecycle for Choose Map;
- no accept/cancel return codes `1`/`2`;
- no restore-on-cancel or restore-on-loader-failure semantics;
- no record-pointer/list-item-data selected-map commit;
- no parent setup hide/show ordering around the chooser;
- no text refresh ordering for `0x6EC` and `0x5A8` from selected-record globals;
- no retail preview wrapper replacement path or random-map `RandMap.img` fallback.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | Remaining |
|---|---|---|---|
| Offline Skirmish dialog route | verified | `0x006AE3F0`, `0x006ACEE0` | none for this slice |
| `0x5AA` parent branch | verified | `0x006AD8E7..0x006ADB52` | none |
| Modal create/pump | verified | `0x005E68A0`, `0x007759E0` | full chooser UI layout out of scope |
| Modal cancel result | verified | `0x005E69D3..0x005E69EC` | none |
| Modal accept result | verified | `0x005E6B63..0x005E6B67`, `0x005E7160`, `0x005E73A4..0x005E73AD` | validation prompts out of scope |
| Selected-record load | verified | `0x005E7BF0`; record decode sibling report | none for field copies |
| Setup label refresh | verified | `0x006ADA90..0x006ADAB1`; `0x005E2EF0`, `0x005E2F60` sibling evidence | none |
| Preview wrapper refresh | verified | `0x006ADAC3..0x006ADB38`, `0x005E74E0` | concrete surface vtable/clipping owned by another slot |
| Repaint handoff | verified | `0x006AE454..0x006AE483`, `0x00640710` | marker geometry owned by prior marker report |
| Map list population internals | touched only as dependency | sibling list/report evidence | out of scope by constraint |

## 8. Open Questions

- Exact preview surface vtable/clipping semantics are delegated to `SKIRMISH_PREVIEW_SURFACE_VTABLE_AND_CLIPPING`.
- Full combo/dropdown chooser UI geometry is outside this slot.
- Random-map generation internals after selecting `RandMap.Sed` remain outside this slot; this report only verifies the `RandMap.img` preview branch.
- Non-offline/WOL host/guest variants were not investigated here.

## Sources

- Fresh read-only Ghidra decompiles: `0x006ACEE0`, `0x005E68A0`, `0x005E7160`, `0x005E74E0`, `0x005E7BF0`, `0x006AE3F0`, `0x00640710`, `0x007757E0`, `0x007759E0`.
- Fresh read-only Ghidra assembly context: `0x006AD8E7`, `0x006AD931`, `0x006AD93C`, `0x006AD947`, `0x006AD94C`, `0x006AD95B`, `0x006ADA21`, `0x006ADA72`, `0x006ADA7D`, `0x006ADB19`, `0x006ADB31`, `0x006AE432`, `0x006AE441`, `0x006AE454`, `0x006AE472`, `0x005E68BE`, `0x005E68C4`, `0x005E68E3`, `0x005E68F5`, `0x005E6909`, `0x005E69D3`, `0x005E6B63`, `0x005E7367`, `0x005E73A4`, `0x005E78FA`, `0x005E7D2C`, `0x005E7D87`.
- Prior reports read: `SKIRMISH_CHOOSE_MAP_ACTION_TRACE.md`, `SKIRMISH_CHOOSE_MAP_MODAL_RETURN_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_PREVIEW_INVALIDATION_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_RECORD_DECODE_FIELDS_GHIDRA_REPORT.md`.
- Rust contrast scan: `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs`.
