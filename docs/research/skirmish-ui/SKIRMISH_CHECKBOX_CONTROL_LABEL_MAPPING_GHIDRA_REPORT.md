# Skirmish Checkbox Control Label Mapping - Ghidra Research Report

**Address(es):** `0x006040B0`, `0x006AE6E0`, `0x006ACEE0`, `0x00697F10`, `0x00671EA0`, `0x00665650`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Offline Skirmish dialog `0x102` checkbox controls `0x54E`, `0x69A`, `0x69D`, `0x693`, and `0x696`: visible dialog labels, tooltip/string IDs, init defaults, and Start Game packing destinations.  
**Non-Scope:** In-game Options dialog, runtime effects of the packed options after match launch, non-Skirmish host/guest lobby variants, CSF English text expansion, and unrelated trackbar/row controls.  
**Confidence:** High for control IDs, string IDs, globals, and binary read/write order; Medium for player-facing English names where they are inferred from CSF key names rather than decoded from a CSF file in this slice.  
**Active in YR:** Yes. Evidence: `FUN_006AE2C0` creates dialog resource `0x102` with proc `0x006AE3F0`; `FUN_006AE3F0` routes init to `0x006AE6E0` and `WM_COMMAND` to `0x006ACEE0`.

## 1. Overview

The five lower Skirmish checkboxes are ordinary dialog child `Button` controls with checkbox style `0x50000003`. Their visible labels come directly from PE dialog resource `0x102`; their shell tooltip/string IDs are returned by the active shell tooltip dispatcher `FUN_006040B0` when the parent dialog kind is `0x102`.

The checked state is initialized from session/mirror globals in `FUN_006AE6E0`, but Start Game does not trust cached values. `FUN_006ACEE0` re-reads each child control with `BM_GETCHECK (0xF0)`, stores `checked == 1` into the live Skirmish option globals, mirrors those values into `DAT_00A8B3D8..3DC`, then continues preview teardown and dialog-result handoff.

## 2. Control / Label / Global Map

| Control | Visible resource title | Tooltip/string ID | Init source -> live global | Start Game live write | Start Game mirror write | Active in YR |
|---:|---|---|---|---|---|---|
| `0x54E` | `GUI:ShortGame` | `STT:SkirmishCBoxShortGame` at `0x00835528` | `DAT_00A8B3D8 -> DAT_00A8B262` | `DAT_00A8B262 = (BM_GETCHECK == 1)` | `DAT_00A8B3D8 = DAT_00A8B262` | Yes |
| `0x69A` | `GUI:SuperWeaponsAllowed` | `STT:SkirmishCBoxSWAllowed` at `0x0083550C` | `DAT_00A8B3D9 -> DAT_00A8B263` | `DAT_00A8B263 = (BM_GETCHECK == 1)` | `DAT_00A8B3D9 = DAT_00A8B263` | Yes |
| `0x69D` | `GUI:BuildOffAlly` | `STT:SkirmishCBoxBuildOffAlly` at `0x008354EC` | `DAT_00A8B3DA -> DAT_00A8B264` | `DAT_00A8B264 = (BM_GETCHECK == 1)` | `DAT_00A8B3DA = DAT_00A8B264` | Yes |
| `0x693` | `GUI:MCVRepacks` | `STT:SkirmishCBoxRedeploys` at `0x00835544` | `DAT_00A8B3DB -> DAT_00A8B320` | `DAT_00A8B320 = (BM_GETCHECK == 1)` | `DAT_00A8B3DB = DAT_00A8B320` | Yes |
| `0x696` | `GUI:CratesAppear` | `STT:SkirmishCBoxCrates` at `0x008354D4` | `DAT_00A8B3DC -> DAT_00A8B261` | `DAT_00A8B261 = (BM_GETCHECK == 1)` | `DAT_00A8B3DC = DAT_00A8B261` | Yes |

Resource evidence: PE `RT_DIALOG` id `0x102`, language `0x409`, data RVA `0x007FB1E4`, file offset `0x004FF1E4`, contains these five `#128` Button controls with the listed titles and rectangles. Active in YR: Yes, because the same resource id is created by the standard offline Skirmish path.

