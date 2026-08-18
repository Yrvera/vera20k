# Skirmish MPModes MustAlly / AlliesAllowed Row Behavior - Ghidra Research Report

**Address(es):** `0x005D5B60`, `0x004E5B60`, `0x004E5D60`, `0x005D5DC0..0x005D5E08`, `0x006ADC20`, `0x006AE6E0`, `0x006ACEE0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish setup/session row behavior driven by the selected MPMode common-object fields `MustAlly` and `AlliesAllowed`: Team combo row population, inactive/active AI-row team defaults, same-team validation, and final Start packing.  
**Non-Scope:** multiplayer/WOL team semantics, post-launch House alliance consumers, start-position combo behavior except to avoid confusing it with Team controls, and full mode override payload extraction beyond prior verified Team/FFA/Coop values.  
**Confidence:** High for native row-control behavior and Start validation/packing consequences; Medium only for exact retail MIX filename hash attribution inherited from the payload report.  
**Active in YR:** Yes for standard offline Skirmish dialog `0x102`; conditional by selected stock mode values.

## 0. Working Notes

- Target question: How do selected MPMode `MustAlly` and `AlliesAllowed` values affect offline Skirmish row controls and launch behavior, especially Battle/Team Game/Free For All/Cooperative differences?
- Non-goals: Do not investigate multiplayer netcode/alliance negotiation, post-launch House alliance consumers, start-position controls, random map generation, or Rust implementation patches.
- Evidence needed to mark COMPLETE: decompile plus assembly/disassembly range for the MPMode constructor fields, Team combo population path, vtable callbacks, AI-row change/default path, dialog initialization refresh order, Start validation/packing, INI/default sources for stock Team/FFA/Coop values, and current Rust surface scan.
- Stop conditions: Stop when every material row-control and launch-packing effect of `MustAlly` / `AlliesAllowed` is verified or explicitly deferred; do not chase network branches or post-session alliance consumers.

## 1. Overview

Offline Skirmish Team controls are not hardcoded the same for every mode. The selected MPMode object supplies two separate policy bytes: `MustAlly` suppresses Team `None` and makes `-2` invalid for that mode's validator, while `AlliesAllowed` controls the default Team selection written to AI rows during row-state changes and initialization.

The player-visible stock result is: Team Game has Team choices `A-D` only and defaults the local player to `A`; Battle/Unholy/ManBattle-style modes have `None,A-D`; Free For All and Cooperative still expose `None,A-D`, but their AI rows default to `None` because `AlliesAllowed=false`. Active in YR: Yes, for standard offline Skirmish.

## 2. Class Layout / Key Offsets

| Field / control | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `MultiplayerGameMode +0x3C` | `AlliesAllowed` byte; default true, then per-mode override | decompile `0x005D5B60`; read by `0x005D5DD0`, `0x006ADC20`, `0x006AE6E0` | Yes / conditional by mode |
| `MultiplayerGameMode +0x3F` | `MustAlly` byte; default false, then per-mode override, cleared if allies disabled | decompile `0x005D5B60`; assembly `0x005D5DC0`, `0x005D5DE0` | Yes / conditional by mode |
| vtable `+0x2C` | Team `None` availability helper: returns `-2` when `MustAlly==0`, else `0` | caller decompile `0x004E5B60`; assembly `0x005D5DC0..0x005D5DCD` | Yes |
| vtable `+0x30` | ally/default helper: returns `3` when `AlliesAllowed!=0`, else `-2` | assembly `0x005D5DD0..0x005D5DDD`; direct row-default readers | Yes |
| vtable `+0x34` | Team value validator: rejects `-2` when `MustAlly!=0`, accepts only `0..3` otherwise | assembly `0x005D5DE0..0x005D5E08`; prior vtable-binding report | Yes as callback semantics |
| Team controls `0x76D..0x774` | local plus seven AI Team combos | `0x004E5D60`, `0x004E5ED0`, `0x006ACEE0` | Yes |
| AI row-state controls `0x50B,0x50E,0x516,0x51A..0x51D` | active/inactive AI row controls; active values are `0,1,2`; inactive is `-1` | decompile `0x006ADC20`, `0x006AE6E0`, `0x006ACEE0` | Yes |

## 3. Core Logic

### 3.1 MPMode field construction

`0x005D5B60` constructs the common MPMode object with these defaults before reading override payloads:

- `AlliesAllowed = 1` at object byte `+0x3C`.
- `MustAlly = 0` at object byte `+0x3F`.
- `WonlineTournamentAllowed` / `WonlineClanTournamentAllowed` also default true, but they do not drive the scoped row behavior.

It then reads `[MultiplayerDialogSettings]` keys in the override payload and applies a load-bearing clamp: if `MustAlly` reads true while `AlliesAllowed` is false, it clears `MustAlly` back to false. Active in YR: Yes. Evidence: decompile `0x005D5B60`; prior payload report cites key readers `0x005D5CDF` for `AlliesAllowed`, `0x005D5CF7` for `MustAlly`, and clear block `0x005D5D05..0x005D5D11`.

Stock mode values relevant to this slice:

| Mode | Roster row source | Common field result | Player-visible row consequence | Active in stock YR |
|---|---|---|---|---|
| Battle | `MPBattleMD.ini`, `standard` | `AlliesAllowed=1`, `MustAlly=0` | Team combo has `None,A-D`; AI row default Team D when active/defaulted by ally helper | Yes |
| Team Game | `MPTeamMD.ini`, `teamgame` | `AlliesAllowed=1`, `MustAlly=1` | Team combo has `A-D` only; `None` is unavailable/repaired away | Yes |
| Free For All | `MPFreeForAllMD.ini`, `standard` | `AlliesAllowed=0`, `MustAlly=0` | Team combo still has `None,A-D`; AI row default is `None` | Yes |
| Cooperative | `MPCoopMD.ini`, `cooperative` | `AlliesAllowed=0`, `MustAlly=0` | Team combo still has `None,A-D`; AI row default is `None` | Yes |

Evidence: `ini/mpmodesmd.ini:8..27`; `SKIRMISH_MPMODES_OVERRIDE_PAYLOAD_VALUES_GHIDRA_REPORT.md` lines for Team/FFA/Coop payload values; binary reader `0x005D5B60`.

### 3.2 Team combo population

`0x004E5D60` refreshes all eight Team controls. In standard offline Skirmish, each row calls `0x004E5B60`; the disabled one-item network helper is not used unless `g_GameMode` is `3` or `4` and network/observer conditions match.

`0x004E5B60` rebuilds one Team combo in this order:

1. Hide the combo, reset contents (`0x14B`), configure shell combo messages `0x4DD` and `0x4DE`.
2. For standard offline Skirmish, call `0x005E2F80` to get the selected MPMode object.
3. Dispatch selected mode vtable `+0x2C`.
4. If the return is negative, insert `GUI:NoneAsSymbols` with item data `-2`.
5. Always append Team `A`, `B`, `C`, `D` from the Team label table with item data `0`, `1`, `2`, `3`.
6. Select row index `0`, clear the disabled/grey flag message `0x4F1`, and restore visibility if it was visible before rebuild.

Active in YR: Yes. Evidence: decompile `0x004E5B60`, `0x004E5D60`; assembly context at `0x004E5B60`; prior string/table report for `A-D` labels.

The vtable `+0x2C` helper has no function boundary in this Ghidra session, so it was inspected read-only as bytes/assembly. It reads `byte [ECX+0x3F]`, then computes `-2` when zero and `0` when nonzero:

- `0x005D5DC0`: `MOV AL,byte ptr [ECX + 0x3f]`
- `0x005D5DC3..0x005D5DCA`: `NEG/SBB/AND 0x2/ADD -0x2`
- `0x005D5DCD`: `RET`

Result: `MustAlly=false` inserts `None`; `MustAlly=true` suppresses it. Active in YR: Yes.

### 3.3 AlliesAllowed row default behavior

`AlliesAllowed` does not remove Team rows from the visible combo. The Team combo still gets optional `None` plus `A-D` according to `MustAlly`, not according to `AlliesAllowed`.

`AlliesAllowed` affects the selected item written to AI Team controls by `0x006ADC20` and the final initialization loop in `0x006AE6E0`:

- If selected mode is null or `AlliesAllowed==0`, the code calls `0x004E5ED0(..., -2)` for the target AI Team combo.
- If `AlliesAllowed!=0`, it calls `0x004E5ED0(..., 3)`, selecting Team D.
- `0x006ADC20` then enables or disables side/color/start/team controls based on the AI row-state value: row-state item data `0`, `1`, or `2` is active; any other value, including `-1`, disables those sibling controls.

Active in YR: Yes. Evidence: decompile `0x006ADC20`; decompile `0x006AE6E0` final loop after selected mode normalization; `0x004E5ED0` select-by-item-data helper.

This means FFA/Coop-style `AlliesAllowed=false` does not grey out an active Team combo and does not delete `A-D` from the list. It defaults affected AI Team controls to `None` (`-2`). Team Game/Battle-style `AlliesAllowed=true` defaults affected AI Team controls to Team D (`3`), but Team Game still omits `None` because `MustAlly=true`.

### 3.4 Team setter and validator details

`0x004E5ED0(hwnd, team_control_id, requested_value)` selects the first combo item whose item data equals `requested_value`. If selected value is not `-2`, it also writes the row owner into the Team reservation table at `DAT_008B3FC8 + value*0xC`. If the requested value is absent, no matching selection is made. Active in YR: Yes; evidence decompile `0x004E5ED0`.

The selected-mode validator at `0x005D5DE0..0x005D5E08` uses `MustAlly`, not `AlliesAllowed`:

- If `MustAlly!=0` and proposed value is `-2`, return false.
- Then accept signed values `0..3`.
- Reject negative values other than allowed `-2`; therefore `-1` is not a Team value.
- Reject `4+`.

Evidence: assembly context `0x005D5DE0..0x005D5E08`, including `CMP EAX,-0x2`, `TEST EAX,EAX`, signed `JL`, `CMP EAX,0x3`, `JG`, success `MOV EAX,0x1`, failure `XOR EAX,EAX`. Active in YR: Yes as mode callback semantics. Direct Start `0x617` did not call this validator in the verified packing block; normal UI rebuild/selection prevents absent `None` rows from being selected in Team Game.

### 3.5 Start validation and packing

Start command `0x617` in `0x006ACEE0` performs ordinary setup validation before packing:

- It counts active AI row-state values `0`, `1`, `2`; inactive `-1` rows do not count.
- It rejects map capacity overflow and no-opponent setup before packing.
- It reads the local Team control `0x76D` through `0x004E6030`. If the local team is nonnegative, it checks every active AI row and rejects only when all active AIs have the same explicit team as the local player.
- This same-team validation is independent of `AlliesAllowed`; it operates on actual Team combo item data.

Active in YR: Yes. Evidence: decompile `0x006ACEE0`; assembly contexts `0x006AD152`, `0x006AD2BA`; prior Start validation modal reports for visible modal text.

After validation and selected-mode `+0x14` acceptance, packing reads AI Team controls and local Team controls:

- Active AI row loop writes `FUN_004E6030(..., -1)` result into `DAT_00A8B2FC[slot]`. Evidence: assembly `0x006AD4DF..0x006AD4E6`.
- Local Team control `0x76D` is read through `FUN_004E6030(..., -1)` and stored in `DAT_00A8B3A4`, then copied into the local node at offset `+0x63`. Evidence: assembly `0x006AD61C..0x006AD641`; decompile `0x006ACEE0`.
- Selected-mode false acceptance blocks only on false plus output dword `0x617`; this slice found no additional `AlliesAllowed`-based Start rejection in the ordinary validation/packing path. Evidence: assembly `0x006AD2BA..0x006AD34B`.

Player-visible implication: Team Game prevents `None` by UI row population and default repair, not by a special Start modal. FFA/Coop do not show a special "allies disabled" validation modal in this scoped Start path; they default teams to `None` but still expose the Team combo rows.

## 4. INI Keys

| INI / data source | Key or row | Native effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `ini/mpmodesmd.ini` | `1=...MPBattleMD.ini, standard, true` | stock Battle selected mode; allies allowed, not must-ally via payload | row reader `0x005D7590`; payload report | Yes |
| `ini/mpmodesmd.ini` | `9=...MPTeamMD.ini, teamgame, false` | stock Team Game selected mode; payload sets `AlliesAllowed=yes`, `MustAlly=yes` | `ini/mpmodesmd.ini:9`; payload report; `0x005D5B60` | Yes |
| `ini/mpmodesmd.ini` | `2=...MPFreeForAllMD.ini, standard, true` | stock FFA selected mode; payload sets `AlliesAllowed=no`, `MustAlly=0` after default/clamp | `ini/mpmodesmd.ini:20`; payload report; `0x005D5B60` | Yes |
| `ini/mpmodesmd.ini` | `3=...MPCoopMD.ini, cooperative, false` | stock Coop selected mode; payload sets `AlliesAllowed=no`, `MustAlly=0` after default/clamp | `ini/mpmodesmd.ini:27`; payload report; `0x005D5B60` | Yes |
| override `[MultiplayerDialogSettings]` | `AlliesAllowed` | byte `+0x3C`; drives AI Team default `-2` vs `3`; does not remove A-D visible rows | `0x005D5B60`, `0x006ADC20`, `0x006AE6E0` | Yes / conditional |
| override `[MultiplayerDialogSettings]` | `MustAlly` | byte `+0x3F`; suppresses Team `None`; rejects `-2` in validator | `0x005D5B60`, `0x005D5DC0`, `0x005D5DE0` | Yes / conditional |
| base `rulesmd.ini` | `AlliesAllowed=no`, `AllyChangeAllowed=yes` | rules/dialog defaults, not the selected MPMode object row-control source | prior object report; absence from scoped row readers except mode object | Yes as adjacent setting; not the combo-row source |

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Mode object construction | defaults and override fields create `+0x3C/+0x3F` | `0x005D5B60`; payload report | Yes |
| Team combo refresh | all eight Team controls rebuild after init/mode changes | `0x004E5D60`, `0x006AE6E0`, `0x006ACEE0` Choose Map accept path | Yes |
| Team None insertion | selected mode vtable `+0x2C` controls optional `-2` row | `0x004E5B60`; assembly `0x005D5DC0..0x005D5DCD` | Yes |
| AI row-state change | affected AI row Team default set from `AlliesAllowed`, then row sibling controls enabled/disabled by AI row-state active value | `0x006ADC20` | Yes |
| Initialization post-mode refresh | after selected mode normalization, AI rows `1..7` get Team D if allies allowed, else None | `0x006AE6E0` | Yes |
| Start ordinary validation | capacity/no-opponent/same-explicit-team checks run on actual combo item data | `0x006ACEE0`; modal reports | Yes |
| Start packing | active AI Team values and local Team value are packed after validation | `0x006AD4DF..0x006AD4E6`, `0x006AD61C..0x006AD641` | Yes |

No simulation tick-cycle integration is claimed here; this is shell/session setup.

## 6. Current Rust Implementation Status

Current Rust already has the data fields but not the native row behavior:

- `src/skirmish_modes.rs` defines `SkirmishGameMode { allies_allowed, must_ally }` and hardcodes stock override effects by filename.
- `src/ui/skirmish_shell/state.rs:1260` builds Team combo items as static `[-2,0,1,2,3]`, so Team Game still exposes `None`.
- `src/ui/skirmish_shell/state.rs` keeps `selected_mode_id`, but the Team combo builder and row-type change logic do not use selected-mode `must_ally` / `allies_allowed`.
- `default_opponents` initializes every opponent `team` to `-2`; native Battle/Team Game initialization/defaulting puts AI rows at Team D when `AlliesAllowed=true`.
- `launch_session` validates same explicit teams and packs `LaunchTeam::from_shell_value`, but it has no selected-mode check/repair for Team Game `None` and still returns `SkirmishLaunchMode::Battle`.
- `LaunchTeam::from_shell_value` maps any negative value to `LaunchTeam::None`; native Team combo has no `-1` Team value, so future validation should not accept `-1` as a normal Team sentinel.

No Rust files were modified by this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Required working-note lines | verified | section 0 | none |
| Common MPMode `AlliesAllowed` / `MustAlly` defaults and clamp | verified | decompile `0x005D5B60`; prior payload report | exact MIX hash attribution remains inherited uncertainty |
| Stock Team/FFA/Coop values | verified | `ini/mpmodesmd.ini`; payload report; binary reader addresses | none for common fields |
| Offline Team combo population | verified | decompile `0x004E5B60`, `0x004E5D60`; assembly context `0x004E5B60` | none |
| vtable `+0x2C` None helper | verified | caller `0x004E5B60`; assembly `0x005D5DC0..0x005D5DCD` | function boundary absent; read-only assembly was enough |
| vtable `+0x30` AlliesAllowed helper | verified | assembly `0x005D5DD0..0x005D5DDD`; `0x006ADC20`/`0x006AE6E0` direct reads | none |
| vtable `+0x34` Team validator | verified | assembly `0x005D5DE0..0x005D5E08`; prior team report | complete indirect caller census deferred as nonmaterial |
| AI row-state change default/enable behavior | verified | decompile `0x006ADC20` | none |
| Dialog init and selected-mode refresh order | verified | decompile `0x006AE6E0` | none for scoped row behavior |
| Start same-team validation | verified | decompile `0x006ACEE0`; prior validation modal reports | exact modal visuals out of scope |
| Start packing of team values | verified | decompile `0x006ACEE0`; assembly `0x006AD4DF..0x006AD4E6`, `0x006AD61C..0x006AD641` | post-launch House consumer out of scope |
| WOL/network team branch | deferred | branch observed in `0x004E5B60` / `0x004E5D60` | out-of-scope multiplayer netcode |
| Start-position combo behavior | deferred | prior start-position combo report | out-of-scope except negative separation |
| Current Rust surfaces | verified | `rg`/Codegraph scan of `src/skirmish_modes.rs`, `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/app_skirmish.rs` | implementation left to parent/future slot |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which investigation mode applies? -> exhaustive-slice for offline Skirmish row behavior only.` (evidence: user target and section 0)
- `[RESOLVED] OQ-02 - Where do `AlliesAllowed` and `MustAlly` live? -> selected MPMode object bytes `+0x3C` and `+0x3F`.` (evidence: `0x005D5B60`, `0x005D5DC0`, `0x005D5DD0`, `0x005D5DE0`)
- `[RESOLVED] OQ-03 - What if a payload sets `MustAlly=yes` with `AlliesAllowed=no`? -> constructor clears `MustAlly` to false.` (evidence: `0x005D5D05..0x005D5D11` from prior payload report; decompile `0x005D5B60`)
- `[RESOLVED] OQ-04 - Which stock scoped modes set the fields? -> Team Game sets both allies allowed and must ally; FFA/Coop disable allies and do not must-ally; Battle allows allies and does not must-ally.` (evidence: `ini/mpmodesmd.ini`; payload report)
- `[RESOLVED] OQ-05 - Does `MustAlly` disable or grey the Team combo? -> No; it changes population by omitting `None`, leaving A-D available and selecting row 0.` (evidence: `0x004E5B60`; `0x005D5DC0..0x005D5DCD`)
- `[RESOLVED] OQ-06 - Does `AlliesAllowed=false` remove A-D rows? -> No; rows still come from `MustAlly` logic; FFA/Coop still expose `None,A-D`.` (evidence: `0x004E5B60`; `0x006ADC20`)
- `[RESOLVED] OQ-07 - What does `AlliesAllowed=false` visibly/default-wise change? -> AI Team controls are selected to `None` (`-2`) instead of Team D (`3`) during row changes/init.` (evidence: `0x006ADC20`; `0x006AE6E0`)
- `[RESOLVED] OQ-08 - What controls row enable/disable? -> AI row-state item data active values `0,1,2` enable side/color/start/team; inactive `-1` disables them, independent of selected mode except the Team default selected just before disable.` (evidence: `0x006ADC20`)
- `[RESOLVED] OQ-09 - Is there a Team `Auto` / `-1` row offline? -> No; population inserts optional `-2` then `0..3`, and validator rejects `-1`.` (evidence: `0x004E5B60`; `0x005D5DE0..0x005D5E08`)
- `[RESOLVED] OQ-10 - Does Start have an `AlliesAllowed=false` special rejection? -> No scoped evidence; ordinary Start validation uses capacity/no-opponent/same-explicit-team and then packs item data.` (evidence: `0x006ACEE0`)
- `[RESOLVED] OQ-11 - Does Team Game force all rows to the same team? -> No; it forces explicit-team selection by omitting `None`; local defaults to A, AI defaults to Team D when allies are allowed.` (evidence: `0x004E5B60`; `0x006AE6E0`)
- `[RESOLVED] OQ-12 - How are team values packed? -> active AI teams go to `DAT_00A8B2FC[slot]`; local team goes to `DAT_00A8B3A4` and node `+0x63`.` (evidence: `0x006AD4DF..0x006AD4E6`; `0x006AD61C..0x006AD641`)
- `[RESOLVED] OQ-13 - Does current Rust gate Team rows by selected mode? -> No; `combo_items` returns static `[-2,0,1,2,3]`.` (evidence: `src/ui/skirmish_shell/state.rs:1260`)
- `[RESOLVED] OQ-14 - Does current Rust carry selected launch mode? -> Not yet; launch packs `SkirmishLaunchMode::Battle`.` (evidence: `src/ui/skirmish_shell/state.rs:1961`; `src/skirmish_launch.rs:14`)
- `[DEFERRED] OQ-15 - Full WOL/network Team combo branch.` (category: out-of-scope; reason: user explicitly scoped offline Skirmish, not multiplayer netcode; next-step-if-pursued: trace `g_GameMode==3||4` branches in `0x004E5B60`/`0x004E5D60`)
- `[DEFERRED] OQ-16 - Complete indirect caller census for vtable `+0x34`.` (category: bounded-cost-too-high; reason: validator semantics are proven and direct Start path was checked; full callback census is not needed for row-control handoff; next-step-if-pursued: scan MPMode vtable indirect calls)
- `[DEFERRED] OQ-17 - Post-launch House alliance consumer for `AlliesAllowed=false` explicit teams.` (category: out-of-scope; reason: target is setup/session parity, not house alliance map/netcode; next-step-if-pursued: use the Team adjunct House alliance handoff slot/report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Team combo rows depend on selected mode `MustAlly`: `MustAlly=false` inserts `None(-2),A-D(0..3)`; `MustAlly=true` inserts only `A-D` and selects row 0. | `0x004E5B60`; assembly `0x005D5DC0..0x005D5DCD`; Team payload report | mismatch: static Team rows always include `-2` | `src/ui/skirmish_shell/state.rs::combo_items`, selected-mode plumbing | Build Team combo item list from selected `SkirmishGameMode.must_ally` and repair/open dropdown selection after mode changes. | Select Battle: local Team combo shows `None,A,B,C,D`; select Team Game: local and AI Team combos show `A,B,C,D` only, with local default `A`. | Do not leave `None` selectable in Team Game; do not pre-disable the combo just because `MustAlly` is true. |
| `AlliesAllowed=false` does not remove A-D or disable active Team combos; it defaults AI Team controls to `None(-2)` instead of Team D. | `0x006ADC20`; `0x006AE6E0`; assembly `0x005D5DD0..0x005D5DDD` | mismatch/partial: default opponents all start `-2`, and row changes do not apply selected-mode ally default | `default_opponents`, AI row-state change handler in `src/ui/skirmish_shell/state.rs`, mode-change accept path | On selected-mode refresh and AI row-state changes, set affected AI team to `3` when allies allowed, else `-2`; enable/disable controls from row active state, not from `AlliesAllowed`. | Battle/Team Game active/default AI rows select Team D; FFA/Coop active/default AI rows select None while still exposing A-D. | Do not interpret `AlliesAllowed=false` as "hide/disable the Team combo" or "delete A-D rows." |
| Team validator semantics are `0..3`, plus `-2` only when `MustAlly=false`; `-1` is not a Team value. | assembly `0x005D5DE0..0x005D5E08`; `0x004E5B60` row insertion | partial risk: `LaunchTeam::from_shell_value` maps all negatives to None | `src/skirmish_launch.rs::LaunchTeam`, shell item application/validation | Treat only `-2` as native Team None in shell/session validation; reject or repair `-1` and out-of-range values before launch. | A synthetic attempt to set Team `-1` is not packed as a valid team; Team Game cannot launch with `LaunchTeam::None` from stale shell state. | Do not reuse AI row-state `-1` as Team Auto/None. |
| Start same-team validation is independent of `AlliesAllowed`; it rejects only when local team is explicit and every active AI has that same explicit team. | decompile `0x006ACEE0`; modal reports | mostly present for current Battle-only launch, but selected-mode repair missing | `launch_session` in `src/ui/skirmish_shell/state.rs`; `LaunchValidationError` if mode repair rejects stale values | Keep same-team validation on actual item data after mode-specific repair; do not add FFA/Coop-only setup modal without binary evidence. | FFA with local None and AI None launches past same-team validation; any mode with local Team A and all active AIs Team A shows the same-team failure. | Do not special-case `AlliesAllowed=false` as a Start rejection in this scoped path. |
| Start packing stores active AI Team values and local Team value after validation; selected mode `+0x14` has no ordinary `AlliesAllowed` special modal in this slice. | `0x006AD4DF..0x006AD4E6`; `0x006AD61C..0x006AD641`; `0x006AD2BA..0x006AD34B` | partial: session mode remains Battle-only and no selected mode data is packed | `src/skirmish_launch.rs`, `launch_session`, app session handoff | Carry selected mode id/data into launch session and pack repaired Team values consistently with native item data. | Launch Team Game: session records mode id 9 and explicit local/AI teams; Launch FFA/Coop: session records selected mode and default `None` teams unless user selected explicit teams. | Do not keep launch session as `Battle` while non-Battle modes are selectable. |

## Negative Facts / Do Not Do

- Do not use base `rulesmd.ini:[MultiplayerDialogSettings] AlliesAllowed=no` to decide Team combo rows. The selected MPMode object defaults `AlliesAllowed=true` and reads per-mode override payloads. Active in YR: Yes; evidence `0x005D5B60`.
- Do not let `MustAlly=true` survive with `AlliesAllowed=false`; the constructor clears it. Active in YR: Yes; evidence `0x005D5B60`, prior range `0x005D5D05..0x005D5D11`.
- Do not hide or disable Team combos merely because `AlliesAllowed=false`. Active FFA/Coop rows still use the same Team combo population; `AlliesAllowed` changes default selection to `-2`. Active in YR: Yes; evidence `0x004E5B60`, `0x006ADC20`.
- Do not add a visible offline Team `Auto` row or pack `-1` as a Team value. Active in YR: Yes; evidence `0x004E5B60`, `0x005D5DE0..0x005D5E08`.
- Do not force all Team Game players onto one team. Team Game requires explicit teams by omitting `None`; it defaults local to A and AI rows to D when allies are allowed. Active in YR: Yes; evidence `0x004E5B60`, `0x006AE6E0`.
- Do not add an `AlliesAllowed=false` Start modal without separate binary evidence. The scoped Start path validates capacity, no opponent, and same explicit team; it then packs actual Team item data. Active in YR: Yes; evidence `0x006ACEE0`.
- Do not confuse Team controls `0x76D..0x774` with start-position controls `0x6A3..0x6AB`; their sentinel `-2` meanings are separate UI families. Active in YR: Yes; evidence prior start-position and team reports.

## Remaining Uncertainty

- Full WOL/network Team combo behavior is intentionally not covered.
- Complete indirect caller census for vtable `+0x34` remains deferred; the validator bytes and standard UI/Start path behavior are sufficient for this offline handoff.
- Exact hashed MIX directory filename attribution for override payloads remains inherited Medium confidence from the payload report; the common field values and binary readers are high confidence.
- Post-launch House alliance interpretation of explicit teams when selected mode has `AlliesAllowed=false` belongs to the adjacent Team adjunct/House alliance handoff, not this row-control slice.

## Sources

- Ghidra read-only decompile/recheck: `0x004E5B60`, `0x004E5D60`, `0x004E5ED0`, `0x004E6030`, `0x005D5B60`, `0x006ADC20`, `0x006AE6E0`, `0x006ACEE0`.
- Ghidra read-only assembly contexts: `0x005D5DC0..0x005D5DCD`, `0x005D5DD0..0x005D5DDD`, `0x005D5DE0..0x005D5E08`, `0x006AD2BA..0x006AD34B`, `0x006AD4DF..0x006AD4E6`, `0x006AD61C..0x006AD641`.
- INI/data checked: `C:/Users/enok/Documents/ra2-rust-game/ini/mpmodesmd.ini`, `ini/rulesmd.ini`, `ini/rules.ini`.
- Prior reports referenced: `SKIRMISH_MPMODES_OVERRIDE_PAYLOAD_VALUES_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_SESSION_PACKING_BROAD_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_TEAM_COMBO_SENTINEL_LABELS_AND_VALUES_GHIDRA_REPORT.md`, `SKIRMISH_TEAM_NONE_INSERTION_VTABLE_0X2C_GHIDRA_REPORT.md`, `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_CURRENT_CONTRACT_RECHECK_GHIDRA_REPORT.md`.
- Rust scanned: `C:/Users/enok/Documents/ra2-rust-game/src/skirmish_modes.rs`, `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/app_skirmish.rs`.
