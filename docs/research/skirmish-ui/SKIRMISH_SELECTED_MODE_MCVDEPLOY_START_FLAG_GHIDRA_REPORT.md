# Skirmish Selected-Mode MCVDeploy Start Flag -- Ghidra Research Report

**Address(es):** `0x00686990`, `0x005D6D80`, `0x005D7030`, `0x005CAAC0`, `0x005CB440`, `0x006886B0`, `0x004FC060`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Whether standard selected Skirmish MPModes auto-deploy starting MCVs via the `MCVDeploy` start flag. This covers selected-mode `ScenarioClass__Post_Map_Init` dispatch, selected-mode MCV callbacks, stock/custom selected MPModes relevant to offline Skirmish, and the null-mode contrast.  
**Non-Scope:** Full deployment mechanics, full mission scheduler, full `SpecialFlags` parser bit layout, random unit generation beyond the MCVDeploy contrast, UI shell/listbox/combo behavior, random-map generation, loose map loading.  
**Confidence:** High for selected-mode no-auto-deploy; High for null-mode-only `Force_MCV_Deploy` callsite; Medium for exact `[SpecialFlags] MCVDeploy` bit naming because existing docs still conflict with the live null-mode `0x10` test.  
**Active in YR:** Yes for selected-mode dispatch and stock selected modes; No for selected-mode MCVDeploy auto-deploy in the verified callbacks; Conditional for the null-mode generator path when no selected mode object is installed.

## 0. Working Notes

- Target question: Do standard selected Skirmish MPModes ever auto-deploy starting MCVs due to the `MCVDeploy` start flag?
- Non-goals: Do not re-investigate deployment mechanics, UI shell visuals, random-map generation, loose map loading, or full `SpecialFlags` serialization.
- Evidence needed to mark COMPLETE: `Post_Map_Init` selected-vs-null dispatch, selected-mode vtable callback targets, selected MCV callback bodies or raw disassembly when Ghidra lacks a function boundary, global callsite scan for `Force_MCV_Deploy`, and Rust surface scan.
- Stop conditions: Stop once every stock selected-mode MCV callback is checked for `Force_MCV_Deploy` / flag behavior and the null-mode-only callsite is proven; defer only exact special-flag bit naming if it requires a separate parser audit.

## 1. Overview

Selected Skirmish MPModes do not use the null-mode `ScenarioClass__Generate_Random_Units @ 0x006886B0` path. `ScenarioClass__Post_Map_Init @ 0x00686990` dispatches either to `Generate_Random_Units` when `DAT_00A8B23C == null`, or to the selected mode object's `+0x84` callback followed by `FUN_005D6D80` when a selected mode object exists.

The selected-mode MCV creation callback used by stock Battle, ManBattle, FreeForAll, and Cooperative is `0x005D7030`. It checks `Bases`, resolves `[General] BaseUnit`, creates and places the MCV, and has no `MCVDeploy` flag test and no call to `Force_MCV_Deploy @ 0x004FC060`. The only direct call to `0x004FC060` found in the retail binary is from the null-mode generator at `0x00688C02`.

## 2. Class Layout / Key Offsets

| Offset / global | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `DAT_00A8B23C` | selected mode object pointer | Chooses selected-mode dispatch in `Post_Map_Init`; null means use `Generate_Random_Units` | decompile `0x00686990` | Yes / Conditional by selected mode presence |
| selected mode vtable `+0x84` | callback | selected-mode start assignment/pre-start callback called before `FUN_005D6D80` | decompile `0x00686990`; vtable raw bytes | Yes |
| selected mode vtable `+0xC8` | callback | per-house selected-mode MCV/base callback called by `FUN_005D6D80` | decompile `0x005D6D80`; vtable raw bytes | Yes |
| selected mode vtable `+0xCC` | callback | per-house extra starting-unit callback called after `+0xC8` succeeds | decompile `0x005D6D80`; vtable raw bytes | Yes |
| `DAT_00A8B258` | byte | `Bases` option; gates selected-mode standard MCV callback | decompile/disassembly `0x005D7030`; `rulesmd.ini [MultiplayerDialogSettings] Bases=yes` | Yes |
| `RulesClass+0xB20` | vector | `[General] BaseUnit` list used by standard selected MCV callback | decompile/disassembly `0x005D7030`; `rulesmd.ini BaseUnit=AMCV,SMCV,PCV` | Yes |
| `ScenarioClass+0x00` / `DAT_00A8B230` | flags dword | null-mode generator tests bit `0x10` before `Force_MCV_Deploy`; exact relationship to `[SpecialFlags] MCVDeploy` remains cross-doc-conflicted | decompile/disassembly `0x006886B0` / `0x00688BF8`; `SPECIAL_FLAGS_SYSTEM.md` conflict | Conditional |
| `House+0x53DC` | pointer | `Force_MCV_Deploy` stores the MCV/primary pointer here after clearing prior state | decompile/disassembly `0x004FC060` | Conditional, null-mode call only in this slice |
| `Unit+0x81` | byte | `Force_MCV_Deploy` limbo guard; nonzero rejects helper | decompile/disassembly `0x004FC060` | Conditional, null-mode call only in this slice |
| `Unit[0x1B3]` | int-indexed field | helper `0x00740DF0` writes if `-1`, then queues mission `2` through vtable `+0x124` | decompile `0x00740DF0` | Conditional, only reached through `0x004FC060` |