Tooltip evidence: `FUN_006040B0` checks parent dialog kind `0x102` and then compares the child control id. Assembly contexts: `0x0060450E..0x00604516` maps `0x54E -> 0x00835528`; `0x0060451E..0x00604526` maps `0x69A -> 0x0083550C`; `0x0060452E..0x00604536` maps `0x69D -> 0x008354EC`; `0x006044FE..0x00604506` maps `0x693 -> 0x00835544`; `0x0060453E..0x00604546` maps `0x696 -> 0x008354D4`. Active in YR: Yes for shell tooltips on dialog `0x102`.

## 3. Core Logic

### 3.1 Initialization

`FUN_006AE6E0` copies the five mirror globals into the live option globals, then sends `BM_SETCHECK (0xF1)` to each checkbox only if `GetDlgItem` returns a non-null child handle. Non-null guarding matters: missing controls leave the copied live globals unchanged but do not crash in this path.

| Control | Init operation | Evidence | Active in YR |
|---:|---|---|---|
| `0x54E` | `DAT_00A8B262 = DAT_00A8B3D8`; send `0xF1` with `DAT_00A8B262 != 0` | `FUN_006AE6E0`, xref read `0x006AEDA0` | Yes |
| `0x69A` | `DAT_00A8B263 = DAT_00A8B3D9`; send `0xF1` with `DAT_00A8B263 != 0` | `FUN_006AE6E0`, xref read `0x006AEDA6` | Yes |
| `0x69D` | `DAT_00A8B264 = DAT_00A8B3DA`; send `0xF1` with `DAT_00A8B264 != 0` | `FUN_006AE6E0`, xref read `0x006AEDAC` | Yes |
| `0x693` | `DAT_00A8B320 = DAT_00A8B3DB`; send `0xF1` with `DAT_00A8B320 != 0` | `FUN_006AE6E0`, xref read `0x006AEDB7` | Yes |
| `0x696` | `DAT_00A8B261 = DAT_00A8B3DC`; send `0xF1` with `DAT_00A8B261 != 0` | `FUN_006AE6E0`, xref read `0x006AEDC3` | Yes |

### 3.2 Click behavior boundary

The checkbox owner-draw/control callback `0x006163A0` owns immediate toggle state, invalidation, click sound, and parent notification. `FUN_006ACEE0` has no command cases for `0x54E`, `0x69A`, `0x69D`, `0x693`, or `0x696`; the option globals are not rewritten on click. They are rewritten only when Start/Back apply reaches the checkbox packing block.

Evidence: prior `SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md` decompiled `0x006163A0`; fresh `FUN_006ACEE0` decompile shows command dispatch returns for these IDs and later reads them only in the Start/Back apply path. Active in YR: Yes.

### 3.3 Start Game packing order

`FUN_006ACEE0` reads controls in this exact order after slider packing:

1. `0x54E` -> `DAT_00A8B262`
2. `0x69A` -> `DAT_00A8B263`
3. `0x69D` -> `DAT_00A8B264`
4. `0x693` -> `DAT_00A8B320`
5. `0x696` -> `DAT_00A8B261`

Each read uses `GetDlgItem`; if the handle is non-null, `SendMessageA(hwnd, 0xF0, 0, 0)` is compared to exactly `1`. Other return values, including `0`, become false. After all reads, mirrors are written in memory order: `DAT_00A8B3D8 = DAT_00A8B262`, `DAT_00A8B3D9 = DAT_00A8B263`, `DAT_00A8B3DA = DAT_00A8B264`, `DAT_00A8B3DB = DAT_00A8B320`, `DAT_00A8B3DC = DAT_00A8B261`.

Evidence: assembly `0x006AD78D..0x006AD889`; specific writes include `0x006AD7BC` (`0x54E` result), `0x006AD7E0` (`0x69A` result), `0x006AD803` (`0x69D` result), `0x006AD827` (`0x693` result), `0x006AD848` (`0x696` result), and mirror writes `0x006AD866..0x006AD889`. Active in YR: Yes.

## 4. INI Keys and Defaults

The Start Game packing block itself does not read INI. The directly verified default path is:

1. `RulesClass::Constructor @ 0x00665650` seeds Rules fields.
2. `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0` reads `[MultiplayerDialogSettings]` fallbacks.
3. `SessionClass__ReadSkirmishSettings @ 0x00697F10` reads persisted `[Skirmish]` keys, falling back to the Rules fields.
4. `FUN_006AE6E0` copies the persisted/mirror checkbox globals into live controls.

| Control | Persisted `[Skirmish]` key | Rules fallback | YR `rulesmd.ini` default evidence | Active in YR |
|---:|---|---|---|---|
| `0x54E` | `ShortGame` | Rules `+0x14B6` | `[MultiplayerDialogSettings] ShortGame=yes`, `ini/rulesmd.ini:3039`; read at `0x00671EA0`, then `0x00697F10` | Yes |
| `0x69A` | `SuperWeaponsAllowed` | Rules `+0x14B9` | not present in supplied `rulesmd.ini`; constructor seeds `+0x14B9 = 1`; read at `0x00671EA0`/`0x00697F10` if present | Yes |
| `0x69D` | `BuildOffAlly` | Rules `+0x14BA` | not present in supplied `rulesmd.ini`; constructor seeds `+0x14BA = 1`; read at `0x00671EA0`/`0x00697F10` if present | Yes |
| `0x693` | `MCVRepacks` | Rules `+0x14B8`, read from Rules key `MCVRedeploys` | `[MultiplayerDialogSettings] MCVRedeploys=yes`, `ini/rulesmd.ini:3041`; read at `0x00671EA0`, then `0x00697F10` uses persisted key `MCVRepacks` | Yes |
| `0x696` | `CratesAppear` | Rules `+0x14B1`, read from Rules key `Crates` | `[MultiplayerDialogSettings] Crates=yes`, `ini/rulesmd.ini:3034`; read at `0x00671EA0`, then `0x00697F10` uses persisted key `CratesAppear` | Yes |

TS legacy boundary: `FogOfWar` is adjacent in the Rules fallback block (`+0x14B7`) and YR default is `FogOfWar=no` (`ini/rulesmd.ini:3040`), but none of the five scoped Skirmish checkboxes maps to `FogOfWar`. Active in YR: No for this checkbox mapping; the key exists but is not one of these controls.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Dialog creation | Standard offline Skirmish creates resource `0x102` and proc `0x006AE3F0` | `FUN_006AE2C0`; prior viewport/layout reports | Yes |
| Tooltip lookup | `FUN_006040B0` resolves tooltip/string IDs by parent dialog kind and child control id | `0x006040B0`, xrefs from `STT:SkirmishCBox*` strings | Yes |
| Checkbox init | `FUN_006AE6E0` sends `BM_SETCHECK` from mirror globals | `0x006AE6E0`, reads `0x006AEDA0..0x006AEDC3` | Yes |
| Start/Back apply | `FUN_006ACEE0` reads `BM_GETCHECK` and writes live + mirror globals | `0x006AD78D..0x006AD889` | Yes |
| Rules/session defaults | Rules fallback and persisted Skirmish keys provide upstream values before dialog init | `0x00665650`, `0x00671EA0`, `0x00697F10` | Yes |

## 6. Current Rust Implementation Status

Rust has partial option storage but not the dialog control mapping. `src/sim/game_options.rs` contains `short_game`, `super_weapons`, `build_off_ally`, `crates`, and `mcv_redeploy`; `src/ui/main_menu.rs` exposes only `short_game` in `SkirmishSettings`; `src/ui/skirmish_shell/state.rs` carries only `short_game` into the experimental shell. The shell layout does not yet define or pack checkbox controls `0x54E`, `0x69A`, `0x69D`, `0x693`, or `0x696`.

