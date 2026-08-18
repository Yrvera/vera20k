# PUDLGBG Mode-2 Dialog Inventory - Ghidra Research Report

**Date:** 2026-05-24  
**Address(es):** `0x0060C7D0`, `0x00622820`, `0x00622B50`, `0x00621E90`, `0x0069BBE0`, `0x0072AA40`, `0x0072AFF0`, `0x0072B010`, `0x0072B030`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** mode-2 dialog-background allow-list and the neutral/allied/soviet/yuri theme selector for dialogs whose shell record mode is `2`.  
**Non-Scope:** button text, Start validation failure triggers, modal text wrapping, exact runtime screenshot pixels, and exhaustive proof that every allow-listed dialog id is reachable from a normal player flow.  
**Confidence:** High for the allow-list and theme branch; Medium for per-dialog gameplay reachability because activation of every id was not traced.  
**Active in YR:** Conditional. The code is live in YR shell/common dialog setup; each dialog id draws `PUDLGBG*` only if that id is instantiated through the common owner-draw setup and reaches `WM_PAINT` with record mode `2`.

## 0. Investigation Contract

**Target question:** Which dialog ids can enter the common `PUDLGBG*` mode-2 background path, and what state selects `PUDLGBGN`, `PUDLGBGA`, `PUDLGBGS`, or `PUDLGBGY`?

**Non-goals:** Do not inspect button labels, Start validation failure text, OK-button frames, text wrapping, or full Rust rendering beyond handoff notes.

**Evidence needed to mark COMPLETE:** decompile plus disassembly-range evidence for the mode-2 allow-list, decompile plus disassembly-range evidence for theme selection in `WM_PAINT_Handler`, prior filename/global mapping evidence for `PUDLGBG*` and `DIALOG*`, and a Rust-facing handoff.

**Stop conditions:** Stop after the mode field setters, mode-2 paint selector, and asset role matrix are inventoried. Do not chase every dialog id to its launcher unless a contradiction appears.

## 1. Overview

`PUDLGBGN/A/S/Y.SHP` are not random unused art. They are the common mode-2 dialog background family. Dialog records whose mode field is set to `2` are painted by `WM_PAINT_Handler @ 0x00621E90`, which fills a scratch surface and then draws exactly one `PUDLGBG*` SHP frame `0`.

The mode-2 dialog-id allow-list is independent from the theme choice. First, setup functions mark specific dialog ids as mode `2`. Later, on paint, `FUN_0069BBE0()` and `ScenarioClass+0x34B8` choose the neutral/allied/soviet/yuri background and matching DIALOG-family palette.

## 2. Class Layout / Key Offsets

| Field / global | Type | Meaning | Active in YR | Evidence |
|---|---|---|---|---|
| shell record `+0x6C` | int | dialog/resource id stored by setup | Yes | `FUN_0060D2C0 @ 0x0060D2C0`; read by `0x0060C7D0`, `0x00622820` |
| shell record `+0xB0` / `piVar9[0x2C]` | int | parent paint mode; value `2` means `PUDLGBG*` mode-2 background | Yes, conditional on allow-listed dialog ids | `0x0060C7D0`; `0x00622820`; read by `0x00621E90` |
| `ScenarioClass+0x30D8` | byte | scenario-active / no-game branch gate read by `FUN_0069BBE0` | Yes | `FUN_0069BBE0 @ 0x0069BBE0`; prior shell/loading docs |
| `ScenarioClass+0x34B8` | int | side selector in the mode-2 paint branch: `0`, `1`, otherwise Yuri/third-side | Yes when scenario-active | `0x00621E90`; prior `LOAD_PROGRESS_MANAGER_SETUP_GHIDRA_REPORT.md` |
| `DAT_00B0FC80` | SHP pointer | `PUDLGBGN.SHP` | Yes, conditional no-game/menu | `0x0072AA40`; prior pointer extraction |
| `DAT_00B0FC84` | SHP pointer | `PUDLGBGA.SHP` | Conditional side `0` | `0x0072AA40`; `0x00621E90` |
| `DAT_00B0FC88` | SHP pointer | `PUDLGBGS.SHP` | Conditional side `1` | `0x0072AA40`; `0x00621E90` |
| `DAT_00B0FC8C` | SHP pointer | `PUDLGBGY.SHP` | Conditional side other than `0/1` | `0x0072AA40`; `0x00621E90` |
| `DAT_00B0FB68` | ConvertClass | `DIALOG.PAL` convert, used by Allied and Soviet branches | Conditional side `0/1` | `0x0072AFF0`; `0x00621E90` |
| `DAT_00B0FB70` | ConvertClass | `DIALOGY.PAL` convert | Conditional side other than `0/1` | `0x0072B010`; `0x00621E90` |
| `DAT_00B0FB60` | ConvertClass | `DIALOGN.PAL` convert | Conditional no-game/menu | `0x0072B030`; `0x00621E90` |