## 3. Core Logic

### Selected vs. null-mode dispatch

`ScenarioClass__Post_Map_Init @ 0x00686990` runs after map load and house creation. If `g_IsMapEditor == 0`, it temporarily clears `g_MapEditorMode` and branches on `DAT_00A8B23C`:

1. `DAT_00A8B23C == null`: call `ScenarioClass__Generate_Random_Units @ 0x006886B0`.
2. `DAT_00A8B23C != null`: call selected mode vtable `+0x84`, then call `FUN_005D6D80`.

Active in YR: Yes. Evidence: decompile `0x00686990`; selected mode objects are loaded from active YR `MPModesMD.ini` per prior `SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`.

### Selected-mode per-house startup driver

`FUN_005D6D80` is selected-mode only in this slice because its caller is the non-null `DAT_00A8B23C` branch. It:

1. Computes an eligible starting-unit budget from `DAT_00A8B270`.
2. Iterates `g_HouseClass_Array`.
3. Skips special houses via `HouseType+0x1A6`.
4. Skips observer-like human node records when matching player-control name and node `+0x6B == -1`.
5. Calls selected mode vtable `+0xC8` with the current house and budget pointer.
6. If `+0xC8` returns true, calls vtable `+0xCC` for extra starting units.

Active in YR: Yes for selected offline Skirmish. Evidence: decompile `0x005D6D80`; caller evidence from `0x00686990`.

### Stock selected-mode MCV callback targets

Raw vtable reads from retail `gamemd.exe` show:

| Mode vtable | Mode | `+0x84` | `+0xC8` MCV/base callback | `+0xCC` extra-units callback | Active in stock offline Skirmish |
|---:|---|---:|---:|---:|---|
| `0x007EE184` | Battle | `0x005D6C70` | `0x005D7030` | `0x005D70F0` | Yes, ids `1`, `9` |
| `0x007EE50C` | ManBattle | `0x005D6C70` | `0x005D7030` | `0x005D70F0` | Yes, ids `5..8` |
| `0x007EE424` | FreeForAll | `0x005D6C70` | `0x005D7030` | `0x005D70F0` | Yes, id `2` |
| `0x007EE27C` | Cooperative | `0x005C2EF0` | `0x005D7030` | `0x005D70F0` | Yes, id `3` |
| `0x007EE814` | Unholy | `0x005D6C70` | `0x005CB440` | `0x005D70F0` | Yes, id `4` |
| `0x007EE6FC` | Siege | `0x005CA800` | `0x005CAAC0` | `0x005D70F0` | No stock row; binary support only |

Active in YR: Yes for listed stock rows except Siege; Siege is Conditional/No for stock offline Skirmish because stock `ini/mpmodesmd.ini` has no `[Siege]` row. Evidence: raw vtable read plus prior mode-construction report and `ini/mpmodesmd.ini`.

### Standard selected MCV callback `0x005D7030`

`0x005D7030` does exactly the standard selected MCV/base work:

1. Read `DAT_00A8B258` (`Bases`).
2. If `Bases == 0`, return success without creating an MCV.
3. Resolve BaseUnit from `RulesClass+0xB20` through `FUN_00505310`.
4. Allocate and construct a `UnitClass`.
5. Get house base coords via `FUN_0050DF30`.
6. Call object vtable `+0xD8` place.
7. If exact placement fails, call `FUN_0050DEF0`, then `FUN_00688ED0` with radius `1`.
8. If fallback fails, delete the MCV through vtable `+0x20`.
9. Return success only when exact or fallback placement succeeds, or `Bases == 0`.

There is no read of `DAT_00A8B230`, no test of `0x10`/`0x100`, and no call to `0x004FC060`.

Active in YR: Yes for Battle, ManBattle, FreeForAll, Cooperative selected modes. Evidence: decompile `0x005D7030`; raw disassembly `0x005D7030..0x005D70E2`; vtable entries above.

### Unholy selected MCV callback `0x005CB440`

Ghidra has no function boundary for `0x005CB440`, so this slice used raw retail binary disassembly. The callback:

1. Iterates the `RulesClass+0xB24/B30` BaseUnit vector backwards.
2. Constructs one unit per BaseUnit entry when non-null.
3. Uses house base via `FUN_0050DEF0`.
4. For the last/first index case it tries an offset of `+3,+3` cells and exact `Place`.
5. Otherwise or on exact failure, calls `FUN_00688ED0` with radius `3`.
6. Deletes failed units and returns success only if no failures were flagged.

It contains no flag read and no `0x004FC060` call.

Active in YR: Yes for selected Unholy Alliance id `4`. Evidence: vtable `0x007EE814 + 0xC8 -> 0x005CB440`; raw disassembly `0x005CB440..0x005CB52F`; stock `ini/mpmodesmd.ini` row `Unholy`.

### Siege selected MCV callback `0x005CAAC0`

Siege is not selectable from stock offline Skirmish, but the binary callback was checked because it is a selected-mode vtable target. Ghidra has no function boundary for `0x005CAAC0`, so this slice used raw retail binary disassembly. The callback:

1. Searches local node records for the current house slot.
2. If no matching node record is found, calls standard `0x005D7030`, then returns false.
3. If node `+0x6B != 1`, delegates to standard `0x005D7030` and returns that result.
4. If node `+0x6B == 1`, resolves a building-like base object from `RulesClass+0x8AC`, constructs it, places it at the house base cell, and falls back through `FUN_00688ED0` radius `1`.

It contains no flag read and no `0x004FC060` call. Its standard branch delegates to `0x005D7030`, which also contains no MCVDeploy behavior.

Active in YR: Conditional/No for stock offline Skirmish. Binary support exists, but stock `ini/mpmodesmd.ini` exposes no Siege row. Evidence: vtable `0x007EE6FC + 0xC8 -> 0x005CAAC0`; raw disassembly `0x005CAAC0..0x005CABC1`; prior mode roster audit.

### Null-mode contrast

`ScenarioClass__Generate_Random_Units @ 0x006886B0` contains the known MCVDeploy-like path. After successful MCV placement, raw disassembly at `0x00688BF2..0x00688C02` loads `DAT_00A8B230`, tests byte bit `0x10`, then calls `Force_MCV_Deploy @ 0x004FC060` with `(house, mcv, 1)`.

A direct callsite scan over retail `gamemd.exe` found:

- calls to `0x004FC060`: `0x00688C02` only
- calls to `0x00740DF0`: `0x004FC095` only
- calls to `0x005D7030`: `0x005CAB03`, `0x005CAB24` only, both from Siege callback delegation

Active in YR: Conditional. This path runs only when `DAT_00A8B23C == null` in `Post_Map_Init`; ordinary selected offline Skirmish installs a selected MPModes object. Evidence: decompile `0x00686990`, decompile `0x006886B0`, raw callsite scan.

## 4. INI Keys

