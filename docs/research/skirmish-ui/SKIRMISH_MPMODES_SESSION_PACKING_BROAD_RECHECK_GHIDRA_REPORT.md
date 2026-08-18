# Skirmish MPModes Session Packing Broad Recheck - Ghidra Research Report

**Address(es):** `0x005D5B60`, `0x005D7590`, `0x005D6130`, `0x005E7160`, `0x0069AE10`, `0x006ACEE0`, `0x005D6310`, `0x005C5D40`, `0x005C1D80`, `0x005CB400`, `0x005CA6D0`, `0x00671EA0`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** reconciliation of Skirmish MPModes object construction/defaults/override payloads, selected-mode chooser values, selected-mode `+0x14` Start acceptance contract, final Start Game packing fields, and current Rust model gaps.  
**Non-Scope:** spawn placement, random map generator internals, shell painting/layout except where mode/session fields feed visible mode/category/map choices or launch behavior.  
**Confidence:** High for stock/local YR mode object fields, chooser/filter flow, stock `+0x14` acceptance/non-`0x617` rejection results, and Start packing shape; Medium for future modded/custom mode behavior and exact MIX hash filename attribution for override payloads.  
**Active in YR:** Yes for stock offline Skirmish mode rows and Start packing; Conditional for per-mode variants; No for stock offline Siege selection.

## 0. Working Notes

- Target question: What implementation-ready model ties together MPModes object data, visible mode/category/map rows, selected-mode acceptance, and Start Game session packing for standard YR Skirmish?
- Non-goals: Do not investigate spawn placement, random map generator details, full Cooperative campaign internals, shell painting, or Rust implementation.
- Evidence needed to mark COMPLETE: Ghidra recheck for constructor/defaults, chooser mode item data/filtering, Start `+0x14` acceptance gate, and packing writes; existing high-confidence reports for stock override payload values, CSF modal text, selected-mode MCVDeploy, team/start destination naming, and Rust scan.
- Stop conditions: Stop after every field or branch needed for a future Rust mode/session model is either verified or explicitly deferred; do not chase post-shell house/unit creation.

## 1. Overview

The stock YR Skirmish mode model is a data-driven shell/session object list built from `MPModesMD.ini`, not a single hardcoded Battle enum. Each row has a numeric id, display key, tooltip key, override filename, map-filter string, and random-map flag; the common object constructor then merges a small set of override booleans into object fields.

The selected mode is stored as an object pointer in `DAT_00A8B23C` and as an id in `DAT_00A8B250`. Choose Map control `0x6EB` stores mode object pointers as item data, filters maps through the selected mode's `mode+0x30` filter string, and commits the selected mode and map together. Start Game later calls selected-mode vtable `+0x14`, but the native generic blocking modal triggers only on false return **and** local output dword `0x617`; stock/local selectable modes do not produce that literal output.

## 2. Class Layout / Key Offsets

| Field / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| mode `+0x20` | display/name string used in `0x6EB` rows | `0x005D626E..0x005D627E` | Yes |
| mode `+0x28` | numeric mode id/key; copied to `DAT_00A8B250` on commit | `0x005D5BB8..0x005D5BBD`; `0x005E734F..0x005E737D` | Yes |
| mode `+0x2C` | override filename string used by common constructor | decompile `0x005D5B60`; prior `SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS...` | Yes |
| mode `+0x30` | map filter string (`standard`, `teamgame`, etc.) | `0x005D6419..0x005D641F`; `0x0069AE10` | Yes |
| mode `+0x34` | random maps allowed row field | `0x005D7590`; `ini/mpmodesmd.ini` | Yes |
| mode `+0x3C` | `AlliesAllowed` common object byte | default/set/read at `0x005D5BF2`, `0x005D5CDF` | Conditional by mode |
| mode `+0x3D` | `WonlineTournamentAllowed` | default/read at `0x005D5BEA`, `0x005D5CA7` | Conditional online; still constructed |
| mode `+0x3E` | `WonlineClanTournamentAllowed` | default/read at `0x005D5BEE`, `0x005D5CC4` | Conditional online; still constructed |
| mode `+0x3F` | `MustAlly`; controls Team None and `-2` team acceptance | default/read/clamp at `0x005D5BF6`, `0x005D5CF7..0x005D5D11` | Yes for Team Game |
| `DAT_00A8B23C` | selected MPModes object pointer | `0x005E71E5..0x005E7382`; Start call `0x006AD2BA..0x006AD2D5` | Yes |
| `DAT_00A8B250` | selected mode id mirror | `0x005E734F..0x005E737D`; Start mirror `0x006AD34B..0x006AD364` | Yes |
| `DAT_00A8B254` | selected scenario/map index | `0x005E7370..0x005E7382`; Start mirror clamps out-of-range to `0` | Yes |