## 3. Core Logic

### 3.1 Mode-2 Allow-List Setter

`FUN_0060C7D0 @ 0x0060C7D0` looks up the shell/window record for the current dialog, reads the stored dialog id from `record+0x6C`, and writes `record+0xB0 = 2` only when the id is in this allow-list:

| Dialog id | Decimal | Mode-2 eligible? | Active in YR | Evidence |
|---:|---:|---|---|---|
| `0x10D` | 269 | Yes | Conditional: if this RT_DIALOG is instantiated through common setup | `0x0060C7D0` decompile; disassembly range `0x0060C7D0..0x0060C93F` |
| `0x0D9` | 217 | Yes | Conditional | same |
| `0x0F0` | 240 | Yes | Conditional | same |
| `0x0CE` | 206 | Yes | Yes for ordinary Skirmish Start validation; conditional generally | same; `VALIDATION_MODAL_*` reports |
| `0x120` | 288 | Yes | Conditional optional generic-message variant | same; `0x005D3490` optional dialog path |
| `0x121` | 289 | Yes | Conditional optional generic-message variant | same; `0x005D3490` optional dialog path |
| `0x115` | 277 | Yes | Conditional | same |
| `0x0D3` | 211 | Yes | Conditional | same |
| `0x0CF` | 207 | Yes | Conditional | same |
| `0x11F` | 287 | Yes | Conditional | same |
| `0x0C3` | 195 | Yes | Conditional | same |
| `0x11B` | 283 | Yes | Conditional | same |
| `0x0E1` | 225 | Yes | Conditional | same |
| `0x11E` | 286 | Yes | Conditional | same |
| `0x0C4` | 196 | Yes | Conditional | same |
| `0x130` | 304 | Yes | Conditional | same |
| `0x0D0` | 208 | Yes | Conditional | same |
| `0x0FC` | 252 | Yes | Conditional | same |
| `0x126` | 294 | Yes | Conditional | same |

Tiny but important details:

- Active in YR: Yes for the setter itself. `FUN_00622B50` calls `FUN_0060C7D0` during `WM_INITDIALOG` when the dialog is not in the fullscreen-shell set. Evidence: decompile `0x00622B50`; `0x0060C7D0` decompile.
- The allow-list writes a numeric mode value, not an asset pointer. Asset selection is deferred until paint.
- The allow-list does not include main Skirmish setup dialog `0x102` or choose-map dialog `0x6B`; those belong to the common shell/right-panel mode, not this `PUDLGBG*` modal-background path.
- Dialog `0xCE` is in the list, which is why the ordinary Start validation popup can draw `PUDLGBGN/A/S/Y` depending on global state.
- `0x130` is present in the fresh `0x0060C7D0` decompile and should be included in future inventories; some narrow validation reports did not need to mention it.

### 3.2 Mirrored Setup Helper

`FUN_00622820 @ 0x00622820` has the same mode-2 allow-list in its late setup path and writes `record+0xB0 = 2` for the same ids.

Active in YR: Yes, conditional. Prior docs show `FUN_00622820` is a common shell setup helper used by shell dialog construction flows; this report verifies the same mode-2 id set is present there. Evidence: decompile `0x00622820`; disassembly range `0x00622A80..0x00622B2F`.

### 3.3 Theme Selection In Paint

`WM_PAINT_Handler @ 0x00621E90` only enters the `PUDLGBG*` branch when the record mode is exactly `2`. It then selects SHP and palette like this:

| Condition | SHP | Palette / ConvertClass | Active in YR | Evidence |
|---|---|---|---|---|
| `FUN_0069BBE0() == 0` | `DAT_00B0FC80` = `PUDLGBGN.SHP` | `FUN_0072B030()` = `DIALOGN.PAL` | Conditional no-game/menu/shell state; yes for ordinary offline shell validation | `0x00621E90` decompile; disassembly `0x006221D0..0x006222BF`; `0x0069BBE0` |
| `FUN_0069BBE0() != 0` and `ScenarioClass+0x34B8 == 0` | `DAT_00B0FC84` = `PUDLGBGA.SHP` | `FUN_0072AFF0()` = `DIALOG.PAL` | Conditional in-game side `0` | same |
| `FUN_0069BBE0() != 0` and `ScenarioClass+0x34B8 == 1` | `DAT_00B0FC88` = `PUDLGBGS.SHP` | `FUN_0072AFF0()` = `DIALOG.PAL` | Conditional in-game side `1` | same |
| `FUN_0069BBE0() != 0` and side is neither `0` nor `1` | `DAT_00B0FC8C` = `PUDLGBGY.SHP` | `FUN_0072B010()` = `DIALOGY.PAL` | Conditional Yuri/third-side branch | same |

Tiny but important details:

- Active in YR: Yes, conditional on record mode `2`; the branch is live and previously verified for loading/progress and validation modal contexts.
- Allied and Soviet share `DIALOG.PAL`; they differ by SHP art only. Do not map Soviet to `DIALOGY.PAL`.
- The no-game branch keeps the initially loaded `PUDLGBGN` pointer and only changes the palette through `FUN_0072B030`.
- The in-game side selector treats every value other than `0` and `1` as the Yuri/third-side branch.
- The draw call is guarded: if either selected SHP or selected ConvertClass is null, `CC_Draw_Shape` is skipped. This follows from `if ((iVar8 != 0) && (iVar10 != 0))`.

### 3.4 Draw Operation

Inside the mode-2 branch, `0x00621E90`:

1. Defines a destination rect from the dialog client/scratch-surface bounds.
2. Clears or copies the current DirectDraw surface region into the scratch `BSurface`.
3. Selects the `PUDLGBG*` SHP and DIALOG-family ConvertClass.
4. Calls `CC_Draw_Shape(selected_shp, frame=0, src=(0,0), dst=client rect, flags=0x400, z/depth=1000, ...)`.
5. Blits the scratch `BSurface` back to the current display surface.

Active in YR: Conditional on mode `2`, but yes for all allow-listed dialog ids that reach paint. Evidence: decompile `0x00621E90`; disassembly `0x006221D0..0x006222BF`; prior `LOADING_SCREEN_WM_PAINT_MODE2_COMPOSITION_GHIDRA_REPORT.md`.

## 4. INI Keys

No INI keys are read in the scoped mode-2 allow-list or theme selector.

| INI key | Default / section | Effect | Active in YR | Evidence |
|---|---|---|---|---|
| none | n/a | Dialog id allow-list is hardcoded in `gamemd.exe`; theme selection reads scenario state, not INI | Yes | `0x0060C7D0`, `0x00621E90` |

## 5. Integration Points

| Integration point | Role | Active in YR | Evidence |
|---|---|---|---|
| `FUN_00622B50 @ 0x00622B50` | Common dialog proc; on `WM_INITDIALOG`, initializes owner-draw records and calls `FUN_0060C7D0` in the non-fullscreen-shell path | Yes for common shell dialogs | decompile `0x00622B50` |
| `FUN_00622820 @ 0x00622820` | Common setup helper with mirrored mode-2 allow-list | Conditional | decompile `0x00622820` |
| `WM_PAINT_Handler @ 0x00621E90` | Reads mode field and draws `PUDLGBG*` when mode is `2` | Conditional; yes for allow-listed dialogs that repaint | decompile `0x00621E90` |
| `FUN_0069BBE0 @ 0x0069BBE0` | Reads `ScenarioClass+0x30D8`; zero selects no-game/menu background | Yes | decompile `0x0069BBE0`; prior shell docs |
| `0x0072AA40` startup loader | Preloads `PUDLGBG*` SHPs and DIALOG-family palettes | Yes | decompile `0x0072AA40`; prior lifecycle reports |

## 6. Current Rust Implementation Status

This slot did not modify Rust. Focused scan found current Rust has explicit modal assets for the recent validation-modal work:

- `src/render/skirmish_shell_chrome.rs` now has `MNBTTN` and a requested Soviet `PUDLGBGS.SHP + DIALOG.PAL` validation background path.
- `src/app_skirmish_shell_render/modals.rs` renders the validation modal with that background by request.

