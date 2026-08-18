# Skirmish Packed Option Global Consumers - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x00686B20`, `0x00687F10`, `0x00686890`, `0x004F86F0`, `0x0050AF10`, `0x00449BC0`, `0x0055E160`  
**Investigation Mode:** exhaustive-slice downgraded to partial  
**Claimed Scope:** immediate post-launch consumers of offline Skirmish Start-branch packed top-level options written around `0x006AD703..0x006AD8A4`: credits, game speed, unit count, Short Game, Super Weapons, Build Off Ally, MCV Repacks, Crates, and forced launch flags.  
**Non-Scope:** deep gameplay formulas, balance effects, full placement formulas, online packet serialization, and UI mapping already covered by prior Skirmish checkbox/trackbar reports.  
**Confidence:** High for all listed writers and first consumers except Build Off Ally, which remains unresolved.  
**Active in YR:** Yes for the offline Skirmish Start packing path and the verified post-launch consumers; Conditional where a consumer is only reached after the related gameplay event.

## 1. Overview

The offline Skirmish Start branch stores the selected trackbar and checkbox state into global session bytes/ints before shell exit. Most first consumers are not the shell itself: scenario initialization, house creation, frame pacing, house update, superweapon enablement, MCV undeploy checks, and post-map crate initialization read those globals later.

The notable correction is that scoped offline Skirmish control `0x54E` writes `DAT_00A8B262`, and the first verified runtime reader treats `DAT_00A8B262` as the multiplayer defeat-mode selector in `HouseClass__Update`. The separate forced `DAT_00A8B31F=0` write participates in scenario SpecialFlags/FogOfWar staging, not this scoped checkbox's runtime Short Game defeat branch.

## 2. Packed Globals And Immediate Consumers

