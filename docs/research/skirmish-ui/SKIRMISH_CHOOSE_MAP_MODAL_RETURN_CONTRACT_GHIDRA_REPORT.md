# Skirmish Choose Map Modal Return Contract - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x005E68A0`, `0x005E6920`, `0x005E7160`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish `Choose Map` command `0x5AA` from `FUN_006ACEE0`: modal entry, modal result values, accept/cancel behavior, selected-map write/restore sites, and setup dialog hide/show order.  
**Non-Scope:** chooser list population/sort/filter rules, PreviewPack decode, preview object lifecycle, preview invalidation, map preview marker rendering, start-game/session packing.  
**Confidence:** High for the parent call/return contract and accept/cancel result values; Medium for uncreated chooser proc `0x005E6920` as a whole because the function body was verified by assembly context rather than a named Ghidra function.  
**Active in YR:** Yes. Evidence: `Main_Game` calls `FUN_006AE2C0` at `0x0052E168`; `FUN_006AE2C0` creates dialog `0x102` with proc `0x006AE3F0` at `0x006AE31C..0x006AE328`; `FUN_006AE3F0` routes `WM_COMMAND` low-word ids into `FUN_006ACEE0` at `0x006AE432..0x006AE443`; `FUN_006ACEE0` contains the live `0x5AA` branch at `0x006AD8E7`.

## 1. Overview

The retail offline Skirmish `Choose Map` button is a modal dialog handoff, not an in-place map cycle. `FUN_006ACEE0` saves the current selected-map globals, disables/animates the setup dialog via `FUN_00608070`, hides the setup window, enters chooser wrapper `FUN_005E68A0`, then branches on the modal result.

The contract is simple but load-bearing: chooser result `2` means cancel/restore, while chooser result `1` means accepted. Acceptance writes the selected map inside chooser helper `FUN_005E7160` before it closes the modal; the parent then rebuilds setup state from the new globals and only restores the old globals if the selected-map load fails.

## 2. Key Globals And Controls

| Item | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Dialog `0x102` | Offline Skirmish setup dialog | `0x006AE321`, `0x006AE328` | Yes - standard Skirmish launcher |
| Proc `0x006AE3F0` | Setup dialog proc dispatching `WM_COMMAND` | `0x006AE432..0x006AE443` | Yes - installed for dialog `0x102` |
| Control `0x5AA` | Setup `Choose Map` command id | `FUN_006ACEE0` branch at `0x006AD8E7` | Yes - reached through `WM_COMMAND` |
| Dialog resource `0x6B` | Map chooser dialog created by wrapper | `FUN_005E68A0` calls `FUN_00775700(param_1, 0x005E6920, 0)` after setup | Yes - wrapper called from live `0x5AA` branch |
| Chooser proc `0x005E6920` | Modal chooser command proc | `FUN_005E68A0`; assembly starts at `0x005E6920` | Yes - direct callback passed to chooser creation |
| Control `0x5C0` | Chooser cancel/back command | `0x005E69D3..0x005E69EC` maps low-word `0x5C0` to modal result `2` | Yes - modal proc branch |
| Control `0x6C5` | Chooser accept command | `0x005E69C2..0x005E69CD`, `0x005E6B63..0x005E6B67` call `FUN_005E7160` | Yes - modal proc branch |
| `DAT_00A8B250` | Selected map/category/session token | saved at `0x006AD8E7`; written by accept at `0x005E7376`; restored at `0x006AD95B` / `0x006ADB52` | Yes |
| `DAT_00A8B254` | Selected scenario/map index | saved on entry in `EBX`; written by accept at `0x005E7370` and `0x005E7388`; restored at `0x006AD961` / `0x006ADB4B` | Yes |
| `DAT_00A8B23C` | Current selected `MPModes` mode/category object pointer, not the selected scenario/map record | saved/restored inside `FUN_005E7160`; assigned selected chooser object at `0x005E7367`; corrected by `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md` | Yes |
| `DAT_00A8B322` | Setup map display-name buffer | copied from old path before chooser at `0x006AD927`; refreshed by loader at `0x005E7D2C..0x005E7D31` | Yes |
| `DAT_00A8B8E0` | Selected map file path buffer | saved before chooser; refreshed by loader at `0x005E7D87..0x005E7D9E` | Yes |

## 3. Core Contract

### Parent command entry

`FUN_006AE3F0` handles `WM_COMMAND` by taking `LOWORD(wParam)` as the command id and `HIWORD(wParam)` as the notification, then calls `FUN_006ACEE0`. Active in YR: Yes. Evidence: `0x006AE432..0x006AE443`.

For command `0x5AA`, `FUN_006ACEE0`:

1. Saves `DAT_00A8B250` into a stack local and `DAT_00A8B254` into `EBX`.
2. Copies current `DAT_00A8B8E0` into a 512-byte stack buffer, then copies that string into `DAT_00A8B322`.
3. Calls `FUN_00608070(setup_hwnd)`.
4. Calls `ShowWindow(setup_hwnd, 0)`.
5. Calls `FUN_005E68A0` through `0x006AD947`.
6. Compares the returned value with `2` at `0x006AD94C`.

