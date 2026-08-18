# Skirmish MPModes Object Construction Defaults - Ghidra Research Report

**Address(es):** `0x005D7590`, `0x005D7CE0`, `0x005D5B60`, `0x005E7160`, `0x005D6130`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish `MPModesMD.ini` mode roster, selected mode object construction/default fields, selected-object assignment to `DAT_00A8B23C`, and mode-object callback contract needed before Start acceptance.  
**Non-Scope:** post-shell unit spawning, full Cooperative mission payload parsing, WOL lobby behavior, map-list record construction except where selected-mode filtering/selection requires it, and full Siege role UI.  
**Confidence:** High for constructor fields, vtable bindings, selected-object assignment, stock exposed roster, and stock override `[MultiplayerDialogSettings]` keys found in retail `ra2md.mix`; Medium for assigning each contiguous override payload to its hashed MIX filename because the filenames are not stored beside the payload text in the MIX body.  
**Active in YR:** Yes for Battle/ManBattle/FreeForAll/Unholy/Cooperative exposed by stock `MPModesMD.ini`; Conditional/No for Siege in offline stock Skirmish because the binary registers the category and `ra2md.mix` contains a Siege override text payload, but stock `MPModesMD.ini` has no `[Siege]` roster entry.

## 0. Working Notes

- Target question: How are offline Skirmish MPModes objects constructed/loaded, which stock modes/defaults are exposed, and how is `DAT_00A8B23C` selected?
- Non-goals: Do not trace gameplay spawn internals, full shell UI, full online lobby behavior, or implement Rust.
- Evidence needed to mark COMPLETE: decompile plus assembly for loader/constructor/selection, vtable bytes for concrete mode classes, INI/default source plus binary reader address for stock keys, and Rust surface scan.
- Stop conditions: Stop after selected-object contract and stock defaults are pinned; defer only filename-to-payload attribution details that require a dedicated MIX-name extraction pass.

## 1. Overview

YR loads `MPModesMD.ini`, registers six binary categories, and constructs one mode object for each row present under those categories. The constructor receives the five comma fields from the roster row plus the numeric key/id, sets common defaults, reads the mode override INI's `[MultiplayerDialogSettings]`, then the derived constructor swaps in the concrete mode vtable.

For ordinary offline Skirmish, the visible selected mode comes from combo/control `0x6EB`. `FUN_005E7160` reads that combo's item data, temporarily tests it, and on accepted change writes it into `DAT_00A8B23C`; Start later dispatches `DAT_00A8B23C` vtable `+0x14`.

## 2. Class Layout / Key Offsets

| Offset/global | Meaning | Verified behavior | Evidence | Active in YR |
|---|---|---|---|---|
| mode `+0x00` | vtable | Base constructor sets `vtable__MultiplayerGameMode`, derived constructors replace it | `0x005D5BE1`, `0x005C0DF8`, `0x005C6178`, `0x005CA658`, `0x005CB3C8`, `0x005C5D08`, `0x005C1540` | Yes |
| mode `+0x20` | display/name string | Used when adding mode rows to combo `0x6EB` | `0x005D626E..0x005D62A0`, `0x005E73F8..0x005E7413` | Yes |
| mode `+0x28` | numeric mode id/key | Copied from roster key; selection writes it to `DAT_00A8B250` | `0x005D5BB8..0x005D5BBD`, `0x005E7363..0x005E7376` | Yes |
| mode `+0x2C` | override filename string | Passed to file open/INI load during constructor | `0x005D5BBD..0x005D5BD1`, `0x005D5C2D..0x005D5C57` | Yes |
| mode `+0x30` | map filter string | Used by map filter via selected object | prior verified `0x005D6419`, roster column 4 | Yes |
| mode `+0x34` | random maps allowed byte | Set from roster column 5 | `0x005D5BD2..0x005D5BD9`; `0x005D7590` parses fifth comma token | Yes |
| mode `+0x3C` | `AlliesAllowed` | Default `1`, then read from override `[MultiplayerDialogSettings]` | `0x005D5BF2`, `0x005D5CDF..0x005D5CEA` | Yes |
| mode `+0x3D` | `WonlineTournamentAllowed` | Default `1`, then override read | `0x005D5BEA`, `0x005D5CA7..0x005D5CB5` | Conditional online; field still constructed in YR |
| mode `+0x3E` | `WonlineClanTournamentAllowed` | Default `1`, then override read | `0x005D5BEE`, `0x005D5CC4..0x005D5CCF` | Conditional online; field still constructed in YR |
| mode `+0x3F` | `MustAlly` | Default `0`, then override read; if `MustAlly=1` but `AlliesAllowed=0`, it is forced back to `0` | `0x005D5BF6`, `0x005D5CF7..0x005D5D11` | Yes for Team Game |
| `DAT_00A8B23C` | selected MPModes object | Temporarily set while validating selection, restored for rejection prompt, then committed to selected combo item data | `0x005E71E5..0x005E721F`, `0x005E7285..0x005E7297`, `0x005E7349..0x005E737D` | Yes |