| Option / flag | Start-branch write | First verified consumer | Consumer effect | Active in YR |
|---|---:|---|---|---|
| Game speed | `0x006AD720..0x006AD739`: `DAT_00A8B268 = 6 - TB_GETPOS(0x529)` and `DAT_00A8EB60 = DAT_00A8B268`; mirror `DAT_00A8B3CC` | frame pacing path documented by `DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION`; spot-check `FUN_0055E160 @ 0x0055E160` consumes `DAT_00887350`, which local skirmish setup copies from `DAT_00A8EB60` | stored speed byte becomes local tick wait budget in `GetRadarTimer()` buckets for mode 5 | Yes; standard local Skirmish is `g_GameMode == 5` |
| Credits | `0x006AD746..0x006AD755`: `DAT_00A8B25C = TB_GETPOS(0x511)`; mirror `DAT_00A8B3D0` | `ScenarioClass__Create_Houses @ 0x00687F10`; `HouseClass__Set_Credits_And_Color @ 0x004FCE00` | writes both House `+0x1DC` and spendable balance `+0x30C` | Yes; non-campaign init calls `Create_Houses` |
| Unit count | `0x006AD762..0x006AD771`: `DAT_00A8B270 = TB_GETPOS(0x50C)`; mirror `DAT_00A8B3D4` | Battle post-start/unit-generation helper beginning `0x005D6D80`, called from `ScenarioClass__Post_Map_Init @ 0x00686890` after mode vtable `+0x84` | reads `DAT_00A8B270`, computes a starting-unit value budget, and loops houses to grant units/credits through mode methods | Yes for Battle-style Skirmish; exact unit allocation formula out of scope |
| Short Game checkbox | `0x006AD77E..0x006AD793`: `DAT_00A8B262 = (BM_GETCHECK(0x54E) == 1)`; mirror `DAT_00A8B3D8` | `HouseClass__Update @ 0x004F86F0` defeat branch | if `DAT_00A8B262 == 0`, defeat requires no buildings plus no owned unit/infantry/aircraft/extra counts; if nonzero, defeat uses no owned buildings plus no counted ConYard-style instances | Yes; runs every frame for non-passive MP houses after frame `> 0` |
| Super Weapons Allowed | `0x006AD7A0..0x006AD7B5`: `DAT_00A8B263 = (BM_GETCHECK(0x69A) == 1)`; mirror `DAT_00A8B3D9` | `HouseClass__AI_ManageProduction @ 0x0050AF10` | when a SuperWeaponType has `DisableableFromShell` byte `+0xE7` set and `DAT_00A8B263 == 0`, the house suppresses that superweapon grant/enable path | Yes; reached during house production/superweapon maintenance |
| Build Off Ally | `0x006AD7C2..0x006AD7D7`: `DAT_00A8B264 = (BM_GETCHECK(0x69D) == 1)`; mirror `DAT_00A8B3DA` | not verified in this slot | likely a building placement/base-adjacency consumer, but no verified first reader was found with the available read-only pass | Partial; global is active/written, consumer unresolved |
| MCV Repacks | `0x006AD7E4..0x006AD7F9`: `DAT_00A8B320 = (BM_GETCHECK(0x693) == 1)`; mirror `DAT_00A8B3DB` | `BuildingClass__CanUndeployMCV @ 0x00449BC0` | construction-yard undeploy requires MP mode, player-control owner, `DAT_00A8B320 != 0`, and link field `Building+0x2C0 == 0`; non-ConYard `UndeploysInto` bypasses this option | Conditional; active when player attempts ConYard -> MCV undeploy |
| Crates Appear | `0x006AD806..0x006AD81B`: `DAT_00A8B261 = (BM_GETCHECK(0x696) == 1)`; mirror `DAT_00A8B3DC` | `ScenarioClass__Post_Map_Init @ 0x00686890` | if nonzero, places initial random crates using Rules `+0x1470/+0x1474` bounds and `DAT_00A8B54C` before later crate regen/death-drop paths | Yes when checkbox enabled; default YR rules enable crates |
| Forced bridge flag | `0x006AD895`: `DAT_00A8B260 = 1` | `ScenarioClass__Full_Init @ 0x00686B20` | in MP/non-campaign load, bit 15 of `DAT_00A8E960` is only cleared when `DAT_00A8B260 == 0`; Start forcing `1` leaves bridge-destruction staging enabled | Yes; standard offline Skirmish is non-campaign |
| Forced fog/session bits | `0x006AD88F`, `0x006AD89E`, `0x006AD8A4`: `DAT_00A8B31F=0`, `DAT_00A8B31D=0`, `DAT_00A8B26C=0` | `ScenarioClass__Full_Init @ 0x00686B20` for `DAT_00A8B31F`; no first gameplay reader found for forced `B31D/B26C` in this slot | `DAT_00A8B31F` clears SpecialFlags bit `0x1000`/FogOfWar staging in the non-campaign path; `DAT_00A8B26C` matches prior MultiEngineer parser-only/desupported findings | Conditional; FogOfWar remains off by default in standard YR |

## 3. Consumer Details

### 3.1 Credits

`ScenarioClass__Create_Houses @ 0x00687F10` reads `DAT_00A8B25C` for both human and AI houses when calling `HouseClass__Set_Credits_And_Color`. The callee at `0x004FCE00` writes the same value to House `+0x1DC` and House `+0x30C`. Active in YR: Yes; `ScenarioClass__Full_Init @ 0x00686B20` calls `Create_Houses` in the non-campaign branch used by Skirmish.

### 3.2 Game Speed

The Start branch writes both session speed `DAT_00A8B268` and live speed `DAT_00A8EB60`. Prior timing docs verify the local Skirmish setup copies live speed to `DAT_00887350`; `FUN_0055E160 @ 0x0055E160` then subtracts elapsed `GetRadarTimer()` buckets from `DAT_00887350` and sleeps/spins against the remaining budget. Active in YR: Yes for local Skirmish mode `5`. Deep tick-rate calibration is owned by timing docs.

### 3.3 Unit Count