Active in YR: Yes. Evidence: `0x006AD8E7..0x006AD94F`.

### Chooser modal entry

`FUN_005E68A0` is the chooser wrapper reached from the setup branch. It prepares shell/map background state, creates the chooser with callback `0x005E6920`, sends init-like message `0x4A9`, shows the chooser window, then calls `FUN_007759E0(0, 1, 1)` and returns that modal loop's result.

`FUN_007759E0` returns global modal result `DAT_00B72F4C` when the target modal window leaves the modal stack. Active in YR: Yes for this invocation. Evidence: `FUN_005E68A0`, `FUN_007759E0`.

### Cancel return

In the chooser proc, `WM_COMMAND` low-word `0x5C0` reaches `0x005E69E3`, loads the chooser HWND into `ECX`, loads `EDX = 2`, and calls `FUN_007757E0`. `FUN_007757E0` stores its second argument into `DAT_00B72F4C`, so `FUN_005E68A0` returns `2`.

The parent recognizes `2` as cancel at `0x006AD94C`, restores `DAT_00A8B250` at `0x006AD95B`, restores `DAT_00A8B254` at `0x006AD961`, performs the refresh path for the restored selection, then calls `ShowWindow(setup_hwnd, 5)` at `0x006AD973..0x006AD976`.

Active in YR: Yes. Evidence: chooser cancel branch `0x005E69D3..0x005E69EC`; modal close result store in `FUN_007757E0`; parent restore branch `0x006AD94C..0x006AD978`.

### Accept return

In the chooser proc, `WM_COMMAND` low-word `0x6C5` reaches `0x005E6B63` and calls `FUN_005E7160`. Inside `FUN_005E7160`, the selected list item from control `0x553` is mapped through `DAT_00A8B8CC` to a scenario index. The selected mode/category object is read from combo/control `0x6EB`.

The selected-map write sequence is:

1. `DAT_00A8B23C = selected mode/category object` at `0x005E7367`.
2. `DAT_00A8B254 = selected_index` at `0x005E7370`.
3. `DAT_00A8B250 = selected_object[10]` at `0x005E7376`.
4. `DAT_00A8B254 = selected_index` again at `0x005E7388`.
5. object vtable calls at `+0x20`, `+0x24`, `+0x18`, and `+0x7C` run around the writes.
6. `FUN_007757E0(chooser_hwnd, 1)` closes the modal and sets result `1` at `0x005E73A4..0x005E73AD`.

Active in YR: Yes. Evidence: `FUN_005E7160` reached by live chooser accept branch; writes at `0x005E7367..0x005E7388`; result `1` close at `0x005E73A4..0x005E73AD`.

### Parent accept branch

Any chooser result other than `2` enters the accept branch at `0x006ADA21`. For normal accept result `1`, the parent:

1. Computes selected map capacity from current `DAT_00A8B254` via `0x005E6520`.
2. Rebuilds setup combo/player state through `0x004E4FC0`, `0x004E5310`, `0x004E5D60`, and `0x006ADDF0`.
3. Calls `ShowWindow(setup_hwnd, 5)` before loading display/path data.
4. Calls loader `0x005E7BF0(DAT_00A8B254)`.
5. If loader fails, restores old `DAT_00A8B254` at `0x006ADB4B` and old `DAT_00A8B250` at `0x006ADB52`, then returns.
6. If loader succeeds, updates child text controls `0x6EC` and `0x5A8` through `FUN_005E2EF0` and `FUN_005E2F60`.

Active in YR: Yes. Evidence: `0x006ADA21..0x006ADB52`; loader display/path writes at `0x005E7D2C..0x005E7D9E`.

## 4. INI Keys

No INI key is read by the modal return contract itself. Active in YR: Yes for the UI path independent of INI settings. The selected map data comes from already-populated chooser/session records, not direct INI parsing in this slice.

## 5. Integration Points

| Area | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Standard launcher | `Main_Game` reaches `FUN_006AE2C0`; that launcher creates dialog `0x102` with proc `0x006AE3F0` | `0x0052E168`, `0x006AE31C..0x006AE328` | Yes |
| Setup command route | `WM_COMMAND` low/high word split dispatches to `FUN_006ACEE0` | `0x006AE432..0x006AE443` | Yes |
| Modal wrapper | `FUN_005E68A0` is called from `0x006AD947`; other callers exist but were not needed for offline Skirmish contract | xrefs to `FUN_005E68A0` include `0x006AD947` | Yes for this caller |
| Modal close helper | `FUN_007757E0` writes the modal result consumed by `FUN_007759E0` | `FUN_007757E0`, `FUN_007759E0` | Yes |
| Setup hide/show | Parent hides setup before chooser and shows setup after cancel/accept return | `ShowWindow(setup,0)` at `0x006AD93C..0x006AD93F`; `ShowWindow(setup,5)` at `0x006AD973..0x006AD976` and `0x006ADA72..0x006ADA75` | Yes |

## 6. Current Rust Implementation Status