## 3. Core Logic

### Loader and factory registration

`0x005D7CE0` clears the existing dynamic vector, then registers categories in this order: `Battle`, `ManBattle`, `Siege`, `Unholy`, `FreeForAll`, `Cooperative`. For each category it builds a temporary string and calls `0x005D7590` with a category-specific initializer/factory object.

Active in YR: Yes. Evidence: assembly `0x005D7D3C..0x005D7E36`; string `MPModesMD.ini` at `0x00830A18` is opened in `0x005D7590`.

`0x005D7590` opens `MPModesMD.ini`, enumerates sections matching each registered category, parses each row as five comma-separated fields after the numeric key, and calls the category factory. It inserts created mode objects into the global dynamic vector sorted by `mode+0x28` ascending. Active in YR: Yes. Evidence: decompile `0x005D7590`; insert compare at `0x005D7B25..0x005D7C14`.

### Constructor/default contract

All six exposed concrete constructors call common constructor `0x005D5B60` with the roster row fields and id, then replace the vtable:

| Category | Factory create | Concrete constructor | Object size | Vtable | `+0x14` Start accept slot | Active in stock offline Skirmish |
|---|---:|---:|---:|---:|---:|---|
| Battle | `0x005D8170` | `0x005C0DD0` | `0x40` | `0x007EE184` | `0x005D6310` | Yes, ids `1`, `9` |
| ManBattle | `0x005D81B0` | `0x005C6150` | `0x40` | `0x007EE50C` | `0x005D6310` | Yes, ids `5..8` |
| Siege | `0x005D81F0` | `0x005CA630` | `0x40` | `0x007EE6FC` | `0x005CA6D0` | No stock roster entry |
| Unholy | `0x005D8230` | `0x005CB3A0` | `0x40` | `0x007EE814` | `0x005CB400` | Yes, id `4` |
| FreeForAll | `0x005D8270` | `0x005C5CE0` | `0x40` | `0x007EE424` | `0x005C5D40` | Yes, id `2` |
| Cooperative | `0x005D82B0` | `0x005C1470` | `0x344` | `0x007EE27C` | `0x005C1D80` | Yes, id `3` |

Common constructor details, Active in YR: Yes:

- It sets common defaults before reading override INI: `AlliesAllowed=1`, `WonlineTournamentAllowed=1`, `WonlineClanTournamentAllowed=1`, `MustAlly=0`. Evidence: `0x005D5BEA..0x005D5BF6`.
- It opens the override filename from roster column 3 by constructing a `CCFileClass` from `mode+0x2C`; if file load succeeds, it reads `[MultiplayerDialogSettings]` keys. Evidence: `0x005D5C2D..0x005D5C57`, section pointer `PTR_s_MultiplayerDialogSettings_007EED18`.
- Read order is `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`, then `MustAlly`. Evidence: `0x005D5C9D..0x005D5D00`.
- `MustAlly` cannot survive if `AlliesAllowed` is false: after reading `MustAlly`, the constructor checks `MustAlly != 0 && AlliesAllowed == 0` and writes `MustAlly=0`. Evidence: `0x005D5D05..0x005D5D11`.

### Default callbacks relevant to shell controls

Base/default callbacks are shared by Battle, ManBattle, FreeForAll, Unholy, and Siege except where the vtable table above differs:

| Vtable slot | Base function | Behavior | Evidence | Active in YR |
|---:|---:|---|---|---|
| `+0x14` | `0x005D6310` | default Start acceptance returns `1` | decompile `0x005D6310` | Yes for Battle/ManBattle |
| `+0x28` | `0x005D5DB0` | fallback country/side value returns `-2` when combo data is outside `[-3,9]` | bytes `B8 FE FF FF FF C3`; caller `0x004E4170` | Yes |
| `+0x2C` | `0x005D5DC0` | Team combo minimum/default: returns `-2` when `MustAlly=0`, otherwise `0` | bytes `8A 41 3F ... 83 C0 FE C3`; prior team insertion caller | Yes |
| `+0x30` | `0x005D5DD0` | allies helper returns `3` when `AlliesAllowed=1`, otherwise `-2` | bytes at `0x005D5DD0`; field `+0x3C` | Yes/Conditional |
| `+0x34` | `0x005D5DE0` | proposed team validator accepts `-2` only when `MustAlly=0`; accepts `0..3`; rejects other values | bytes at `0x005D5DE0` | Yes |
| `+0x40` | category-specific displayability | `0x005D6130` calls it only when `g_GameMode == 5` before adding a mode to offline Skirmish combo | `0x005D625B..0x005D626C` | Yes for offline Skirmish |
| `+0x98` | player-count cap adjustment | `0x005E7160` calls it and clamps selected map capacity if return is not `-1` and less than map capacity | `0x005E7241..0x005E7260` | Yes |
| `+0xBC` | role/mode capability byte | Called in list population and selection validation; result is fed to mode/game-mode checks | `0x005D6239..0x005D6259`, `0x005E722A..0x005E723C` | Conditional by category |

`0x004E4170` is the concrete country/default consumer: it reads combo item data and, if the item-data value is outside `[-3,9]`, calls `DAT_00A8B23C` vtable `+0x28`; if no selected mode exists, it falls back to `-2`. Active in YR: Yes. Evidence: decompile/disassembly `0x004E4170..0x004E41C3`.

### Selected-object assignment

`FUN_005E7160` uses two controls:

- `0x553`: selected map/category list. It reads current selection with `CB_GETCURSEL (0x188)`, item data with `CB_GETITEMDATA (0x199)`, then finds the matching record in `DAT_00A8B8CC[0..DAT_00A8B8D8)`. Evidence: `0x005E7163..0x005E71D2`.
- `0x6EB`: selected MPModes combo. It reads current selection and item data; the item data is a mode object pointer. Evidence: `0x005E71E5..0x005E7219`.

The function temporarily writes `DAT_00A8B23C = selected_mode`, calls the selected mode `+0x40`, `+0xBC`, optional `+0x98`, and game-mode checks, then restores the old value before showing any rejection prompt. Only after rejection checks pass does it commit:

- If old selected mode differs, call old mode `+0x9C` when non-null.
- Write `DAT_00A8B250 = selected_mode+0x28`.
- Write `DAT_00A8B23C = selected_mode`.
- Write `DAT_00A8B254 = selected map index`.
- Call new mode `+0x20`.
- Then call selected mode `+0x24`, `+0x18`, `+0x7C`, update shell controls/text, and call `+0x60`.

Active in YR: Yes for offline Skirmish shell selection. Evidence: assembly `0x005E7219..0x005E7260`, restore at `0x005E7285..0x005E7297`, commit at `0x005E7349..0x005E7382`, post-commit calls `0x005E738F..0x005E7452`.

## 4. INI Keys

### Stock exposed roster

Evidence: `ini/mpmodesmd.ini` and retail `ra2md.mix` text payload at `rg -a` lines `111261..111286`; binary reader `0x005D7590`; category registration `0x005D7CE0`.