## 3. Core Logic

### 3.1 Mode construction and defaults

`0x005D7590` opens `MPModesMD.ini`, enumerates registered mode categories, parses five comma fields per numeric row, constructs a concrete mode object, and inserts it into the global mode vector sorted by id. Active in YR: Yes; decompile confirms the file string `MPModesMD.ini`, comma field parsing, factory call, and sorted insertion.

`0x005D5B60` is the common constructor. It writes object defaults before override reads:

- `WonlineTournamentAllowed=1` at `0x005D5BEA`
- `WonlineClanTournamentAllowed=1` at `0x005D5BEE`
- `AlliesAllowed=1` at `0x005D5BF2`
- `MustAlly=0` at `0x005D5BF6`

Then it reads only these common-object keys from `[MultiplayerDialogSettings]`: `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`, and `MustAlly`. If `MustAlly` reads true while `AlliesAllowed` is false, it clears `MustAlly` back to zero at `0x005D5D05..0x005D5D11`. Active in YR: Yes.

Adjacent dialog defaults such as `Money`, `UnitCount`, `TechLevel`, `GameSpeed`, `ShortGame`, `BuildOffAlly`, `FogOfWar`, and `MCVRedeploys` are read by `RulesClass__ReadMultiplayerDialogSettings @ 0x00671EA0`, not by the common MPModes object constructor. Active in YR: Yes as dialog/rules defaults; negative for the MPModes object layout.

### 3.2 Stock/local mode rows and override effects

| Id | Category | UI key | Override | Filter | Random | Common override effect | Active in stock offline YR |
|---:|---|---|---|---|---:|---|---|
| 1 | Battle | `GUI:Battle` | `MPBattleMD.ini` | `standard` | true | `AlliesAllowed=yes`, `MustAlly=false` | Yes |
| 2 | FreeForAll | `GUI:FreeForAll` | `MPFreeForAllMD.ini` | `standard` | true | `AlliesAllowed=no`, clan tournament false | Yes |
| 3 | Cooperative | `GUI:Cooperative` | `MPCoopMD.ini` | `cooperative` | false | `AlliesAllowed=no`, tournament false | Yes |
| 4 | Unholy | `GUI:UnholyAlliance` | `MPUnholyMD.ini` | `standard` | false | `AlliesAllowed=yes`, `MustAlly=false` | Yes |
| 5 | ManBattle | `GUI:Megawealth` | `MPMWMD.ini` | `megawealth` | false | `AlliesAllowed=yes`, `MustAlly=false` | Yes |
| 6 | ManBattle | `GUI:Duel` | `MPDuelMD.ini` | `duel` | false | `AlliesAllowed=yes`, `MustAlly=false` | Yes |
| 7 | ManBattle | `GUI:MeatGrind` | `MPMeatMD.ini` | `meatgrind` | false | `AlliesAllowed=yes`, `MustAlly=false` | Yes |
| 8 | ManBattle | `GUI:NavalWar` | `MPNavalMD.ini` | `navalwar` | false | `AlliesAllowed=yes`, `MustAlly=false` | Yes |
| 9 | Battle | `GUI:TeamGame` | `MPTeamMD.ini` | `teamgame` | false | `AlliesAllowed=yes`, `MustAlly=yes` | Yes |
| none | Siege | none in stock roster | binary support exists | none | n/a | no stock selectable object | No for stock offline selection |

Evidence: `ini/mpmodesmd.ini`; binary reader `0x005D7590`; common override readers `0x005D5CA7..0x005D5D11`; prior `SKIRMISH_MPMODES_OVERRIDE_PAYLOAD_VALUES_GHIDRA_REPORT.md` for retail payload text.

### 3.3 Visible mode/category rows and map filtering