The standard Battle post-start helper beginning at `0x005D6D80` reads `DAT_00A8B270` immediately (`MOV EAX,[0x00A8B270]`) and exits early if it is `<= 0`. Its nearby assembly computes an average eligible unit cost, multiplies by `DAT_00A8B270`, then loops houses and calls mode vtable methods before awarding resources/spawns. Active in YR: Yes for Battle-style offline Skirmish through `ScenarioClass__Post_Map_Init @ 0x00686890`; exact allocation formula is out of scope.

### 3.4 Short Game

`HouseClass__Update @ 0x004F86F0` is the first verified runtime consumer of scoped offline Skirmish `DAT_00A8B262`. The branch is gated by nonzero game mode, house not already defeated, current frame `> 0`, and non-passive house type. Active in YR: Yes. When the byte is zero, defeat waits until buildings plus owned object totals are zero. When the byte is nonzero, the defeat test is shorter: owned building count `< 1` and no counted ConYard-style instances.

### 3.5 Super Weapons Allowed

`HouseClass__AI_ManageProduction @ 0x0050AF10` iterates a house's superweapon instances and their granting buildings. If the SuperWeaponType byte at `+0xE7` is set and `DAT_00A8B263 == 0`, it forces the grant predicate false before activation/suspend/deactivation logic. Active in YR: Yes; this is ordinary house production/superweapon upkeep.

### 3.6 MCV Repacks

`BuildingClass__CanUndeployMCV @ 0x00449BC0` reads `DAT_00A8B320` only after confirming the building has `UndeploysInto`, is a ConstructionYard-style building (`Type+0x16B9 != 0`), game mode is nonzero, owner exists and is player-controlled, and `Building+0x2C0 == 0`. Active in YR: Conditional; only relevant to ConYard undeploy attempts. Non-ConYard `UndeploysInto` returns true before this option gate.

### 3.7 Crates

`ScenarioClass__Post_Map_Init @ 0x00686890` checks `DAT_00A8B261` soon after unit generation and before late post-map initialization. If enabled, it clamps an initial crate count between Rules `+0x1470` and `+0x1474`, considering `DAT_00A8B54C`, then calls `MapClass__PlaceCrateAtRandomCell` repeatedly. Active in YR: Yes when Crates is enabled; standard YR `[MultiplayerDialogSettings] Crates=yes`.

### 3.8 Forced Launch Flags

The Start branch forces `DAT_00A8B31F=0`, `DAT_00A8B260=1`, `DAT_00A8B31D=0`, and `DAT_00A8B26C=0`. `ScenarioClass__Full_Init @ 0x00686B20` immediately consumes `DAT_00A8B31F` in the non-campaign path to set/clear SpecialFlags bit `0x1000`, and later consumes `DAT_00A8B260`: if it is zero, bit `0x8000` is cleared from `DAT_00A8E960`. Active in YR: Yes for non-campaign Skirmish. `DAT_00A8B31D` and `DAT_00A8B26C` were not tied to a first gameplay reader in this slot; prior engineer docs treat MultiEngineer as parser-only/desupported in standard YR.

## 4. INI Keys

| Key | File / line | Default | Packed field | Active in YR |
|---|---|---:|---|---|
| `[MultiplayerDialogSettings] GameSpeed` | `ini/rulesmd.ini:3026` | `1` | `DAT_00A8B268`, `DAT_00A8EB60` | Yes |
| `MinMoney/Money/MaxMoney/MoneyIncrement` | `ini/rulesmd.ini:3018..3021` | `5000/10000/10000/100` | `DAT_00A8B25C` | Yes |
| `MinUnitCount/UnitCount/MaxUnitCount` | `ini/rulesmd.ini:3022..3024` | `0/10/10` | `DAT_00A8B270` | Yes |
| `Crates` | `ini/rulesmd.ini:3034` | `yes` | `DAT_00A8B261` | Yes |
| `ShortGame` | `ini/rulesmd.ini:3039` | `yes` | `DAT_00A8B262` in scoped offline Skirmish Start | Yes |
| `MCVRedeploys` | `ini/rulesmd.ini:3041` | `yes` | `DAT_00A8B320` | Yes |
| `SuperWeaponsAllowed` | Rules constructor fallback / optional key | constructor seed `yes` in prior docs | `DAT_00A8B263` | Yes |
| `BuildOffAlly` | Rules constructor fallback / optional key | constructor seed `yes` in prior docs | `DAT_00A8B264` | Written; consumer unresolved |
| `FogOfWar` | `ini/rulesmd.ini:3040` | `no` | forced `DAT_00A8B31F=0` affects SpecialFlags bit `0x1000` | Conditional; default off |