Current Rust delta from native mode-2 inventory:

- Missing a generalized mode-2 dialog-background resolver keyed by dialog id and scenario/shell state.
- The validation modal currently applies Soviet `PUDLGBGS + DIALOG.PAL` in the shell/no-game Start validation context by user request; native parity for that context is neutral `PUDLGBGN + DIALOGN.PAL`.
- No Rust surface was found for the full 19-id mode-2 allow-list.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Mode-2 allow-list in `FUN_0060C7D0` | verified | decompile `0x0060C7D0`; disassembly `0x0060C7D0..0x0060C93F` | Per-id activation callers remain separate |
| Mirrored allow-list in `FUN_00622820` | verified | decompile `0x00622820`; disassembly `0x00622A80..0x00622B2F` | Exact caller inventory for this helper |
| `FUN_00622B50` integration | verified | decompile `0x00622B50` | None for setup sequencing |
| `WM_PAINT_Handler` mode-2 branch | verified | decompile `0x00621E90`; disassembly `0x006221D0..0x006222BF` | Native screenshot/RGB capture |
| Theme selector no-game/allied/soviet/yuri | verified | `0x00621E90`, `0x0069BBE0`, accessors `0x0072AFF0/B010/B030` | Exact side writes outside loading already covered by sibling docs |
| Asset filename/global mapping | verified by prior docs | `PUDLGBG_LOADING_SCREEN_SHP_LIFECYCLE_GHIDRA_REPORT.md`, `DIALOG_PALETTE_STARTUP_0072AA40_GHIDRA_REPORT.md` | none for this inventory |
| Every allow-listed dialog's player-facing launcher | deferred | not in this slot | separate per-dialog activation inventory |
| Rust comparison | touched-not-exhausted | focused `rg` scan | full UI renderer audit if implementing all mode-2 dialogs |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which function writes mode `2` for dialog backgrounds? -> `FUN_0060C7D0` writes shell record `+0xB0 = 2` for a hardcoded dialog-id allow-list.` (evidence: `0x0060C7D0`; disassembly `0x0060C7D0..0x0060C93F`)
- `[RESOLVED] OQ-02 - Is there a mirrored setup path? -> Yes, `FUN_00622820` contains the same mode-2 id set and write.` (evidence: `0x00622820`; disassembly `0x00622A80..0x00622B2F`)
- `[RESOLVED] OQ-03 - What dialog IDs are in the mode-2 allow-list? -> `0x10D,0xD9,0xF0,0xCE,0x120,0x121,0x115,0xD3,0xCF,0x11F,0xC3,0x11B,0xE1,0x11E,0xC4,0x130,0xD0,0xFC,0x126`.` (evidence: `0x0060C7D0`)
- `[RESOLVED] OQ-04 - Is the allow-list asset-specific? -> No; it only sets mode `2`; asset choice happens later in `0x00621E90`.` (evidence: `0x0060C7D0`, `0x00621E90`)
- `[RESOLVED] OQ-05 - What selects neutral vs side-themed art? -> `FUN_0069BBE0()==0` selects neutral; otherwise `ScenarioClass+0x34B8` selects side branch.` (evidence: `0x00621E90`, `0x0069BBE0`)
- `[RESOLVED] OQ-06 - Does Soviet use a distinct palette? -> No; Soviet side `1` uses `PUDLGBGS.SHP` with `DIALOG.PAL`, same palette accessor as Allied.` (evidence: `0x00621E90`, `0x0072AFF0`)
- `[RESOLVED] OQ-07 - Is `PUDLGBGY` selected only for side exactly `2`? -> No; it is the fallback for any in-game side value other than `0` or `1`.` (evidence: `0x00621E90`)
- `[RESOLVED] OQ-08 - Does this prove ordinary Skirmish Start validation should be Soviet-themed? -> No; ordinary shell validation uses no-game branch `PUDLGBGN + DIALOGN`; Soviet is a valid in-game branch or deliberate override.` (evidence: `0x00621E90`; `SKIRMISH_START_VALIDATION_MODAL_ACTIVATION_RECHECK_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-09 - Are `PUDLGBG*` loaded lazily by paint? -> No; startup loader `0x0072AA40` preloads them; paint reads globals.` (evidence: `PUDLGBG_LOADING_SCREEN_SHP_LIFECYCLE_GHIDRA_REPORT.md`; `0x00621E90`)
- `[RESOLVED] OQ-10 - Are INI keys involved in the allow-list/theme selector? -> No scoped INI reads occur.` (evidence: decompile `0x0060C7D0`, `0x00621E90`)
- `[DEFERRED] OQ-11 - Which exact game UI flow instantiates every listed id?` (category: `out-of-scope`; reason: this slot inventories the mode-2 allow-list, not every dialog launcher; next-step-if-pursued: run per-id xref/resource activation pass)
- `[DEFERRED] OQ-12 - Final RGB screenshot/pixel rects for each theme.` (category: `needs-runtime-debugger`; reason: binary branch and assets are known, but runtime capture was not part of this slot; next-step-if-pursued: capture native mode-2 dialogs in shell and in-game side contexts)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `FUN_0060C7D0 @ 0x0060C7D0` | dialog id in 19-id allow-list | none | n/a | n/a | Conditional | writes mode `2` |
| 2 | `WM_PAINT_Handler @ 0x00621E90` | record mode `+0xB0 == 2` | scratch `BSurface` | client/scratch rect | current display format | Conditional | paint staging |
| 3 | `WM_PAINT_Handler @ 0x00621E90` | `FUN_0069BBE0()==0` | `PUDLGBGN.SHP#0` | source `(0,0)`, destination client rect | `DIALOGN.PAL` via `0x0072B030` | Conditional no-game/menu | neutral modal background |
| 3 | `WM_PAINT_Handler @ 0x00621E90` | scenario-active and side `0` | `PUDLGBGA.SHP#0` | same | `DIALOG.PAL` via `0x0072AFF0` | Conditional side `0` | Allied modal background |
| 3 | `WM_PAINT_Handler @ 0x00621E90` | scenario-active and side `1` | `PUDLGBGS.SHP#0` | same | `DIALOG.PAL` via `0x0072AFF0` | Conditional side `1` | Soviet modal background |
| 3 | `WM_PAINT_Handler @ 0x00621E90` | scenario-active and side not `0/1` | `PUDLGBGY.SHP#0` | same | `DIALOGY.PAL` via `0x0072B010` | Conditional third-side/Yuri | Yuri modal background |
| 4 | `WM_PAINT_Handler @ 0x00621E90` | after optional shape draw | scratch `BSurface` | back to current display rect | current display format | Conditional | present/blit |