| Key | Location | Default / stock value | Effect in this slice | Evidence | Active in YR |
|---|---|---:|---|---|---|
| `Bases` | `[MultiplayerDialogSettings]` | `yes` in `rulesmd.ini` | Gates standard selected `0x005D7030`; false returns success without creating MCV | `rulesmd.ini:3032`; disassembly `0x005D7030..0x005D7045` | Yes |
| `BaseUnit` | `[General]` | `AMCV,SMCV,PCV` in `rulesmd.ini` | Supplies selected-mode MCV type(s); standard callback uses first side/house match, Unholy iterates the vector | `rulesmd.ini:390`; `0x005D7051..0x005D7059`; `0x005CB440..0x005CB46B` | Yes |
| `MCVDeploy` | `[SpecialFlags]` | default `0` per `SPECIAL_FLAGS_SYSTEM.md`; stock maps/options vary only when supplied | No selected-mode callback checked here reads it or calls `Force_MCV_Deploy`; null-mode generator checks bit `0x10` in `ScenarioClass+0x00` before `0x004FC060` | `0x00688BF2..0x00688C02`; `SPECIAL_FLAGS_SYSTEM.md` conflict notes | No for selected-mode callbacks; Conditional for null-mode |
| `MCVRedeploys` | `[MultiplayerDialogSettings]` | `yes` in `rulesmd.ini` | Not the start auto-deploy flag; Rust currently models this as `mcv_redeploy` | `rulesmd.ini:3041`; Rust scan | Yes for redeploy option, not this behavior |

## 5. Integration Points

Selected Skirmish startup integration:

`ScenarioClass__Post_Map_Init @ 0x00686990` -> selected mode `+0x84` -> `FUN_005D6D80 @ 0x005D6D80` -> selected mode `+0xC8` (`0x005D7030`, `0x005CB440`, or `0x005CAAC0`) -> selected mode `+0xCC` (`0x005D70F0`) if the MCV/base callback succeeds.

Null-mode integration:

`ScenarioClass__Post_Map_Init @ 0x00686990` -> `ScenarioClass__Generate_Random_Units @ 0x006886B0` -> after successful BaseUnit placement, `test byte ptr [ScenarioClass+0], 0x10` -> `Force_MCV_Deploy @ 0x004FC060` -> `0x00740DF0`.

The selected and null-mode paths are mutually exclusive at the `DAT_00A8B23C` branch in `Post_Map_Init`.

## 6. Current Rust Implementation Status

Rust launch-session seeding is in `src/app_skirmish.rs::apply_skirmish_launch_session`. It creates launch houses, applies alliance state, assigns waypoints, spawns MCVs, and sets base center. It does not parse or apply `[SpecialFlags] MCVDeploy` to selected-mode startup.

`src/skirmish_launch.rs` has `mcv_redeploy`, which corresponds to the lobby redeploy option family, not selected-mode MCV auto-deploy.

`src/map/basic.rs::SpecialFlagsSection` parses `TiberiumGrows`, `TiberiumSpreads`, and `DestroyableBridges` only. It does not currently parse `MCVDeploy`.

`src/sim/world/world_commands.rs` handles `Command::DeployMcv`, and `src/sim/world/world_spawn.rs::deploy_mcv` immediately performs the current Rust deploy conversion after checks. This is player/AI deploy behavior, not selected-mode startup auto-deploy.