## 5. Current Rust Implementation Status

| Rust surface | Current state | Delta |
|---|---|---|
| `src/ui/main_menu.rs` `SkirmishSettings` | stores selected map/countries/credits/start/short_game/zoom only | missing game speed, unit count, super weapons, build off ally, crates, MCV repack, and per-row packed state |
| `src/ui/skirmish_shell/state.rs` | mirrors only a subset into launch settings | does not preserve native top-level option globals or forced flag semantics |
| `src/app_skirmish.rs` | seeds opening MCVs and applies starting credits directly | bypasses native `Create_Houses` credits path, unit-count budget, start table, and option consumers |
| `src/sim/game_options.rs` | has fields for several options | defaults differ for `build_off_ally`; UI launch path does not feed most fields |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Start option packing | verified | `FUN_006ACEE0 @ 0x006ACEE0` | none |
| credits consumer | verified | `0x00687F10`, `0x004FCE00` | none |
| game speed immediate runtime bridge | touched-not-exhausted | Start write plus timing docs; spot-check `0x0055E160` | exact setup write to `DAT_00887350` not re-decompiled in this slot |
| unit count consumer | verified for entry | assembly `0x005D6D80..0x005D6ED6`, caller `0x00686890` | allocation formula out of scope |
| Short Game consumer | verified | `HouseClass__Update @ 0x004F86F0` | exact owned-object totals beyond branch distinction out of scope |
| SuperWeaponsAllowed consumer | verified | `HouseClass__AI_ManageProduction @ 0x0050AF10` | none |
| BuildOffAlly consumer | deferred | writer verified at `0x006AD7C2..0x006AD7D7`; no reader verified | focused building-placement/base-adjacency xref trace |
| MCVRepacks consumer | verified | `BuildingClass__CanUndeployMCV @ 0x00449BC0` | none |
| Crates consumer | verified | `ScenarioClass__Post_Map_Init @ 0x00686890` | regen/death-drop formulas out of scope |
| forced flags | touched-not-exhausted | `0x006AD88F..0x006AD8A4`, `0x00686B20` | `DAT_00A8B31D` first reader not found; `DAT_00A8B26C` relies on prior MultiEngineer audit |

## 7. Open Questions - Final State

[RESOLVED] OQ-1 - Which globals are written by the Start branch? `DAT_00A8B268/EB60/B25C/B270/B262/B263/B264/B320/B261`, mirrors `DAT_00A8B3CC..3DC`, and forced `DAT_00A8B31F/B260/B31D/B26C`. Evidence: `0x006AD703..0x006AD8A4`.

[RESOLVED] OQ-2 - Does credits flow through house creation? Yes, `Create_Houses` passes `DAT_00A8B25C` to `HouseClass__Set_Credits_And_Color`, which writes House `+0x1DC/+0x30C`. Evidence: `0x00687F10`, `0x004FCE00`.

[RESOLVED] OQ-3 - Is `DAT_00A8B262` a live Short Game defeat selector for this path? Yes for the scoped offline Skirmish checkbox path; first verified reader is `HouseClass__Update @ 0x004F86F0`. Evidence: Start branch control `0x54E` and update branch on `DAT_00A8B262`.