Rust currently has a `ChooseMap0x5aa` control/action identity, but it does not implement the modal contract. The parity shell action cycles the selected map index in place and app routing swallows `ChooseMap` without a modal screen transition. Evidence: `src/ui/skirmish_shell/state.rs:165-167`, `src/app.rs:557`. Active in YR: Yes for the target behavior being compared; this Rust note is implementation status, not binary evidence.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline Skirmish setup reachability | verified | `0x0052E168`, `0x006AE31C..0x006AE328` | none for this slice |
| Setup `WM_COMMAND` to `FUN_006ACEE0` | verified | `0x006AE432..0x006AE443` | none |
| `FUN_006ACEE0` `0x5AA` branch | verified | `0x006AD8E7..0x006AD94F` | preview refresh past return is out-of-scope |
| `FUN_005E68A0` chooser modal wrapper | verified | decompile `FUN_005E68A0` | non-skirmish callers deferred |
| Chooser proc cancel branch | verified | `0x005E69D3..0x005E69EC` | none for result contract |
| Chooser proc accept branch | verified | `0x005E69C2..0x005E69CD`, `0x005E6B63..0x005E6B67`, `FUN_005E7160` | list population/order belongs to slot 2 |
| `FUN_005E7160` selected-map writes | verified | `0x005E7367..0x005E7388` | validation prompts beyond result contract deferred |
| Parent cancel restore | verified | `0x006AD94C..0x006AD961` | none |
| Parent accept load-failure restore | verified | `0x006ADA7D..0x006ADB52` | exact loader failure causes out-of-scope |
| Display/path loader writes | touched-not-exhausted | `0x005E7D2C..0x005E7D9E` | full map record decode/list fields out-of-scope |
| Preview object lifecycle/decode/invalidation | deferred | user hard constraint | owned by other swarm slots |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1` - Is this active standard YR Skirmish behavior? Yes. Evidence: `Main_Game` call `0x0052E168`, dialog `0x102` creation `0x006AE321`, command route `0x006AE443`.
- `[RESOLVED] OQ-2` - Which function enters the chooser? `FUN_005E68A0`, called by `FUN_006ACEE0` at `0x006AD947`. Evidence: Ghidra xref and decompile.
- `[RESOLVED] OQ-3` - What result means cancel? Result `2`; chooser control `0x5C0` calls `FUN_007757E0(hwnd, 2)`, and parent restores only when return equals `2`. Evidence: `0x005E69D3..0x005E69EC`, `0x006AD94C..0x006AD961`.
- `[RESOLVED] OQ-4` - What result means accept? Result `1`; chooser control `0x6C5` calls `FUN_005E7160`, which closes via `FUN_007757E0(hwnd, 1)`. Evidence: `0x005E6B63..0x005E6B67`, `0x005E73A4..0x005E73AD`.
- `[RESOLVED] OQ-5` - Where are selected-map and selected-mode globals written? `FUN_005E7160` writes `DAT_00A8B23C` as the selected mode/category object, `DAT_00A8B254` as the selected scenario index, and `DAT_00A8B250` as the selected mode token at `0x005E7367..0x005E7388`. The selected map record itself is `DAT_00A8B8CC[DAT_00A8B254]`. Evidence: assembly context/decompile plus `SKIRMISH_CHOOSE_MAP_RECORD_DECODE_FIELDS_GHIDRA_REPORT.md`.
- `[RESOLVED] OQ-6` - Does accepted return ever restore the old selected map? Yes, only if `0x005E7BF0(DAT_00A8B254)` fails in the parent accept branch. Evidence: `0x006ADA7D..0x006ADB52`.
- `[RESOLVED] OQ-7` - What is the full chooser list population and order? The modal consumes `DAT_00A8B8CC` in source order, filters by selected mode/GameModes, and uses scenario-record pointers as list item data. Evidence: `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`.
- `[RESOLVED] OQ-8` - What happens inside PreviewPack decode/object lifecycle/preview invalidation? Offline Skirmish preview lifecycle, invalidation, and PreviewPack channel/order are resolved by `SKIRMISH_PREVIEW_DAT_00AC1154_LIFECYCLE_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_PREVIEW_INVALIDATION_GHIDRA_REPORT.md`, and `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`.

## Sources

- Ghidra decompile: `FUN_006ACEE0`, `FUN_006AE3F0`, `FUN_006AE2C0`, `FUN_005E68A0`, `FUN_005E7160`, `FUN_007759E0`, `FUN_007757E0`, `FUN_00608070`, `FUN_005E2EF0`, `FUN_005E2F60`, `FUN_006ADDF0`, `FUN_006ACD60`.
- Ghidra assembly context: `0x0052E168`, `0x006AE317..0x006AE328`, `0x006AE432..0x006AE443`, `0x006AD8E7..0x006ADB52`, `0x005E6920..0x005E69EC`, `0x005E6B63..0x005E6B78`, `0x005E7367..0x005E73AD`, `0x005E7D2C..0x005E7D9E`.
- Existing related report read for duplication/scope separation: `docs/research/SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`.
- Rust status scan: `src/ui/skirmish_shell/state.rs`, `src/app.rs`.