| Category | id | UI name | tooltip | override file | map filter | random maps | Active in stock YR offline Skirmish |
|---|---:|---|---|---|---|---|---|
| Battle | 1 | `GUI:Battle` | `STT:ModeBattle` | `MPBattleMD.ini` | `standard` | `true` | Yes |
| Battle | 9 | `GUI:TeamGame` | `STT:ModeTeamGame` | `MPTeamMD.ini` | `teamgame` | `false` | Yes |
| ManBattle | 5 | `GUI:Megawealth` | `STT:ModeMegawealth` | `MPMWMD.ini` | `megawealth` | `false` | Yes |
| ManBattle | 6 | `GUI:Duel` | `STT:ModeDuel` | `MPDuelMD.ini` | `duel` | `false` | Yes |
| ManBattle | 7 | `GUI:MeatGrind` | `STT:ModeMeatGrind` | `MPMeatMD.ini` | `meatgrind` | `false` | Yes |
| ManBattle | 8 | `GUI:NavalWar` | `STT:ModeNavalWar` | `MPNavalMD.ini` | `navalwar` | `false` | Yes |
| FreeForAll | 2 | `GUI:FreeForAll` | `STT:ModeFreeForAll` | `MPFreeForAllMD.ini` | `standard` | `true` | Yes |
| Unholy | 4 | `GUI:UnholyAlliance` | `STT:ModeUnholyAlliance` | `MPUnholyMD.ini` | `standard` | `false` | Yes |
| Cooperative | 3 | `GUI:Cooperative` | `STT:ModeCooperative` | `MPCoopMD.ini` | `cooperative` | `false` | Yes |
| Siege | none | none | none | stock text payload exists | none in roster | none | No stock `MPModesMD.ini` entry |

### Stock override `[MultiplayerDialogSettings]` values visible in retail `ra2md.mix`

Evidence: binary constructor reads these exact keys at `0x005D5C9D..0x005D5D11`; retail `rg -a` over `ra2md.mix` shows payload text around lines `110945..111641`.

| Mode payload comment | Override keys observed | Effect on constructed object | Active in stock offline Skirmish |
|---|---|---|---|
| `Mode == Battle` | `AlliesAllowed=yes` | `+0x3C=1`, `+0x3F=0` unless another key says otherwise | Yes, id 1 |
| `Mode == Team Game` | `AlliesAllowed=yes`, `AllyChangeAllowed=no`, `MustAlly=yes` | `+0x3C=1`, `+0x3F=1`; Team `None` suppressed and `-2` team value rejected by callbacks | Yes, id 9 |
| `Mode == Mega Wealth` | `AlliesAllowed=yes` | `+0x3C=1`, `+0x3F=0` | Yes, id 5 |
| `Mode == Duel` | `AlliesAllowed=yes` | `+0x3C=1`, `+0x3F=0` | Yes, id 6 |
| `Mode == Meat-Grinder` | `AlliesAllowed=yes` | `+0x3C=1`, `+0x3F=0` | Yes, id 7 |
| `Mode == Naval` | `AlliesAllowed=yes` | `+0x3C=1`, `+0x3F=0` | Yes, id 8 |
| `Mode == Free For All` | `AlliesAllowed=no`, `WonlineClanTournamentAllowed=no` | `+0x3C=0`, `+0x3F=0`; allies helper returns `-2` | Yes, id 2 |
| `Mode == Unholy Alliance` | `AlliesAllowed=yes` | `+0x3C=1`, `+0x3F=0` | Yes, id 4 |
| `Mode == Cooperative Campaign` | `AlliesAllowed=no`, `WonlineTournamentAllowed=no` | `+0x3C=0`, `+0x3F=0` | Yes, id 3 |
| `Mode == Siege` | no `[MultiplayerDialogSettings]` block found before the next payload boundary in the text scan | no stock exposed selectable object because no roster entry | No stock offline selection |