[RESOLVED] OQ-4 - Does MCV Repacks affect every `UndeploysInto` building? No. `BuildingClass__CanUndeployMCV @ 0x00449BC0` bypasses `DAT_00A8B320` for non-ConYard `UndeploysInto` buildings and only gates ConYard-style undeploy. Evidence: `0x00449BC0`.

[DEFERRED] OQ-5 - What is the first verified BuildOffAlly reader? Category: requires-different-system-context. Reason: the Start write is verified, but scoped decompilation of placement/can-build candidates did not verify a reader. Next step: targeted xref/data scan for `DAT_00A8B264` in placement/base adjacency code.

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Start Game must carry all top-level options, not just credits/short game | `0x006AD703..0x006AD8A4` | missing/partial | `src/ui/main_menu.rs`, `src/ui/skirmish_shell/state.rs`, launch handoff | persist game speed, credits, unit count, five checkboxes, and forced flags into match setup | changing every Skirmish option affects deterministic match state | do not hardcode defaults after the user changes controls |
| Credits initialize each created house balance through a shared house-creation path | `0x00687F10`, `0x004FCE00` | partial shortcut | `src/app_skirmish.rs`, future scenario init | apply selected credits to every created human/AI house | all enabled players start with selected credits | do not only seed local/two-MCV shortcut houses |
| UnitCount drives starting-unit budget; zero exits early | `0x005D6D80` | missing | scenario spawn/start-unit setup | consume selected unit count before starting-unit generation | UnitCount 0 produces no extra starting units; UnitCount 10 produces normal budget | do not treat it as cosmetic shell text |
| Short Game checkbox is consumed by defeat detection as `DAT_00A8B262` | `0x004F86F0` | partial/unclear | `src/sim/game_options.rs`, defeat logic | switch between long defeat and short defeat condition | toggling Short Game changes when a player loses after last buildings/units are destroyed | do not route scoped checkbox through `DAT_00A8B31F`/FogOfWar semantics |
| Crates option places initial crates during post-map init | `0x00686890` | missing/unchecked | crate system / scenario post-init | gate initial crate placement and later crate behavior on selected option | Crates off starts with no random crates from this path | do not confuse baked map overlays with random crate placement |

## Stale Docs / Follow-up Docs

- `SPECIAL_FLAGS_SYSTEM.md` is misleading for this scoped offline Skirmish Start path if read as "`DAT_00A8B31F` is the ShortGame checkbox." For dialog `0x102`, control `0x54E` writes `DAT_00A8B262`; `DAT_00A8B31F` is forced to `0` by Start and is observed in `ScenarioClass__Full_Init` as SpecialFlags/FogOfWar staging.
- `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md` correctly deferred non-credit option consumers; this report fills most of that gap but leaves BuildOffAlly's first reader open.

## Sources

- Ghidra decompiled/read-only: `FUN_006ACEE0 @ 0x006ACEE0`, `ScenarioClass__Full_Init @ 0x00686B20`, `ScenarioClass__Create_Houses @ 0x00687F10`, `ScenarioClass__Post_Map_Init @ 0x00686890`, `HouseClass__Set_Credits_And_Color @ 0x004FCE00`, `HouseClass__Update @ 0x004F86F0`, `HouseClass__AI_ManageProduction @ 0x0050AF10`, `BuildingClass__CanUndeployMCV @ 0x00449BC0`, `FUN_0055E160 @ 0x0055E160`.
- Ghidra assembly context: standard Battle helper `0x005D6D80..0x005D6ED6` for `DAT_00A8B270`.
- Prior docs checked: `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`, `DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION_GHIDRA_REPORT.md`, `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`, `MCV_DEPLOY_GHIDRA_REPORT.md`, `CRATE_SYSTEM_GHIDRA_REPORT.md`, `SUPERCLASS_SYSTEM_GHIDRA_REPORT.md`, `SPECIAL_FLAGS_SYSTEM.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scan: `src/ui/main_menu.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish.rs`, `src/sim/game_options.rs`.