## 10. Asset Role Matrix

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `PUDLGBGN.SHP` | Yes | Conditional | Yes in no-game/menu mode-2 dialogs | No | Yes | No | No | Inactive in in-game side branches | `0x0072AA40`; `0x00621E90` |
| `PUDLGBGA.SHP` | Yes | Conditional | Yes when scenario-active side `0` | No | Yes | No | No | Inactive for no-game/Soviet/Yuri | `0x0072AA40`; `0x00621E90` |
| `PUDLGBGS.SHP` | Yes | Conditional | Yes when scenario-active side `1` | No | Yes | No | No | Inactive for no-game/Allied/Yuri | `0x0072AA40`; `0x00621E90` |
| `PUDLGBGY.SHP` | Yes | Conditional | Yes when scenario-active side is not `0/1` | No | Yes | No | No | Inactive for no-game/Allied/Soviet | `0x0072AA40`; `0x00621E90` |
| `DIALOGN.PAL` | Yes | Palette input | Yes in no-game/menu mode-2 dialogs | No | Palette | No | No | Inactive in in-game side branches | `0x0072B030`; `0x00621E90` |
| `DIALOG.PAL` | Yes | Palette input | Yes for side `0` and side `1` | No | Palette | No | No | Inactive for no-game/Yuri | `0x0072AFF0`; `0x00621E90` |
| `DIALOGY.PAL` | Yes | Palette input | Yes for side not `0/1` | No | Palette | No | No | Inactive for no-game/Allied/Soviet | `0x0072B010`; `0x00621E90` |

