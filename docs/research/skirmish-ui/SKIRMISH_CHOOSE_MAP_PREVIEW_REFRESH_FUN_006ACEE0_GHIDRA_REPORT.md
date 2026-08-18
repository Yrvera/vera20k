# Skirmish Choose Map Preview Refresh - FUN_006ACEE0 Ghidra Report

**Address(es):** `0x006ACEE0` primary offline Skirmish command handler; related `0x006AE3F0`, `0x006AE2C0`, `0x005E68A0`, `0x005E7160`, `0x005E74E0`, `0x00641DB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** successful `0x5AA` Choose Map command branch and immediate selected-map/preview refresh effects returning to offline Skirmish dialog `0x102`  
**Non-Scope:** full map chooser list behavior, full PreviewPack decode format, random-map generator internals, start-marker projection formulas  
**Confidence:** High for parent branch and repaint sequence; Medium for chooser dialog internals because `0x005E6920` was analyzed from assembly context rather than a decompiler-created function  
**Active in YR:** Yes. Evidence: `0x006AE2C0` creates/pumps Skirmish setup dialog `0x102`; `0x006AE3F0` routes `WM_COMMAND` to `0x006ACEE0`; `0x006ACEE0` has a live `param_2 == 0x5AA` branch. No TS-only gate was found on this UI route.

## 1. Overview

`Choose Map` in `gamemd.exe` is not an in-place next-map button. The Skirmish setup dialog hides itself, runs a modal map-selection dialog, then restores the setup dialog and refreshes text/combo state plus the preview object before invalidating the setup HWND for repaint.

The load-bearing preview behavior is that `DAT_00AC1154` is destroyed before replacement, the replacement is initially a tiny wrapper with a null inner surface, and `RandMap.img` or the selected scenario file path is used to populate the real preview surface. If loading leaves the inner surface null, `0x005E74E0` is the fallback path that attempts to load from the selected map file and invalidates the dialog when a preview exists.

## 2. Key Globals And Controls

| Item | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Dialog `0x102` | Offline Skirmish setup dialog | `0x006AE2C0` creates dialog proc `0x006AE3F0` | Yes - standard Skirmish setup |
| Control `0x5AA` | Choose Map command id | `0x006ACEE0` branch at `0x006AD8E7` from command dispatch | Yes - `WM_COMMAND` route |
| Control `0x468` | Setup dialog preview child | `0x006AE3F0` `WM_PAINT` gets `0x468` before `DrawStartPositions` | Yes - repaint path |
| `DAT_00A8B250` | Selected mode/category token, copied before chooser and restored on failure/cancel branches | `0x006AD8E7`, `0x006AD95B`, `0x006ADB52`; object identity corrected by `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md` | Yes - live branch |
| `DAT_00A8B254` | Selected scenario index | `0x006AD905`, `0x005E7160`, `0x006AD961`, `0x006ADB4B` | Yes - live branch |
| `DAT_00A8B322` | Display map name/text buffer used by setup label `0x5A8` | `0x005E7BF0`, `0x005E2F60` | Yes - live branch |
| `DAT_00A8B8E0` | Selected map file path copied before chooser | `0x006AD8ED..0x006AD927`, `0x005E7BF0` | Yes - live branch |
| `DAT_00AC1154` | Preview-object wrapper pointer used by setup paint | `0x006AE3F0`, `0x006ADAC3..0x006ADB19` | Yes - live branch |
| `RandMap.Sed` | Random map sentinel string | `0x0069ADF0` compares selected record `+0x58` to `RandMap.Sed` | Conditional - random-map branch only |
| `RandMap.img` | Random map preview image file used to fill the new preview wrapper | `0x00641DB0` call at `0x006ADAF6` with string `0x00829ABC` | Conditional - random-map branch only |

## 3. Command Entry And Activity

1. `0x006AE2C0` initializes the Skirmish setup shell, creates dialog `0x102`, stores a result pointer with `SetWindowLongA(hwnd, 8, &local_4)`, shows the dialog, and pumps until `local_4` becomes `0x617` or `0x5C0`.
   - Active in YR: Yes. This is the standard offline Skirmish setup loop.
2. `0x006AE3F0` handles `WM_COMMAND` (`0x111`) by splitting `wParam`: low word goes to `FUN_006ACEE0` as command id and high word goes as notification.
   - Evidence: assembly `0x006AE432..0x006AE443` masks low word with `0xFFFF`, shifts high word by `0x10`, pushes notification/lParam, and calls `0x006ACEE0`.
   - Active in YR: Yes. No game-mode or TS-only gate on this dispatch.
3. `0x006ACEE0` recognizes `0x5AA` and enters the Choose Map branch at `0x006AD8E7`.
   - Active in YR: Yes. It is reached from `WM_COMMAND` for the live setup dialog.

## 4. Successful Choose Map Branch

### Pre-chooser setup

At branch entry `0x006AD8E7`, the handler saves current selected state:

- Reads `DAT_00A8B250` and keeps it in a stack local.
- Reads `DAT_00A8B254` into `EBX`.
- Copies current selected file path `DAT_00A8B8E0` into a stack buffer and then copies that string into `DAT_00A8B322`.
- Calls `0x00608070` before hiding the setup dialog. That helper may disable the window, play a UI sound, mark state byte `+0xC2`, and invalidate while it waits up to 5000 ms for a modal operation. In this branch it is immediately followed by `ShowWindow(setup, 0)`.
- Calls `ShowWindow(setup, 0)`.
- Calls map chooser `0x005E68A0`.

Active in YR: Yes. This is inside the live `0x5AA` branch and uses the current Skirmish setup HWND.

### Chooser return split

`0x005E68A0` creates a modal map dialog from resource `0x6B` with proc entry `0x005E6920`, shows it, pumps via `0x007759E0`, and returns modal result `DAT_00B72F4C`.

The parent branch then splits:

- Return `2`: restore path. `DAT_00A8B250` and `DAT_00A8B254` are restored from the saved values at `0x006AD95B..0x006AD961`. The setup dialog is shown again, preview is refreshed from the restored selection, and the setup HWND is invalidated twice on the random-map preview path.
- Return not `2`: accepted/success path. The handler does not restore the saved selected globals immediately; it rebuilds setup state from the chooser's selected map and only restores saved values if the selected-map load later fails.

Active in YR: Yes. The branch is direct from live `0x5AA`; modal return `2` is also used by map dialog command `0x5C0` at assembly `0x005E69E3..0x005E69EC`.

### Accepted path state rebuild

On non-`2` return, `0x006ACEE0` performs the immediate rebuild in this order:

1. Calls `0x005E6520(DAT_00A8B254)` to compute scenario player/start capacity used for UI visibility.
2. Calls the selected `MPModes` mode/category object's vtable `+0x04`; if it returns true, clamps the capacity against `DAT_00A8B230+0x11E4`.
3. Calls `0x004E4FC0`, `0x004E5310`, and `0x004E5D60` to rebuild per-player combo/list state. In standard offline Skirmish (`g_GameMode == 5`) the `g_GameMode == 3/4` alternate branches inside `0x004E5310` and `0x004E5D60` are not taken.
4. Calls `0x006ADDF0(setup, old_scenario_index, DAT_00A8B254)`, which hides/shows player rows based on map player-count differences and updates combo enablement.
5. Calls `ShowWindow(setup, 5)`.
6. Calls `0x005E7BF0(DAT_00A8B254)`. This loads the selected scenario record into display/path globals, including `DAT_00A8B322`, `DAT_00A8B8E0`, `ScenarioClass+0x125C`, `DAT_00A8BB08`, `DAT_00A8BB0C`, and `DAT_00A8BB04`.
7. If `0x005E7BF0` returns false, restores `DAT_00A8B250` and `DAT_00A8B254` from saved values and returns without repaint invalidation.
8. If load succeeds, refreshes map label controls: `0x005E2EF0` writes a derived string to child `0x6EC`, and `0x005E2F60` writes `DAT_00A8B322` to child `0x5A8`.
9. Calls `0x006ACD60` to update dependent combo/button enabled state.
10. Refreshes preview object and invalidates, described below.

Active in YR: Yes. All steps are on the non-`2` return from the live Choose Map branch. The MP-only branches in `0x004E5310`/`0x004E5D60` are conditional and not active for `g_GameMode == 5`.

## 5. Preview Object Replacement And Repaint

### Random-map detection

The parent branch checks whether the selected scenario record is the random-map sentinel by calling `0x0069ADF0(record_ptr)`. That helper compares `record_ptr + 0x58` against string `RandMap.Sed`.

Active in YR: Conditional. The check runs in standard YR, but the replacement path only applies when the selected record path equals `RandMap.Sed`.

### Random-map preview replacement

If `0x0069ADF0` returns true:

1. Read old `DAT_00AC1154`.
2. If non-null, call `0x006406F0(old)` to destroy its inner surface and set `*old = 0`, then call `0x007C8B3D(old)` to free the wrapper.
3. Allocate 4 bytes.
4. If allocation succeeds, call `0x006406E0` to initialize the wrapper by writing `*wrapper = 0`, then store it in `DAT_00AC1154`.
5. Call `0x00641DB0(DAT_00AC1154, "RandMap.img")`.
6. If `*DAT_00AC1154 == 0`, call `0x005E74E0(setup)` as fallback.
7. Invalidate setup HWND once with `InvalidateRect(setup, NULL, FALSE)`.

Active in YR: Conditional. It is active only when the selected map record is `RandMap.Sed`; the branch itself is live and not TS-gated.

Tiny details that matter:

- The new wrapper is only 4 bytes; it stores a single inner-surface pointer.
- `0x006406E0` does not load anything; it only zeroes that pointer.
- The old wrapper is freed after `0x006406F0`, so holding old `DAT_00AC1154` across selection changes would be stale.
- `InvalidateRect` uses `erase = FALSE`.
- The accepted random-map path invalidates once; the return-`2` restore/random-map path invalidates twice.

### Non-random preview refresh

If `0x0069ADF0` returns false, the accepted path calls `0x005E74E0(setup)` and returns. `0x005E74E0` first destroys any existing `DAT_00AC1154`, then tries to load a preview from the current selected map path `DAT_00A8B8E0`. If it obtains a preview object and `0x0069AE70` says it is not `RandMap.Sed` at record offset `+0x6A8`, it calls a later preview decode/helper path and invalidates the setup HWND.

Active in YR: Yes for normal stock-map selection. The exact PreviewPack decode inside the later helper is out of this slot and belongs to the PreviewPack slot.

### Paint path after invalidation

When the invalidated setup dialog receives `WM_PAINT`, `0x006AE3F0` checks `DAT_00AC1154 != 0`, gets child `0x468`, calls `0x006067A0`, and if that returns false calls `DrawStartPositions @ 0x00640710`. After that it validates the parent rect.

Active in YR: Yes. This is the standard Skirmish setup paint route. The preview object must be non-null for this work to run.

## 6. Current Rust Implementation Status

Rust currently models `Choose Map` as an in-place map-index increment, not as a modal chooser plus accepted/cancel restore path.

| Rust area | Status vs this slice | Evidence |
|---|---|---|
| Button identity | Has `ChooseMap0x5aa` enum and maps hit-test to `SkirmishShellAction::ChooseMap` | `src/ui/skirmish_shell/state.rs:11`, `:100`, `:114` |
| Action behavior | Not equivalent: increments `selected_map_idx = (selected_map_idx + 1) % maps.len()` | `src/ui/skirmish_shell/state.rs:165-167` |
| App action routing | Swallows `ChooseMap` as no screen transition | `src/app.rs:553` |
| Preview surface | Not implemented: active renderer returns no real preview surface | `src/app_skirmish_shell_render.rs:458`, `:590-593` |
| Preview metadata | Records metadata/has pack only; no decoded surface | `src/map/preview.rs:27-46` |
| Source bounds | Intentionally `None` pending verified source | `src/app_list_maps.rs:88-92` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x006AE2C0` setup dialog loop | verified | decompile `0x006AE2C0` | none for this slot |
| `0x006AE3F0` `WM_COMMAND` route | verified | assembly `0x006AE432..0x006AE443`; decompile `0x006AE3F0` | none |
| `0x006ACEE0` `0x5AA` branch | verified | decompile and assembly branch at `0x006AD8E7` | none for parent branch |
| `0x005E68A0` modal chooser wrapper | verified | decompile `0x005E68A0`; modal close helper `0x007759E0` | chooser list internals out of scope |
| Map dialog proc `0x005E6920` accept/cancel | touched-not-exhausted | assembly context `0x005E69E3..0x005E6B51`; helper decompiles | full dialog behavior out of scope |
| `0x005E7160` selected-map accept | verified for selected globals and close | decompile `0x005E7160` | full validation prompts out of scope |
| Accepted parent rebuild | verified | assembly `0x006ADA21..0x006ADB45`; decompile `0x006ACEE0` | none for immediate refresh |
| Restore/cancel parent branch | verified for selected globals and preview refresh | assembly `0x006AD94C..0x006ADA1E` | modal result semantics beyond return `2` out of scope |
| `DAT_00AC1154` destruction/init | verified | `0x006406F0`, `0x006406E0`, branch assembly `0x006ADAC3..0x006ADB19` | concrete surface class internals out of scope |
| Random preview image load | verified | `0x00641DB0`, string `RandMap.img @ 0x00829ABC` | image format details out of scope |
| Non-random preview loader | touched-not-exhausted | `0x005E74E0`, `0x0069AE70` | PreviewPack decode belongs to slot 2 |
| Repaint into setup preview | verified | `0x006AE3F0` `WM_PAINT`; `DrawStartPositions @ 0x00640710` | marker math out of scope |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is `0x5AA` live in standard YR Skirmish? -> Yes, dialog `0x102` routes `WM_COMMAND` low-word `0x5AA` into `0x006ACEE0`.` (evidence: `0x006AE2C0`, `0x006AE3F0`, `0x006ACEE0`)
- `[RESOLVED] OQ-2 - Does Choose Map directly cycle a selected index? -> No. It hides setup, opens modal chooser `0x005E68A0`, then handles chooser return.` (evidence: `0x006AD931..0x006AD94C`)
- `[RESOLVED] OQ-3 - Which chooser return is treated as restore/cancel by parent? -> Return `2` restores saved `DAT_00A8B250/DAT_00A8B254`.` (evidence: `0x006AD94C..0x006AD961`)
- `[RESOLVED] OQ-4 - Where does accepted map selection update globals? -> `0x005E7160` writes `DAT_00A8B23C = selected mode/category object`, `DAT_00A8B250 = selected_mode[10]`, and `DAT_00A8B254 = selected scenario index`; the selected map record is `DAT_00A8B8CC[DAT_00A8B254]`.` (evidence: `0x005E7160`, `SKIRMISH_CHOOSE_MAP_RECORD_DECODE_FIELDS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-5 - Does parent restore saved selection on accepted load failure? -> Yes, if `0x005E7BF0(DAT_00A8B254)` returns false, parent restores saved selected globals and returns.` (evidence: `0x006ADA7D..0x006ADB52`)
- `[RESOLVED] OQ-6 - Does preview object get replaced after accepted selection? -> Yes for random map; old wrapper is destroyed/freed, a 4-byte wrapper is allocated/zeroed, `RandMap.img` is loaded, then parent invalidates.` (evidence: `0x006ADAC3..0x006ADB1E`, `0x00641DB0`)
- `[RESOLVED] OQ-7 - How does non-random map preview refresh? -> Parent calls `0x005E74E0(setup)`, which destroys old preview, loads from selected path, and invalidates if it obtains a preview object.` (evidence: `0x006ADB31..0x006ADB38`, `0x005E74E0`)
- `[RESOLVED] OQ-8 - Does repaint call into the offline preview path after invalidation? -> Yes, `WM_PAINT` checks `DAT_00AC1154`, child `0x468`, then calls `DrawStartPositions`.` (evidence: `0x006AE454..0x006AE483`)
- `[RESOLVED] OQ-9 - What is the full PreviewPack decode format?` PreviewPack text is INI-binary decoded, then LZO-decompressed, then consumed in row-major RGB byte order before packed-color conversion. Evidence: `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`.
- `[RESOLVED] OQ-10 - What are all map chooser list filter/sort rules?` The chooser consumes `DAT_00A8B8CC` in source order, filters by selected mode/GameModes, and stores scenario-record pointers as list item data. Evidence: `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`.

## Sources

- Ghidra decompile: `0x006ACEE0`, `0x006AE3F0`, `0x006AE2C0`, `0x005E68A0`, `0x007759E0`, `0x007757E0`, `0x00608070`, `0x00608260`, `0x005E7160`, `0x005E74E0`, `0x005E7BF0`, `0x006ADDF0`, `0x006ACD60`, `0x006406E0`, `0x006406F0`, `0x00641DB0`, `0x0069ADF0`, `0x0069AE70`, `0x005D5F30`, `0x005E2EF0`, `0x005E2F60`
- Ghidra assembly context: `0x006AE3F0`, `0x006AD8E7`, `0x005E6920`, `0x005E69FD`, `0x005E6B63`, `0x005E6B78`
- Existing trace: `docs/research/traces/SKIRMISH_CHOOSE_MAP_ACTION_TRACE.md`
- Rust scan: `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/map/preview.rs`, `src/app_list_maps.rs`