Important implementation mismatch: current Rust `GameOptions::default()` has `build_off_ally: false`, while the verified YR Rules constructor fallback seeds `BuildOffAlly` true unless an INI/persisted override changes it. Evidence: `src/sim/game_options.rs:55-65`; Ghidra `0x00665650`; `rulesmd.ini` has no `BuildOffAlly` override in `[MultiplayerDialogSettings]`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| PE dialog `0x102` visible titles | verified | RT_DIALOG `0x102`, RVA `0x007FB1E4`, file offset `0x004FF1E4` | CSF English text expansion out of scope |
| `FUN_006040B0` Skirmish checkbox tooltip IDs | verified | `0x00604506..0x00604546`; strings `0x008354D4..0x00835544` | none |
| `FUN_006AE6E0` checkbox init state | verified | `0x006AE6E0`; xrefs `0x006AEDA0..0x006AEDC3` | upstream persisted mirror writes outside this narrow slice |
| `FUN_006ACEE0` Start checkbox packing | verified | `0x006AD78D..0x006AD889` | runtime consumers after shell exit out of scope |
| `[Skirmish]` persisted key reads | verified | `SessionClass__ReadSkirmishSettings @ 0x00697F10` | physical RA2MD.INI user values not inspected |
| `[MultiplayerDialogSettings]` fallback reads | verified | `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`; `ini/rulesmd.ini:3034-3041` | none for these controls |
| Rules constructor fallback for absent keys | verified | `RulesClass::Constructor @ 0x00665650` | none |
| In-game Options dialog comparison | deferred | user scope excludes it | separate targeted investigation if needed |
| Runtime gameplay effect of each packed option | deferred | out of scope | trace option consumers after match launch |

## 8. Open Questions - Final State

[RESOLVED] OQ1 - Which visible resource label belongs to each scoped checkbox? Listed in section 2. Evidence: PE `RT_DIALOG` `0x102`, RVA `0x007FB1E4`, file offset `0x004FF1E4`.  
[RESOLVED] OQ2 - Which tooltip/string ID belongs to each scoped checkbox? Listed in section 2. Evidence: `FUN_006040B0`, `0x00604506..0x00604546`.  
[RESOLVED] OQ3 - Does Start Game read live controls or cached globals? It reads live child controls with `BM_GETCHECK (0xF0)` when handles exist, then writes globals. Evidence: `0x006AD78D..0x006AD889`.  
[RESOLVED] OQ4 - Are unchecked/non-`1` returns treated differently? No. Each result is stored as `SendMessageA(...) == 1`; any other return becomes false. Evidence: `0x006AD7B6..0x006AD848`.  
[RESOLVED] OQ5 - Are click-time globals updated? No for these option globals; checkbox click state lives in owner-draw control state until Start/Back apply reads it. Evidence: `0x006163A0`, `0x006ACEE0` dispatch shape.  
[RESOLVED] OQ6 - Do scoped controls include Fog of War? No. `FogOfWar` is adjacent in Rules defaults but none of `0x54E`, `0x69A`, `0x69D`, `0x693`, or `0x696` maps to it. Evidence: `0x00671EA0`, `0x00697F10`, section 2 control map.  
[DEFERRED] OQ7 - What exact English text do the `GUI:*` and `STT:*` keys display in all localizations? Category: out-of-scope. Reason: this slice maps binary string IDs, not CSF/localization payloads.  
[DEFERRED] OQ8 - Which post-launch systems consume each packed global? Category: out-of-scope. Reason: this slot ends at Start Game shell packing.

## Sources

- Ghidra decompile/read: `FUN_006040B0`, `FUN_006AE6E0`, `FUN_006ACEE0`, `SessionClass__ReadSkirmishSettings @ 0x00697F10`, `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`, `RulesClass::Constructor @ 0x00665650`, `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`.
- Ghidra string/xref evidence: `STT:SkirmishCBoxCrates @ 0x008354D4`, `STT:SkirmishCBoxBuildOffAlly @ 0x008354EC`, `STT:SkirmishCBoxSWAllowed @ 0x0083550C`, `STT:SkirmishCBoxShortGame @ 0x00835528`, `STT:SkirmishCBoxRedeploys @ 0x00835544`.
- PE resource read: `gamemd.exe` `RT_DIALOG 0x102`, language `0x409`, RVA `0x007FB1E4`, file offset `0x004FF1E4`.
- Prior reports used as context: `SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md`, `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`.
- Rust status scan: `src/sim/game_options.rs`, `src/ui/main_menu.rs`, `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/layout.rs`.