`0x005D6130` clears control `0x6EB`, iterates the global MPModes vector, applies conditional WOL/tournament gates, applies `+0xBC` in the relevant node context, applies `+0x40` when `g_GameMode == 5`, adds `mode+0x20` display text, selects the row whose `mode+0x28` equals the requested id, and stores the mode object pointer as row item data via message `0x19A`. Active in YR: Yes for standard offline Skirmish; WOL gates are conditional.

Map filtering uses `mode+0x30`, not the visible label or category name. `0x005D6419..0x005D641F` passes `mode+0x30` to `0x0069AE10`; `0x0069AE10` accepts maps with no `GameModes` only when selected filter is literal `standard`, otherwise it scans each parsed `GameModes` string for equality. `RandMap.Sed` bypasses that list and asks selected-mode random-map capability. Active in YR: Yes.

`0x005E7160` accepts the Choose Map modal by reading map control `0x553` and mode control `0x6EB`, temporarily writing `DAT_00A8B23C`, validating through mode callbacks, then committing `DAT_00A8B23C`, `DAT_00A8B250`, and `DAT_00A8B254` together. Active in YR: Yes.

### 3.4 Start acceptance contract

Start `0x617` in `0x006ACEE0` disables the Start button, validates capacity/min-player/same-team constraints, initializes a local output object, and calls selected mode vtable `+0x14`:

```text
006AD2BA load DAT_00A8B23C
006AD2C4 initialize output object
006AD2D2 call selected_vtable+0x14(&output)
006AD2D5 test AL
006AD2D9 if false, compare output dword to 0x617
006AD2E1 if not 0x617, call 0x005D5E10 and continue to packing
006AD2E9..006AD343 only false+0x617 shows modal/re-enables Start/returns
```

Active in YR: Yes; stock mode result is Conditional by selected mode. This recheck confirms the recent correction: not every false return blocks the launch.

Concrete stock/local `+0x14` results:

| Mode class | `+0x14` target | Behavior | Output dword `0x617`? | Active in YR |
|---|---:|---|---|---|
| Battle / Team Game | `0x005D6310` | returns `1` unconditionally | No | Yes |
| ManBattle | `0x005D6310` | returns `1` unconditionally | No | Yes |
| FreeForAll | `0x005C5D40` | delegates to base accept, then rewrites node `+0x6B` values to node indices | No | Conditional by selected FFA |
| Cooperative | `0x005C1D80` | optional two-node cooperative pre-call, then base accept | No | Conditional by selected Coop |
| Unholy | `0x005CB400` | false when `DAT_00A8B258==0`; false branch writes no output | No; initialized output remains zero | Conditional by selected Unholy/global byte |
| Siege | `0x005CA6D0` | validates node `+0x6B` roles; failure writes allocated string pointer via `0x007B6880` | No literal `0x617`; no stock row | Conditional for modded data, No stock |

### 3.5 Final Start Game packing

After acceptance, Start copies shell controls into globals/session structures before writing the dialog result:

- `DAT_00A8B3C4 = DAT_00A8B250`; `DAT_00A8B3C8 = DAT_00A8B254`, but selected map index is forced to `0` if out of range (`0x006AD34B..0x006AD36B`). Active in YR: Yes.
- AI row arrays store row kind, country, color, start, and team. Corrected start/team destinations are `DAT_00A8B2DC[slot]` for start and `DAT_00A8B2FC[slot]` for team (`0x006AD4C7..0x006AD4E6`). Active in YR: Yes.
- Local start/team values are read from controls `0x6A3` and `0x76D`, stored in `DAT_00A8B39C` and `DAT_00A8B3A4`, then copied into the allocated 0x85-byte local node at offsets `+0x5B` and `+0x63` (`0x006AD60C..0x006AD641`). Active in YR: Yes.
- `SessionClass__ProcessRandomAssignments @ 0x0069B8C0` runs before option mirrors. Active in YR: Yes.
- Trackbars mirror current controls: game speed stores `6 - trackbar_value`, money and unit count store raw control values (`0x006AD703..0x006AD79E`). Active in YR: Yes.
- Checkboxes mirror controls `0x54E`, `0x69A`, `0x69D`, `0x693`, `0x696` into `DAT_00A8B3D8..3DC` (`0x006AD7A4..0x006AD889`). Active in YR: Yes.
- Forced launch-state flags are written after option mirrors: `DAT_00A8B31F=0`, `DAT_00A8B260=1`, `DAT_00A8B31D=0`, `DAT_00A8B26C=0` (`0x006AD88F..0x006AD8A4`). Active in YR: Yes.
- Preview object `DAT_00AC1154` is destroyed before the result pointer is written (`0x006AD85A..0x006AD8BD`), then the saved dialog result receives the output value (`0x006AD8C7..0x006AD8D5`). Active in YR: Yes.