## 11. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Mode-2 eligible dialog ids are hardcoded as `0x10D,0xD9,0xF0,0xCE,0x120,0x121,0x115,0xD3,0xCF,0x11F,0xC3,0x11B,0xE1,0x11E,0xC4,0x130,0xD0,0xFC,0x126` | `0x0060C7D0`; `0x00622820` | missing generalized mode-2 dialog-id inventory | future UI/dialog chrome resolver; validation modal is one consumer | If a future Rust dialog maps to one of these native ids, it should be eligible for `PUDLGBG*` mode-2 background, not generic shell panel art | Regression fixture for id `0xCE` and a second non-validation id uses the same mode-2 resolver | Do not infer from asset names or selected faction; id/mode controls eligibility |
| Shell/no-game mode-2 branch uses neutral `PUDLGBGN.SHP + DIALOGN.PAL` | `0x00621E90`, `0x0069BBE0`, `0x0072B030` | current validation modal uses user-requested Soviet override | `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render/modals.rs` | Preserve clear distinction between native parity mode and requested Soviet skin override | Invalid Start in offline Skirmish native-parity mode shows neutral background | Do not document Soviet validation as parity-correct for shell/no-game |
| In-game side `0` and side `1` use different SHPs but the same `DIALOG.PAL` | `0x00621E90`, `0x0072AFF0` | unchecked outside validation override | future in-game modal/background renderer | Allied uses `PUDLGBGA + DIALOG.PAL`; Soviet uses `PUDLGBGS + DIALOG.PAL` | Palette snapshot for Soviet does not use `DIALOGY.PAL` | Do not map Soviet to Yuri palette |
| In-game side values other than `0/1` route to `PUDLGBGY.SHP + DIALOGY.PAL` | `0x00621E90`, `0x0072B010` | unchecked | future in-game modal/background renderer | Treat side `2` and any non-0/1 side value as Yuri/third-side branch for this paint path | Yuri-side mode-2 dialog decodes through `DIALOGY.PAL` | Do not clamp unknown side values back to neutral |

## 12. Stale Docs / Follow-up Docs

- Narrow validation reports are still correct for `0xCE`, but future summaries should not imply `0xCE/0x120/0x121` are the entire mode-2 dialog universe. Replacement wording: "The complete mode-2 allow-list currently verified is `0x10D,0xD9,0xF0,0xCE,0x120,0x121,0x115,0xD3,0xCF,0x11F,0xC3,0x11B,0xE1,0x11E,0xC4,0x130,0xD0,0xFC,0x126`; the validation modal uses the `0xCE` member of that list."
- Current Rust notes should label the Soviet validation background as a requested style override. Replacement wording: "Native shell/no-game mode-2 validation uses `PUDLGBGN + DIALOGN`; Rust may deliberately use `PUDLGBGS + DIALOG.PAL` only as a non-parity Soviet skin."

## Sources

- Ghidra read-only decompile: `FUN_0060C7D0 @ 0x0060C7D0`, `FUN_00622820 @ 0x00622820`, `FUN_00622B50 @ 0x00622B50`, `WM_PAINT_Handler @ 0x00621E90`, `FUN_0069BBE0 @ 0x0069BBE0`, `FUN_0072AA40 @ 0x0072AA40`, `FUN_0072AFF0 @ 0x0072AFF0`, `FUN_0072B010 @ 0x0072B010`, `FUN_0072B030 @ 0x0072B030`.
- Ghidra read-only disassembly ranges checked: `0x0060C7D0..0x0060C93F`, `0x00622A80..0x00622B2F`, `0x006221D0..0x006222BF`, `0x0072AA40..0x0072AAFF`.
- Prior docs referenced: `PUDLGBG_LOADING_SCREEN_SHP_LIFECYCLE_GHIDRA_REPORT.md`, `DIALOG_PALETTE_STARTUP_0072AA40_GHIDRA_REPORT.md`, `LOADING_SCREEN_WM_PAINT_MODE2_COMPOSITION_GHIDRA_REPORT.md`, `skirmish-ui/VALIDATION_MODAL_0X005D3490_PAINT_COMPOSITION_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_START_VALIDATION_MODAL_ACTIVATION_RECHECK_GHIDRA_REPORT.md`, `LOAD_PROGRESS_MANAGER_SETUP_GHIDRA_REPORT.md`.
- Current Rust focused scan: `rg "PUDLGBG|DIALOGN|DIALOGY|PUDLGBGN|PUDLGBGA|PUDLGBGS|PUDLGBGY|dialog background|validation modal" src`.

**Status:** COMPLETE for the requested mode-2 allow-list and theme-selector inventory; per-dialog activation flows are explicitly deferred.