Rust implication: do not add selected Battle/TeamGame/FreeForAll/Cooperative startup auto-deploy based on null-mode evidence. If a future implementation supports null-mode generator behavior, keep it separate from selected-mode Skirmish launch.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Post_Map_Init` selected-vs-null dispatch | verified | decompile `0x00686990` | none |
| selected-mode `FUN_005D6D80` per-house driver | verified | decompile `0x005D6D80` | exact extra-unit body details outside scope |
| Battle/ManBattle/FreeForAll/Coop `+0xC8 = 0x005D7030` | verified | vtable raw read; decompile/disassembly `0x005D7030` | none |
| Unholy `+0xC8 = 0x005CB440` | verified | vtable raw read; raw disassembly `0x005CB440..0x005CB52F` | none for MCVDeploy question |
| Siege `+0xC8 = 0x005CAAC0` | verified for binary support, not stock exposure | vtable raw read; raw disassembly `0x005CAAC0..0x005CABC1`; no stock `[Siege]` row | none for MCVDeploy question |
| `Generate_Random_Units` MCVDeploy-like call | verified | decompile `0x006886B0`; raw disassembly `0x00688BF2..0x00688C02` | exact flag bit naming remains in SpecialFlags audit territory |
| global direct calls to `Force_MCV_Deploy @ 0x004FC060` | verified | raw retail binary call scan found only `0x00688C02` | indirect calls not found/needed; selected callbacks checked directly |
| `Force_MCV_Deploy @ 0x004FC060` behavior | verified | decompile/disassembly `0x004FC060..0x004FC0AB`; decompile `0x00740DF0` | full mission scheduler after queued mission outside scope |
| Rust selected startup seeding | verified | `src/app_skirmish.rs`, `src/skirmish_launch.rs`, `src/map/basic.rs`, `src/sim/world/world_commands.rs`, `src/sim/world/world_spawn.rs` scan | no implementation performed |
| Exact `[SpecialFlags] MCVDeploy` parser bit layout | deferred | `SPECIAL_FLAGS_SYSTEM.md` conflicts with live null-mode `test byte ptr [ScenarioClass+0], 0x10` | separate SpecialFlags parser/serializer audit |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is this exhaustive or coverage-map? -> Exhaustive-slice for selected-mode MCVDeploy auto-deploy; exact SpecialFlags bit layout is explicitly deferred.` (evidence: scope and coverage ledger)
- `[RESOLVED] OQ-02 - Does selected Skirmish call `Generate_Random_Units`? -> No when `DAT_00A8B23C` is non-null; selected mode `+0x84` and `FUN_005D6D80` are used instead.` (evidence: decompile `0x00686990`)
- `[RESOLVED] OQ-03 - What calls selected-mode MCV/base callbacks? -> `FUN_005D6D80` calls selected mode vtable `+0xC8`, then `+0xCC` if `+0xC8` succeeds.` (evidence: decompile `0x005D6D80`)
- `[RESOLVED] OQ-04 - Which stock modes use standard `0x005D7030`? -> Battle, ManBattle, FreeForAll, and Cooperative; Team Game is a Battle row and uses the Battle vtable.` (evidence: vtable raw read; `ini/mpmodesmd.ini`; mode construction report)
- `[RESOLVED] OQ-05 - Does `0x005D7030` check `MCVDeploy` or call `0x004FC060`? -> No; it checks `Bases`, BaseUnit, exact place, fallback place, and delete-on-failure only.` (evidence: decompile/disassembly `0x005D7030..0x005D70E2`)
- `[RESOLVED] OQ-06 - Does Unholy selected mode auto-deploy via `MCVDeploy`? -> No; `0x005CB440` iterates BaseUnit entries and places units, with no flag read or `0x004FC060` call.` (evidence: vtable `0x007EE814+0xC8`; raw disassembly `0x005CB440..0x005CB52F`)
- `[RESOLVED] OQ-07 - Does Siege selected mode auto-deploy via `MCVDeploy`? -> No in the binary callback checked; Siege is also not stock-selectable offline. Its callback delegates to `0x005D7030` or places a building-like base object, with no `0x004FC060` call.` (evidence: vtable `0x007EE6FC+0xC8`; raw disassembly `0x005CAAC0..0x005CABC1`; `ini/mpmodesmd.ini`)
- `[RESOLVED] OQ-08 - Where is `Force_MCV_Deploy @ 0x004FC060` called directly? -> Only `0x00688C02` in the retail binary direct-call scan.` (evidence: raw binary callsite scan)
- `[RESOLVED] OQ-09 - What is the null-mode MCVDeploy-like check? -> `Generate_Random_Units` tests byte bit `0x10` at `ScenarioClass+0` before calling `0x004FC060`.` (evidence: disassembly `0x00688BF2..0x00688C02`)
- `[RESOLVED] OQ-10 - Is `Force_MCV_Deploy` itself a direct ConYard spawn? -> No; it clears prior state, calls `0x00740DF0`, stores `House+0x53DC`, and returns success/failure.` (evidence: decompile/disassembly `0x004FC060`; decompile `0x00740DF0`)
- `[RESOLVED] OQ-11 - Is Rust currently auto-deploying selected-mode startup MCVs? -> No selected-mode startup auto-deploy field or call found; deploy support exists as player/AI command path.` (evidence: Rust scan paths in section 6)
- `[RESOLVED] OQ-12 - Does `mcv_redeploy` equal `MCVDeploy`? -> No; `mcv_redeploy` models the lobby redeploy option family and must not be used for startup auto-deploy.` (evidence: `rulesmd.ini MCVRedeploys=yes`; Rust scan)
- `[RESOLVED] OQ-13 - Null pointer edge: what if no selected mode object exists? -> The null-mode generator path runs; that is not ordinary selected offline Skirmish once `DAT_00A8B23C` is set.` (evidence: decompile `0x00686990`)
- `[RESOLVED] OQ-14 - Zero Bases edge in selected mode? -> Standard selected `0x005D7030` returns success with no MCV creation when `DAT_00A8B258 == 0`.` (evidence: `0x005D7030..0x005D7045`)
- `[RESOLVED] OQ-15 - Observer/special-house edge in selected mode? -> `FUN_005D6D80` skips special houses and observer-like selected human nodes before `+0xC8`.` (evidence: decompile `0x005D6D80`)
- `[DEFERRED] OQ-16 - Exact `[SpecialFlags] MCVDeploy` bit mapping and why `Generate_Random_Units` tests `0x10` while docs name `0x100`.` (category: requires-different-system-context; reason: this target only needed selected-mode yes/no and null-mode contrast; next-step-if-pursued: re-audit `FUN_006B8CA0` load, `FUN_006B8B30` save, and session flag packing together)
- `[DEFERRED] OQ-17 - Exact scheduler tick when `0x00740DF0` mission `2` is consumed.` (category: out-of-scope; reason: selected-mode callbacks never reach the helper; next-step-if-pursued: targeted deploy mission scheduler trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard selected Skirmish MCV callbacks do not auto-deploy MCVs from `MCVDeploy`; Battle/TeamGame/ManBattle/FreeForAll/Coop `+0xC8` is `0x005D7030`, which has no flag test and no `0x004FC060` call. | `0x00686990`, `0x005D6D80`, vtable raw read, `0x005D7030..0x005D70E2`; direct-call scan `0x004FC060` only from `0x00688C02` | current Rust has no selected-mode auto-deploy; this matches the verified no-op for selected mode | `src/app_skirmish.rs::apply_skirmish_launch_session`, `src/skirmish_launch.rs` | Keep selected-mode Battle-style launch from queuing/deploying MCVs based only on map `[SpecialFlags] MCVDeploy`; selected MCVs should remain undeployed after spawn unless another verified selected path says otherwise. | `skirmish_selected_battle_mcvdeploy_flag_does_not_auto_deploy_starting_mcv`: launch a selected Battle session with map `MCVDeploy=yes` and assert an MCV entity remains, no construction yard is created by startup. | Do not copy null-mode `Generate_Random_Units` behavior into selected Battle/TeamGame. |
| Unholy selected mode uses custom `+0xC8 = 0x005CB440`, but it also does not read the MCVDeploy flag or call `Force_MCV_Deploy`; it places BaseUnit entries with radius-3 fallback. | vtable `0x007EE814+0xC8`; raw disassembly `0x005CB440..0x005CB52F` | unchecked/missing because Rust currently hardcodes Battle-like launch mode | future selected-mode launch model in `src/skirmish_launch.rs` and `src/app_skirmish.rs` | When Unholy support is added, do not add startup auto-deploy; implement its distinct BaseUnit placement separately from the MCVDeploy question. | `skirmish_unholy_startup_does_not_use_mcvdeploy_auto_deploy`: selected Unholy startup with flag set produces placed BaseUnit objects, not forced ConYard deployment. | Do not treat "special mode" as permission to apply null-mode auto-deploy. |
| Siege binary callback `0x005CAAC0` does not auto-deploy, and stock offline Skirmish exposes no Siege row. | vtable `0x007EE6FC+0xC8`; raw disassembly `0x005CAAC0..0x005CABC1`; `ini/mpmodesmd.ini` no `[Siege]` | none for stock roster; future mode loader must avoid exposing Siege from binary category alone | mode roster/parser surface, `src/skirmish_launch.rs` | Do not expose or implement stock Siege startup behavior unless data supplies a Siege row; if implemented for mods, keep no-auto-deploy unless new evidence contradicts this callback. | `stock_mpmodes_roster_omits_siege_and_mcvdeploy_does_not_enable_it`: stock mode parsing has no Siege row; selected-mode MCVDeploy tests never route through Siege. | Do not synthesize a selectable Siege row from vtable support. |
| Null-mode generator is the only direct caller of `Force_MCV_Deploy`; if Rust later implements null-mode/no-selected-mode startup generation, it must be a separate path from selected Skirmish launch. | `0x00686990` null branch; `0x00688BF2..0x00688C02`; direct-call scan to `0x004FC060` | missing/unmodeled; current Rust selected launch should not use it | future null-mode generator, not `apply_skirmish_launch_session` selected path | Model null-mode generator's auto-deploy only behind a verified no-selected-mode scenario path and resolved flag semantics. | `skirmish_null_mode_generator_mcvdeploy_is_separate_from_selected_launch`: constructing a no-selected-mode startup uses the null-mode flag path, while selected Battle does not. | Do not use `src/sim/game_options::mcv_redeploy` as this flag. |

### Stale Docs / Follow-up Docs

- `docs/research/MCVDEPLOY_START_FLAG_AUTO_DEPLOY_GHIDRA_REPORT.md`: replace the top-level Active-in-YR claim with: `Active in YR: Conditional for the null-mode ScenarioClass__Generate_Random_Units path; No for verified selected-mode Skirmish callbacks. Standard selected Battle/TeamGame/ManBattle/FreeForAll/Coop use 0x005D7030, which has no MCVDeploy check or Force_MCV_Deploy call.`
- Same report, Overview: replace "standard skirmish/multiplayer startup" with "null-mode startup generation when `DAT_00A8B23C == null`; ordinary selected MPModes route through selected callbacks instead."
- `docs/research/MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`: replace "In offline/skirmish mode, Generate_Random_Units is called directly" with: "`Generate_Random_Units` is called by `Post_Map_Init` only when no selected MPModes object is installed. Ordinary selected Skirmish routes through selected mode `+0x84` and `FUN_005D6D80`."
- `docs/research/skirmish-ui/SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`: retain the caveat that MCVDeploy belongs to null-mode evidence; this report now closes the selected-mode question as "No for verified stock selected callbacks."

## Negative Facts / Do Not Do

- Do not implement selected Battle/TeamGame startup auto-deploy from the old `MCVDEPLOY_START_FLAG_AUTO_DEPLOY_GHIDRA_REPORT.md`; that report's path is null-mode, not selected-mode.
- Do not use `mcv_redeploy` / `MCVRedeploys` as `MCVDeploy`; they are separate option families.
- Do not expose Siege in stock offline Skirmish just because its vtable exists; stock `MPModesMD.ini` has no Siege row.
- Do not make `MCVDeploy` a direct construction-yard spawn shortcut. Even the null-mode helper calls a unit helper path; selected-mode callbacks do not call it at all.

## Remaining Uncertainty

- The exact parser/serializer bit mapping for `[SpecialFlags] MCVDeploy` remains inconsistent across older docs. Live null-mode disassembly shows `test byte ptr [ScenarioClass+0], 0x10` before `0x004FC060`, while `SPECIAL_FLAGS_SYSTEM.md` names INI `MCVDeploy` as bit `0x100`. This does not change the selected-mode result because selected callbacks do not read either bit or call `0x004FC060`.
- The tick scheduler that consumes the queued mission from `0x00740DF0` was not traced because selected-mode callbacks never reach that helper.

## Sources

- Ghidra decompiled/read-only: `0x00686990`, `0x005D6D80`, `0x005D7030`, `0x006886B0`, `0x004FC060`, `0x00740DF0`.
- Raw retail `gamemd.exe` disassembly/read-only: `0x005D7030..0x005D70E2`, `0x005CAAC0..0x005CABC1`, `0x005CB440..0x005CB52F`, `0x00688BF2..0x00688C02`, `0x004FC060..0x004FC0AB`; direct callsite scan for `0x004FC060`, `0x00740DF0`, `0x005D7030`; vtable entries at `0x007EE184`, `0x007EE50C`, `0x007EE6FC`, `0x007EE814`, `0x007EE424`, `0x007EE27C`.
- Prior docs referenced: `skirmish-ui/SKIRMISH_SPAWN_PLACEMENT_AFTER_ASSIGNED_START_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`, `SPECIAL_FLAGS_SYSTEM.md`, `MCVDEPLOY_START_FLAG_AUTO_DEPLOY_GHIDRA_REPORT.md`, `MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/mpmodesmd.ini`.
- Rust scanned: `src/app_skirmish.rs`, `src/skirmish_launch.rs`, `src/map/basic.rs`, `src/sim/world/world_commands.rs`, `src/sim/world/world_spawn.rs`, `src/sim/ai.rs`.