`AllyChangeAllowed` is present in Team Game's override, but this slice did not find it read by `0x005D5B60`; it belongs to broader rules/session settings, not the common mode object's fields verified here.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Startup/load | `0x005D7CE0` registers categories and drives `0x005D7590` over `MPModesMD.ini` | `0x005D7CE0`, `0x005D7590` | Yes |
| Offline mode combo population | `0x005D6130` clears a combo, iterates mode objects, gates on `g_GameMode == 5` and vtable `+0x40`, adds display text, and stores mode pointer as item data | `0x005D6130..0x005D62BB` | Yes |
| Mode selection | `0x005E7160` reads selected mode pointer from control `0x6EB` and commits `DAT_00A8B23C` plus `DAT_00A8B250/254` | `0x005E71E5..0x005E7382` | Yes |
| Start acceptance | Start handler calls selected `DAT_00A8B23C` vtable `+0x14`; default Battle/ManBattle returns true, other categories override | `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, vtable bytes above | Yes/Conditional by selected mode |
| Country fallback | invalid/out-of-range country combo data calls selected mode vtable `+0x28`; base returns `-2` | `0x004E4170`, bytes at `0x005D5DB0` | Yes |
| Team default/validation | team controls rely on `+0x2C`, `+0x30`, `+0x34` callbacks and object `MustAlly`/`AlliesAllowed` bytes | bytes `0x005D5DC0..0x005D5E08`; prior Team None report | Yes |

No simulation tick-cycle integration occurs in this slice. These paths are synchronous shell/message-loop code before launch handoff.

## 6. Current Rust Implementation Status

Current Rust has a single hardcoded launch mode:

- `src/skirmish_launch.rs:14` defines `SkirmishLaunchMode` with only `Battle`.
- `src/ui/skirmish_shell/state.rs:391..392` builds `SkirmishLaunchSession { mode: SkirmishLaunchMode::Battle, ... }`.
- Scoped search found no `MPModesMD`, `MPBattleMD`, `MustAlly`, or `AlliesAllowed` model/parser under `src/`.

Rust delta: missing full MPModes roster/parser, missing selected-mode object state, missing map-filter strings from the selected mode, missing `MustAlly`/`AlliesAllowed` callback effects on team UI, and missing selected-mode Start acceptance dispatch surface.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MPModesMD.ini` file open and section/row parse | verified | `0x005D7590`; string `0x00830A18` | none |
| Category registration order | verified | `0x005D7CE0` assembly | none |
| Common mode constructor defaults | verified | `0x005D5B60`, `0x005D5BEA..0x005D5D11` | none |
| Concrete mode constructors/vtables | verified | constructors and vtable memory at `0x007EE184`, `0x007EE50C`, `0x007EE6FC`, `0x007EE814`, `0x007EE424`, `0x007EE27C` | none for callback addresses |
| Stock exposed roster | verified | `ini/mpmodesmd.ini`; retail `ra2md.mix` text; binary reader `0x005D7590` | none |
| Override `[MultiplayerDialogSettings]` values | verified | retail `rg -a` text plus binary key reads `0x005D5C9D..0x005D5D11` | exact hashed filename-to-payload extraction can improve attribution wording |
| `DAT_00A8B23C` selection path | verified | `0x005E7160` decompile and assembly | no caller xref found; active path inferred from shell control/global use and prior shell reports |
| default country/team callbacks | verified | `0x004E4170`, bytes `0x005D5DB0..0x005D5E08` | no separate "start-position default callback" found in this slice |
| Siege stock exposure | verified | binary category and override text exist; no `[Siege]` in `mpmodesmd.ini` | role constructor owned by slot 4 report |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which file creates offline YR mode objects? -> `MPModesMD.ini` opened/read by `0x005D7590`.` (evidence: `0x005D7590`, string `0x00830A18`)
- `[RESOLVED] OQ-02 - Which categories are registered? -> Battle, ManBattle, Siege, Unholy, FreeForAll, Cooperative, in that order.` (evidence: `0x005D7D3C..0x005D7E36`)
- `[RESOLVED] OQ-03 - Which stock categories expose rows? -> Battle, ManBattle, FreeForAll, Unholy, Cooperative; no stock `[Siege]` section.` (evidence: `ini/mpmodesmd.ini:7..27`)
- `[RESOLVED] OQ-04 - What fields does each roster row provide? -> display key, tooltip key, override filename, map filter, random-map bool plus numeric id from row key.` (evidence: `0x005D7590`, `ini/mpmodesmd.ini`)
- `[RESOLVED] OQ-05 - What are constructor defaults before override? -> AlliesAllowed/WonlineTournamentAllowed/WonlineClanTournamentAllowed default true, MustAlly false.` (evidence: `0x005D5BEA..0x005D5BF6`)
- `[RESOLVED] OQ-06 - Where are MustAlly/AlliesAllowed read? -> Common constructor reads them from `[MultiplayerDialogSettings]` in the override INI.` (evidence: `0x005D5CDF..0x005D5D11`)
- `[RESOLVED] OQ-07 - Can MustAlly be true when AlliesAllowed is false? -> No; constructor clears MustAlly in that case.` (evidence: `0x005D5D05..0x005D5D11`)
- `[RESOLVED] OQ-08 - Which stock mode sets MustAlly? -> Team Game sets `MustAlly=yes` and `AlliesAllowed=yes`.` (evidence: retail `ra2md.mix` text lines `111638..111641`, reader `0x005D5CF7`)
- `[RESOLVED] OQ-09 - Which stock modes disable allies? -> Free For All and Cooperative set `AlliesAllowed=no`; base rules default also says no, but most mode overrides set yes.` (evidence: retail `ra2md.mix` lines `111059..111061`, `111625..111627`; reader `0x005D5CDF`)
- `[RESOLVED] OQ-10 - How is `DAT_00A8B23C` selected? -> Control `0x6EB` item data is the mode pointer; `0x005E7160` validates then commits it.` (evidence: `0x005E71E5..0x005E7382`)
- `[RESOLVED] OQ-11 - What does country fallback do? -> invalid combo data calls selected mode `+0x28`; base returns `-2`.` (evidence: `0x004E4170`, bytes at `0x005D5DB0`)
- `[RESOLVED] OQ-12 - What does Team None depend on? -> `MustAlly`; `+0x2C` returns `-2` only when `MustAlly=0`.` (evidence: bytes `0x005D5DC0..0x005D5DCD`)
- `[RESOLVED] OQ-13 - What does team-value validation depend on? -> `MustAlly`; `-2` rejected when `MustAlly=1`, `0..3` accepted.` (evidence: bytes `0x005D5DE0..0x005D5E08`)
- `[RESOLVED] OQ-14 - Is Siege active in standard offline roster? -> No, not selectable from stock `MPModesMD.ini`; binary support and override text are present.` (evidence: `0x005D7DA0`, `ini/mpmodesmd.ini`, retail text `Mode == Siege`)
- `[RESOLVED] OQ-15 - Does Rust model these mode objects? -> No direct MPModes parser/model found; launch mode is hardcoded Battle.` (evidence: `src/skirmish_launch.rs:14`, `src/ui/skirmish_shell/state.rs:392`, `rg`)
- `[DEFERRED] OQ-16 - Exact hash-table filename attribution for each contiguous override payload in `ra2md.mix`.` (category: bounded-cost-too-high; reason: requires a dedicated MIX directory/name extraction pass; next-step-if-pursued: use `AssetManager::get_with_source_ref` or an external MIX lister without writing extracted files)
- `[DEFERRED] OQ-17 - Full Cooperative campaign payload beyond object defaults.` (category: out-of-scope; reason: Cooperative mission/list internals are not needed for offline selected-mode construction/defaults; next-step-if-pursued: separate Cooperative mode investigation)
- `[DEFERRED] OQ-18 - Full UI caller census for `0x005E7160`.` (category: requires-different-system-context; reason: Ghidra reports no direct callers, likely callback/table-driven; prior shell docs establish active shell path; next-step-if-pursued: trace dialog callback/control notification table)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock MPModes are data-driven rows from `MPModesMD.ini` plus override `[MultiplayerDialogSettings]`; there are 9 stock selectable rows and no stock Siege roster row. | `0x005D7590`, `0x005D7CE0`, `ini/mpmodesmd.ini`, retail `ra2md.mix` text | missing | needed rules/assets mode loader; `src/skirmish_launch.rs` mode enum/model | Parse roster id/name/tooltip/override/filter/random and merge override defaults into a mode model. | `parses_stock_mpmodesmd_roster_and_overrides_from_retail_assets`: asserts 9 exposed modes, id 9 TeamGame has `MustAlly`, id 2 FreeForAll has `AlliesAllowed=false`, no selectable Siege. | Do not hardcode "Battle only" or synthesize a visible Siege row from binary support alone. |
| Selected mode is a pointer-like object selected through combo `0x6EB`; committed selection writes `DAT_00A8B23C`, `DAT_00A8B250=mode id`, and `DAT_00A8B254=map index`, then calls mode callbacks. | `0x005E71E5..0x005E7382` | missing | `src/ui/skirmish_shell/state.rs`, selected-mode state and map filtering | Store selected mode object/id/filter in shell state and update map filtering/UI from that selected mode, not from a global Battle assumption. | `selected_mode_change_updates_filter_and_launch_mode`: selecting Team Game changes map filter to `teamgame`, launch mode id to 9, and selected map index is preserved/clamped like native. | Do not treat map `GameModes=standard` as universal for all modes; Team Game has its own `teamgame` filter. |
| Team `None` and team value acceptance are callbacks over mode fields: `MustAlly=yes` suppresses `None` and rejects `-2`; `AlliesAllowed=no` makes allies helper return `-2`. | bytes `0x005D5DC0..0x005D5E08`, Team Game override text | missing | `src/ui/skirmish_shell/state.rs`, team combo model/validation | Rebuild team choices from selected mode fields: Team Game must force explicit team values; FreeForAll/Coop must not expose ally behavior as if Battle. | `team_game_must_ally_suppresses_team_none`: Team Game omits `None` and rejects launch/session packing with `LaunchTeam::None`; Battle permits `None`. | Do not model Team None as a static choice independent of selected mode. |

## Negative Facts / Do Not Do

- Do not expose Siege in stock offline Skirmish just because `0x005D7CE0` registers a Siege category; stock `MPModesMD.ini` has no `[Siege]` row.
- Do not treat `AlliesAllowed` from base `rulesmd.ini` as the selected mode value; `0x005D5B60` reads each mode's override file and most stock mode overrides replace it.
- Do not let `MustAlly=yes` survive when `AlliesAllowed=no`; the native constructor clears it.
- Do not implement Team `None` as a constant combo row; it is selected-mode dependent through vtable `+0x2C`.
- Do not use floating-point or simulation-side dependencies for this shell model; all verified behavior here is UI/rules data and fixed integer ids/booleans.

## Remaining Uncertainty

- Exact filename-to-payload attribution inside hashed `ra2md.mix` remains Medium, although the payload comments and `MPModesMD.ini` filenames line up with the stock mode list.
- Direct caller/callback table for `0x005E7160` was not resolved in this slice; selected-object behavior itself is verified.
- Cooperative campaign-specific extra data inside the large `0x344` object is outside this report.

## Stale Docs / Follow-up Docs

Replace the old partial-audit wording "exact stock override-file values need archive extraction" with:

> Stock retail `ra2md.mix` contains readable override payloads. For the common MPModes object fields read by `0x005D5B60`, Battle/Megawealth/Duel/Meat-Grinder/Naval/Unholy set `AlliesAllowed=yes`; Free For All sets `AlliesAllowed=no` and `WonlineClanTournamentAllowed=no`; Cooperative sets `AlliesAllowed=no` and `WonlineTournamentAllowed=no`; Team Game sets `AlliesAllowed=yes`, `AllyChangeAllowed=no`, and `MustAlly=yes`. Exact MIX hash filename attribution remains a separate extraction concern, but the common-object defaults and Start/UI callback effects are now verified.

## Sources

- Ghidra decompiled/read-only: `0x005D7590`, `0x005D7CE0`, `0x005D5B60`, `0x005D6130`, `0x005E7160`, `0x004E4170`, `0x005D6310`, `0x006ACCA0`, constructors `0x005C0DD0`, `0x005C6150`, `0x005CA630`, `0x005CB3A0`, `0x005C5CE0`, `0x005C1470`.
- Ghidra memory/assembly read-only: vtables `0x007EE184`, `0x007EE50C`, `0x007EE6FC`, `0x007EE814`, `0x007EE424`, `0x007EE27C`; bytes `0x005D5DB0..0x005D5E08`; factory registration `0x005D7D3C..0x005D7E36`; selection commit `0x005E71E5..0x005E7382`.
- INI/data checked: `C:/Users/enok/Documents/ra2-rust-game/ini/mpmodesmd.ini`; retail text scan of `C:/Users/enok/Documents/Command and Conquer Red Alert II/ra2md.mix`; `ini/rulesmd.ini` base `[MultiplayerDialogSettings]`.
- Prior docs used for context: `skirmish-ui/SKIRMISH_MPMODES_RETAIL_VALUES_AUDIT_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_MODE_ROLE_UI_NODE_0X6B_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_SIEGE_ATTACKER_ROLE_CONSTRUCTOR_GHIDRA_REPORT.md`.
- Rust scan: `src/skirmish_launch.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish.rs`.