## 4. INI Keys

| INI source | Key(s) | Native role | Evidence | Active in YR |
|---|---|---|---|---|
| `ini/mpmodesmd.ini` | category rows `1..9` | visible stock mode roster, filter, random-map flag, override filename | `0x005D7590`; repo INI | Yes |
| mode override `[MultiplayerDialogSettings]` | `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`, `MustAlly` | common selected-mode object fields only | `0x005D5CA7..0x005D5D11` | Yes/Conditional |
| `rulesmd.ini:[MultiplayerDialogSettings]` | `Money=10000`, `UnitCount=10`, `GameSpeed=1`, `ShortGame=yes`, `Bases=yes`, `Crates=yes`, `FogOfWar=no`, etc. | dialog/rules defaults, later controls are packed on Start | `0x00671EA0`; `rulesmd.ini:3017..3040` | Yes |
| `rulesmd.ini:[MultiplayerDialogSettings] MCVRedeploys` | `yes` | redeploy option family, not selected-mode startup auto-deploy | `0x00671EA0`; selected-mode MCVDeploy report | Yes, but not `MCVDeploy` |

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Mode object load | `MPModesMD.ini` rows create sorted MPModes objects | `0x005D7590`; `0x005D7CE0` from prior reports | Yes |
| Choose Map mode rows | `0x6EB` rows store mode object pointer item data | `0x005D6130`, `0x005D6298..0x005D62A0` | Yes |
| Choose Map filtering | selected `mode+0x30` vs scenario record `GameModes`; empty means `standard` only | `0x005D6419`, `0x0069AE10` | Yes |
| Choose Map accept | commits selected mode pointer/id and selected map index together | `0x005E7160`, `0x005E734F..0x005E7382` | Yes |
| Start validation | capacity/min-player/same-team before mode `+0x14` | `0x006ACFBD..0x006AD2A7`; modal text report | Yes |
| Start acceptance | selected `DAT_00A8B23C` vtable `+0x14` with false+`0x617` gate | `0x006AD2BA..0x006AD34B` | Yes/Conditional |
| Start packing | row arrays, local node, options, forced flags, preview teardown, result write | `0x006AD34B..0x006AD8D5` | Yes |

No simulation tick-cycle path is claimed here; this is shell/setup handoff before scenario startup.

## 6. Current Rust Implementation Status

Rust now has a partial data model:

- `src/skirmish_modes.rs:20` defines `SkirmishGameMode` with id, UI key, tooltip, override file, map filter, random-map flag, `allies_allowed`, and `must_ally`.
- `src/skirmish_modes.rs:63` hardcodes known stock override effects by filename; it does not parse MIX-backed override payloads.
- `src/skirmish_scenarios.rs:175` parses map `GameModes`, and `src/skirmish_scenarios.rs:197` implements native-shaped map filtering including empty=`standard` and random-map flag gating.
- `src/ui/skirmish_shell/state.rs:541` stores `selected_mode_id`, and Choose Map modal state uses modes for filtering.

Remaining gaps:

- `src/skirmish_launch.rs:14` has only `SkirmishLaunchMode::Battle`, and `src/ui/skirmish_shell/state.rs:1427` always packs `mode: Battle`; selected mode id/object is not carried into `SkirmishLaunchSession`.
- `launch_session` at `src/ui/skirmish_shell/state.rs:1346` takes only state and maps, so it cannot run native selected-mode `+0x14` acceptance or selected-mode-specific launch/session behavior.
- Team combo items are static `[-2,0,1,2,3]` at `src/ui/skirmish_shell/state.rs:932`; Team Game/MustAlly suppression and selected-mode `-2` rejection are not represented in the combo model.
- Common override values are stock-hardcoded, not data-driven from retail/mod override INIs.
- Native Start packing has a richer row/global/node table than the current clean Rust session. Clean Rust does not need those globals internally, but tests still need to prove equivalent observable behavior for selected mode id/filter/team constraints, map choice, active rows, starts, teams, random assignments, options, and no false generic mode rejection for stock modes.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MPModesMD.ini` row parsing and sorted mode vector | verified | decompile `0x005D7590`; `ini/mpmodesmd.ini` | exact factory decompile per category inherited from prior object report |
| Common constructor defaults and override reads | verified | decompile `0x005D5B60`; assembly `0x005D5BEA..0x005D5D11` | none for common fields |
| Stock override common payload values | verified | prior `SKIRMISH_MPMODES_OVERRIDE_PAYLOAD_VALUES...`; reader addresses rechecked | exact MIX hash filename attribution |
| Rules/dialog defaults reader | verified | decompile `0x00671EA0` | full mode override rules application outside common object |
| `0x6EB` row item data and selection by id | verified | decompile `0x005D6130`; assembly `0x005D625B..0x005D62A0` | none |
| map filter predicate | verified | `0x005D6419`; decompile/assembly `0x0069AE10` | official-map runtime gate outside standard offline scope |
| Choose Map accept commit | verified | decompile `0x005E7160`; assembly `0x005E71E5..0x005E7382` | none for selected mode/map commit |
| Start `+0x14` caller gate | verified | decompile `0x006ACEE0`; assembly `0x006AD2BA..0x006AD34B` | modded/custom false+`0x617` object behavior |
| Battle/ManBattle `+0x14` | verified | decompile `0x005D6310` | none |
| FFA/Coop/Unholy/Siege `+0x14` | verified by assembly and prior reports | assembly `0x005C5D40`, `0x005C1D80`, `0x005CB400`, `0x005CA6D0` | no stock Siege row; modded data out of scope |
| final Start packing | verified | decompile `0x006ACEE0`; assembly ranges in section 3.5 | post-shell consumers out of scope |
| selected-mode MCVDeploy contrast | verified by prior focused report | `SKIRMISH_SELECTED_MODE_MCVDEPLOY_START_FLAG...` | exact null-mode flag conflict owned by SpecialFlags report |
| current Rust mode model | verified | `src/skirmish_modes.rs`, `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs` | no code changes in this slot |
| spawn placement/random map generator | deferred | user non-scope | separate spawn/RMG reports already exist |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which mode rows are stock selectable? -> ids 1,2,3,4,5,6,7,8,9 from `MPModesMD.ini`; no stock Siege row.` (evidence: `ini/mpmodesmd.ini`; `0x005D7590`)
- `[RESOLVED] OQ-02 - Which fields are common MPModes object defaults? -> tournament flags true, `AlliesAllowed=true`, `MustAlly=false`.` (evidence: `0x005D5BEA..0x005D5BF6`)
- `[RESOLVED] OQ-03 - Which override keys does the common object read? -> only `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`, `MustAlly`.` (evidence: `0x005D5CA7..0x005D5D11`)
- `[RESOLVED] OQ-04 - Are money/unit/speed common mode object fields? -> No; they are read by `RulesClass__ReadMultiplayerDialogSettings`.` (evidence: `0x00671EA0`)
- `[RESOLVED] OQ-05 - How is `0x6EB` populated? -> from the MPModes vector, display `+0x20`, select by id `+0x28`, item data is object pointer.` (evidence: `0x005D6130`; `0x005D625B..0x005D62A0`)
- `[RESOLVED] OQ-06 - How are maps filtered by selected mode? -> selected `mode+0x30` vs record `GameModes`, empty records match only `standard`; random sentinel uses random-map capability.` (evidence: `0x005D6419`; `0x0069AE10`)
- `[RESOLVED] OQ-07 - How is selected mode committed? -> `0x005E7160` commits `DAT_00A8B23C`, `DAT_00A8B250`, and `DAT_00A8B254` together after validation.` (evidence: `0x005E71E5..0x005E7382`)
- `[RESOLVED] OQ-08 - Does every selected-mode false return block Start? -> No; native blocks only when false return leaves output dword exactly `0x617`.` (evidence: `0x006AD2BA..0x006AD34B`)
- `[RESOLVED] OQ-09 - Do stock/local selectable modes write output dword `0x617`? -> No; Battle/ManBattle/FFA/Coop accept, Unholy false writes no output, Siege is not stock and writes string pointers.` (evidence: `0x005D6310`; assembly `0x005C5D40`, `0x005C1D80`, `0x005CB400`, `0x005CA6D0`)
- `[RESOLVED] OQ-10 - What Start fields are packed after acceptance? -> selected map mirrors, row arrays, local node, random assignments, trackbars, checkboxes, forced flags, and preview teardown before result write.` (evidence: `0x006AD34B..0x006AD8D5`)
- `[RESOLVED] OQ-11 - Does selected stock Skirmish MCVDeploy auto-deploy via mode/session packing? -> No; selected-mode callbacks do not call `Force_MCV_Deploy`; null-mode only.` (evidence: `SKIRMISH_SELECTED_MODE_MCVDEPLOY_START_FLAG_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-12 - What current Rust pieces already exist? -> partial MPModes row model, stock hardcoded common fields, mode filtering, selected_mode_id state, launch validation/session packing.` (evidence: Rust files in section 6)
- `[RESOLVED] OQ-13 - What current Rust pieces are missing? -> selected mode in launch session, mode acceptance contract, MIX-backed override parsing, mode-dependent team combo/validation, and tests tying selected mode to launch behavior.` (evidence: Rust files in section 6)
- `[DEFERRED] OQ-14 - Exact MIX hash entry-to-filename attribution for each override payload.` (category: bounded-cost-too-high; reason: existing report proves common values from visible retail payload text and binary readers; exact hash attribution needs dedicated MIX directory extraction; next-step-if-pursued: read archive index through asset tooling without writing files)
- `[DEFERRED] OQ-15 - Full rules override application beyond common mode object fields.` (category: out-of-scope; reason: this slot reconciles mode object/session packing, not scenario/rules mutation; next-step-if-pursued: trace selected override filename into full rules load)
- `[DEFERRED] OQ-16 - Post-shell consumers of every packed global/node field.` (category: out-of-scope; reason: spawn placement and post-start consumers are excluded; next-step-if-pursued: use existing Start-to-spawn reports)
- `[DEFERRED] OQ-17 - Modded/custom MPModes object that intentionally writes output dword `0x617`.` (category: out-of-scope; reason: no stock/local class does this; next-step-if-pursued: investigate extension hooks or custom mode class paths)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock visible modes are data rows from `MPModesMD.ini` plus common override fields; Siege is not stock-selectable. | `0x005D7590`; `0x005D5B60`; `ini/mpmodesmd.ini`; override payload report | partial: roster parsed, common overrides hardcoded by filename | `src/skirmish_modes.rs` | Replace stock filename branches with MIX-backed override parsing for common fields; preserve no stock Siege row. | Stock assets produce 9 modes: id 9 TeamGame `must_ally=true`, FFA/Coop `allies_allowed=false`, no Siege. | Do not synthesize Siege from binary category registration. |
| Map rows are filtered by selected mode filter string; empty map `GameModes` means `standard` only; random sentinel is mode-gated. | `0x005D6419`; `0x0069AE10`; `ini/mpmodesmd.ini` | mostly implemented | `src/skirmish_scenarios.rs`, Choose Map modal state | Keep filtering tied to selected mode object, and test it through modal selection plus launch session. | Selecting Team Game shows `teamgame` maps and hides random; selecting Battle/FFA shows standard maps and random. | Do not filter by UI label or treat missing `GameModes` as match-all. |
| Choose Map commits selected mode and map together; mode row item data is object identity, id copied separately. | `0x005E7160`; `0x005D6130` | partial: selected_mode_id exists, launch still ignores it | `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs` | Carry selected mode id/data into launch session and preserve accepted/cancelled modal transaction semantics. | Change mode and map in modal, cancel restores old launch mode/map; accept changes both and subsequent Start packs that mode id. | Do not update launch mode from display text or leave session as Battle-only. |
| Start selected-mode rejection blocks only on false return plus output dword `0x617`; stock/local modes do not write that dword. | `0x006AD2BA..0x006AD34B`; mode sweep | missing mode acceptance surface | future selected-mode acceptance model, `launch_session` validation | Model stock Battle/ManBattle/FFA/Coop as accepting; Unholy false must not be mapped to generic `0x469` modal unless output is literal `0x617`. | Synthetic test mode false+`0x617` blocks; stock Unholy false/non-`0x617` path does not use generic `0x469` modal. | Do not implement "any false return blocks" or show `0x469` as a body string. |
| Team None and `-2` team acceptance are selected-mode dependent through `MustAlly`; Team Game suppresses None. | constructor `+0x3F`; prior team report; `src/ui/skirmish_shell/state.rs:932` static rows | missing/mismatch | combo item builder, launch validation, mode model | Build team rows from selected mode: Battle has None+A-D; Team Game omits None and rejects `LaunchTeam::None`. | Selecting Team Game removes Team None for all team combos and cannot pack a `None` team; Battle permits it. | Do not keep team rows as static `[-2,0,1,2,3]` for all modes. |
| Start packing mirrors selected mode/map, active rows, start/team, trackbars, checkboxes, forced flags, and destroys preview before result. | `0x006AD34B..0x006AD8D5` | partial clean session; launch mode is Battle-only | `src/skirmish_launch.rs`, `src/ui/skirmish_shell/state.rs`, `src/app.rs` | Clean Rust can avoid native globals, but must expose equivalent deterministic session fields and tests for selected mode id, rows, options, random assignments, and no launch on validation failure. | Start with Team Game, explicit starts/teams/options: session contains selected mode id 9, selected map, local+AI starts/teams/options; invalid capacity/no-opponent/same-team never creates pending session. | Do not mistake `MCVRedeploys` for startup `MCVDeploy`; do not fold spawn placement into this shell/session model. |

## Negative Facts / Do Not Do

- Do not hardcode `SkirmishLaunchMode::Battle` as the only possible launch result once non-Battle rows are visible. Active in YR: Yes for visible rows id 2..9; evidence `MPModesMD.ini`, `0x005D7590`, `0x005E7160`.
- Do not expose Siege in stock offline Skirmish. Active in YR: No for stock roster; evidence no `[Siege]` row in `ini/mpmodesmd.ini`, despite binary category support.
- Do not treat all `[MultiplayerDialogSettings]` keys as MPModes object fields. Active in YR: common object reads only four keys at `0x005D5CA7..0x005D5D11`; money/unit/speed/checkboxes are RulesClass/dialog settings at `0x00671EA0`.
- Do not let `MustAlly=yes` survive when `AlliesAllowed=no`. Active in YR: Yes; constructor clears it at `0x005D5D05..0x005D5D11`.
- Do not treat every selected-mode false return as a Start-blocking modal. Active in YR: Yes; `0x006ACEE0` blocks only when output dword equals `0x617`.
- Do not use `0x469` as a unique rejection body. Active in YR: Conditional; it is the OK/control text per the modal text report, while the body comes from the mode output object.
- Do not treat `MCVRedeploys` as `[SpecialFlags] MCVDeploy` or add selected-mode startup auto-deploy. Active in YR: selected stock modes do not use that null-mode auto-deploy path.
- Do not claim native parity from map list display order or row filtering alone unless mode id/filter/random sentinel and accept/cancel transaction semantics are tested together.

## Sources

- Ghidra read-only decompile/recheck: `005D5B60`, `005D7590`, `005D6130`, `005E7160`, `006ACEE0`, `005D6310`, `00671EA0`.
- Ghidra assembly contexts: `0x005D5BEA..0x005D5D11`, `0x005D625B..0x005D62A0`, `0x005D6419..0x0069AE30`, `0x005E71E5..0x005E7382`, `0x006AD2BA..0x006AD34B`, `0x005C5D40..0x005C5D86`, `0x005C1D80..0x005C1DB2`, `0x005CB400..0x005CB421`, `0x005CA6D0..0x005CA7BF`, `0x006AD34B..0x006AD8D5`.
- Prior reports reconciled: `SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_OVERRIDE_PAYLOAD_VALUES_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_RETAIL_VALUES_AUDIT_GHIDRA_REPORT.md`, `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`, `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODE_CATEGORY_0X6EB_GHIDRA_REPORT.md`, `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md`, `SKIRMISH_SELECTED_MODE_MCVDEPLOY_START_FLAG_GHIDRA_REPORT.md`.
- INI files checked: `ini/mpmodesmd.ini`, `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/skirmish_modes.rs`, `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/app_list_maps.rs`.
