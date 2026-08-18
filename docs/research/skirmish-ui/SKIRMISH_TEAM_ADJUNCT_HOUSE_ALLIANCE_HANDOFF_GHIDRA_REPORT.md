# Skirmish Team Adjunct House Alliance Handoff - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x004E5B60`, `0x00687F10`, `0x00686990`, `0x005D74A0`, `0x005D6BE0`, `0x004F9B70`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish team combo item data and selected-mode gates through Start Game packing, `House+0x16058` / `House+0x1605C` house writes, selected-mode same-team alliance callback, and current Rust launch/session handoff.  
**Non-Scope:** WOL/network lobby team controls, full non-Battle mode callback taxonomy, alliance notification/EVA timing, post-launch diplomacy UI, and multiplayer netcode.  
**Confidence:** High for the scoped offline Skirmish Battle-style path; Medium only for adjacent non-Battle mode variants explicitly deferred below.  
**Active in YR:** Yes for standard offline Skirmish dialog `0x102` and selected Battle/TeamGame-style session initialization.

Target question: Reconcile Team combo / launch team values with `House+0x1605C` team/alliance adjunct and same-team mutual alliance handoff in offline Skirmish.  
Non-goals: Do not expand into multiplayer netcode, WOL lobby variants, full MPModes taxonomy, or Rust implementation.  
Evidence needed to mark COMPLETE: decompile/caller evidence for final team packing and house writes, read-only disassembly/vtable proof for sentinels and selected-mode callbacks, `MakeAlly` effects, and current Rust surface scan.  
Stop conditions: all scoped values and handoff boundaries resolved or explicitly deferred; no Ghidra mutations; write only this report plus the shared claims file.

## 1. Overview

Offline Skirmish Team values are signed shell item data: `None = -2`, explicit teams `A..D = 0..3`, and no standard offline Team `Auto/-1` row exists. Start Game packs Team separately from Start: local Team goes to node `+0x63`, AI Team goes to `DAT_00A8B2FC[slot]`, and `ScenarioClass__Create_Houses` copies those values into `House+0x1605C`.

`House+0x1605C` is the team/alliance adjunct. It is not the explicit start field. Explicit start preassignment uses `House+0x16058`. Later, `ScenarioClass__Post_Map_Init` calls the selected mode vtable `+0x88`; Battle binds that slot to the same-team alliance pass at `0x005D74A0`, which mutual-allies non-special houses with equal non-sentinel `+0x1605C`.

Current Rust already has the launch-level same-explicit-team alliance shape (`LaunchTeam`, `launch_session`, `launch_alliance_map`) and keeps start/team separate. The remaining Rust gaps are focused acceptance tests plus selected-mode UI gating/repair for Team `None` and inactive-row defaults.

## 2. Class Layout / Key Offsets

| Field / source | Meaning | Evidence | Active in YR |
|---|---|---|---|
| Team combo `0x76D..0x774` | local row plus AI rows 1..7 team controls | `FUN_004E5940` decompile maps rows to control IDs | Yes |
| Team item data `-2` | visible Team `None` when selected mode allows it | `0x004E5B60` decompile inserts item data `-2` after selected-mode `+0x2C < 0` | Yes/Conditional by `MustAlly` |
| Team item data `0..3` | Team `A..D`, zero-based | `0x004E5AC0` loads letter strings; `0x004E5B60` inserts loop data `0..3` | Yes |
| `MPModes +0x3C` | `AlliesAllowed`; inactive-row default helper returns Team D (`3`) when true, `-2` when false | disassembly `0x005D5DD0..0x005D5DDD`; reads in `0x006ADC20` / `0x006AE6E0` | Yes/Conditional by selected mode |
| `MPModes +0x3F` | `MustAlly`; suppresses Team `None` and rejects `-2` in validator | disassembly `0x005D5DC0..0x005D5E08`; caller `0x004E5B60` | Yes/Conditional by selected mode |
| local node `+0x5B` / AI `DAT_00A8B2DC[slot]` | explicit start item data | `0x006AD4C7..0x006AD4D4`, `0x006AD60C..0x006AD63B`; copied to `House+0x16058` by `0x00687F10` | Yes |
| local node `+0x63` / AI `DAT_00A8B2FC[slot]` | team/alliance adjunct item data | `0x006AD4DB..0x006AD4E6`, `0x006AD61C..0x006AD641`, node store `0x006AD694..0x006AD699`; copied to `House+0x1605C` by `0x00687F10` | Yes |
| `House+0x16058` | selected explicit start preassignment | disassembly `0x005D6C12`, scenario table write `0x005D6C2F` | Yes |
| `House+0x1605C` | selected team/alliance adjunct | `0x006880FC..0x00688101`, `0x006881F5..0x006881FB`, disassembly consumer `0x005D74D4..0x005D7503` | Yes |

## 3. Core Logic

### 3.1 Team combo values and sentinels

`FUN_004E5B60(hwnd, control_id)` rebuilds one Team combo. It clears the control, sets shell combo messages `0x4DD` and `0x4DE`, calls the selected mode vtable `+0x2C`, and inserts Team `None` only when that callback returns a negative value. The offline string id is `0x45F` from `GDlgSupp.cpp`, and the stored item data is `-2`. It then appends the four letter rows from `DAT_008B3FC0..0x008B3FE4` and stores item data `0`, `1`, `2`, `3`. Active in YR: Yes; decompile `0x004E5B60`, label setup `0x004E5AC0`, fanout `0x004E5D60`.

The selected-mode callbacks are tiny functions that Ghidra has not defined as separate functions in this read-only project, so they were decoded from bytes:

| Callback | Disassembly result | Effect | Active in YR |
|---|---|---|---|
| vtable `+0x2C` at `0x005D5DC0` | `mov al,[ecx+0x3F]`; boolean arithmetic returns `-2` when `MustAlly == 0`, else `0` | controls Team `None` insertion | Yes |
| vtable `+0x30` at `0x005D5DD0` | `mov al,[ecx+0x3C]`; boolean arithmetic returns `3` when `AlliesAllowed != 0`, else `-2` | controls inactive AI team default | Yes |
| vtable `+0x34` at `0x005D5DE0` | if `MustAlly != 0 && value == -2`, returns false; otherwise accepts signed `0..3` only | rejects `-1`, `4+`, and Team `None` in MustAlly modes | Yes as callback |

Tiny detail: `-1` is not a standard offline Team row. It is used by the adjacent AI row-state combo for inactive rows, but the Team validator uses a signed `<0` rejection after the `-2` special case, so `-1` is not accepted as Team. Active in YR: Yes; evidence `0x005D5DF5..0x005D5E08`.

### 3.2 Start Game packing boundary

`FUN_006ACEE0` is the offline Skirmish Start Game handler for command `0x617`. In the active AI row loop, it writes:

- row type to `DAT_00A8B27C[slot]`;
- country to `DAT_00A8B29C[slot]`;
- color to `DAT_00A8B2BC[slot]`;
- start to `DAT_00A8B2DC[slot]`;
- team to `DAT_00A8B2FC[slot]`.

The critical order and destination split are visible in decompile and bytes around `0x006AD4C7..0x006AD4E6`: it calls `FUN_004E5900(-1)` for Start, stores to `DAT_00A8B2DC[slot]`, then calls `FUN_004E6030(-1)` for Team and stores to `DAT_00A8B2FC[slot]`. Local row mirrors the same split: Start reads `0x6A3` into `DAT_00A8B39C`, Team reads `0x76D` into `DAT_00A8B3A4`, then the newly allocated node receives start at `+0x5B` and team at `+0x63`. Active in YR: Yes; decompile `0x006ACEE0`, assembly bytes `0x006AD4C0..0x006AD4EF`.

Start validation uses the same signed team convention. If local Team is negative, same-team validation is skipped. If local Team is explicit (`>= 0`) and every active AI has the same explicit team, native shows the cannot-ally modal and returns before final packing. Active in YR: Yes; decompile `0x006ACEE0`, branch `0x006AD16C..0x006AD236`.

### 3.3 House writes and field separation

`ScenarioClass__Create_Houses @ 0x00687F10` copies the shell/session values into `HouseClass` fields:

- local/human path: `NodeNameTag__GetTeam()` writes `House+0x16058`, then node `+0x63` writes `House+0x1605C`;
- AI path: `DAT_00A8B2DC[slot]` writes `House+0x16058`, then `DAT_00A8B2FC[slot]` writes `House+0x1605C`;
- AI team value `!= -1` also sets `ScenarioClass+0x11E0 = 1`, but that is a separate scenario flag and not the alliance consumer.

Active in YR: Yes. This is the non-campaign Skirmish house creation path; evidence decompile `0x00687F10`, writes near `0x006880F2..0x00688101` and `0x006881EC..0x006881FB`.

Explicit start then reads `House+0x16058`, not `+0x1605C`. The Battle vtable `+0x80` target at `0x005D6BE0` calls `ScenarioClass__Gather_Start_Positions`, skips special houses, reads `House+0x16058`, skips only `-2`, and writes the house index into `ScenarioClass+0x1180 + start_index*4`. Active in YR: Yes; read-only disassembly `0x005D6BE0..0x005D6C2F`.

### 3.4 Same-team alliance handoff

`ScenarioClass__Post_Map_Init @ 0x00686990` calls selected mode vtable `+0x88` after house creation, start assignment, base/unit setup, and player/current-house initialization. For the standard Battle vtable at `0x007EE184`, read-only memory decodes:

- `+0x80 -> 0x005D6BE0` explicit start preassignment;
- `+0x84 -> 0x005D6C70` alternate/fallback start helper;
- `+0x88 -> 0x005D74A0` same-team alliance pass;
- `+0x8C -> 0x005D7570` follow-up mode callback.

`0x005D74A0` is another missing function boundary in this read-only Ghidra project, but the bytes decode cleanly. It loops unordered house pairs. For each outer house it skips `HouseType+0x1A6` special houses, reads `House+0x1605C`, and skips outer values `-2` and `-1`. The inner loop starts at the next house index, skips special inner houses, compares `outer+0x1605C == inner+0x1605C`, and calls `HouseClass__MakeAlly @ 0x004F9B70` twice: outer to inner and inner to outer. Active in YR: Yes; caller evidence is `ScenarioClass__Post_Map_Init` vtable `+0x88`, vtable memory `0x007EE184`, and disassembly `0x005D74A0..0x005D7549`.

`HouseClass__MakeAlly` is a gameplay mutation, not bookkeeping. It early-outs when the target is already non-enemy, sets the ally bit, rebuilds threat data, clears `EnemyHouseIndex` if it pointed at the new ally, may recalculate alliances/notifications under current-player/rules gates, and calls `FUN_004F42F0(0)` before returning. Active in YR: Yes; decompile `0x004F9B70`.

## 4. INI Keys

| Key / source | Default / effect | Evidence | Active in YR |
|---|---|---|---|
| `ini/mpmodesmd.ini [Battle] 1=GUI:Battle...MPBattleMD.ini...true` | stock Battle mode; `must_ally=false`, `allies_allowed=true` in current Rust's verified stock defaults | `ini/mpmodesmd.ini:7..9`; Rust `src/skirmish_modes.rs` | Yes |
| `ini/mpmodesmd.ini [Battle] 9=GUI:TeamGame...MPTeamMD.ini...false` | Team Game mode; Rust stock defaults set `must_ally=true`, `allies_allowed=true`; native callback suppresses Team `None` | `ini/mpmodesmd.ini:7..10`; disassembly `0x005D5DC0`; Rust `src/skirmish_modes.rs` | Yes/Conditional |
| `ini/mpmodesmd.ini [FreeForAll] 2=...MPFreeForAllMD.ini...true` | FFA mode; current Rust stock defaults set `allies_allowed=false`, which maps inactive AI team default to `-2` | `ini/mpmodesmd.ini:21..22`; disassembly `0x005D5DD0`; Rust `src/skirmish_modes.rs` | Yes/Conditional |
| `[MultiplayerDialogSettings] AlliesAllowed=no` in `rulesmd.ini` | base setting exists, but scoped Team combo gates read selected mode object fields, not this base value directly | `ini/rulesmd.ini:3017..3038`; selected-mode reads `0x005D5DD0`, `0x006ADC20`, `0x006AE6E0` | Conditional; do not use directly for combo rows |

No rules/art INI key directly writes `House+0x1605C`; it is a shell/session value copied through node/AI arrays during house creation.

## 5. Integration Points

The scoped offline order is:

1. Dialog setup `0x006AE6E0` initializes Team label table, refreshes Team combos, selects the mode object, refreshes again, and applies inactive-row defaults.
2. Team combo population `0x004E5B60` uses selected-mode `+0x2C` to include or suppress `None`, then appends `A..D`.
3. Start Game `0x006ACEE0` validates capacity/no-opponent/same-explicit-team, then packs Start and Team into distinct destinations.
4. `ScenarioClass__Create_Houses @ 0x00687F10` copies Start to `House+0x16058` and Team to `House+0x1605C`.
5. Start assignment consumes `House+0x16058` through selected mode `+0x80`.
6. `ScenarioClass__Post_Map_Init @ 0x00686990` invokes selected mode `+0x88`; Battle `+0x88` consumes `House+0x1605C` for same-team mutual alliances.

This is offline Skirmish/session setup. It is not multiplayer netcode. Active in YR: Yes for the standard offline shell and selected Battle-style launch path.

## 6. Current Rust Implementation Status

| Rust surface | Current status | Evidence |
|---|---|---|
| `src/skirmish_launch.rs::LaunchTeam` | collapses any negative shell team value to `None`, non-negative to `Team(u8)` | `LaunchTeam::from_shell_value`, tests at lines `255..257` |
| `src/ui/skirmish_shell/state.rs::combo_items` | still returns static Team rows `[-2,0,1,2,3]` for every mode | `src/ui/skirmish_shell/state.rs:1260..1305` |
| `src/ui/skirmish_shell/state.rs::launch_session` | same-explicit-team validation, then packs local/opponent `team` through `LaunchTeam::from_shell_value` | `src/ui/skirmish_shell/state.rs:1914..1995` |
| `src/app_skirmish.rs::apply_skirmish_launch_session` | installs launch alliance map before spawning MCVs | `src/app_skirmish.rs:162..205` |
| `src/app_skirmish.rs::launch_alliance_map` | builds bidirectional alliances for matching explicit `LaunchTeam::Team(_)`; ignores `None` | `src/app_skirmish.rs:324..351` |
| `src/skirmish_modes.rs` | has stock `allies_allowed` / `must_ally` data, but UI Team combo does not yet consume it | `src/skirmish_modes.rs:28..78`, tests `134..150` |

Observed Rust status: the launch/session alliance handoff mostly matches the native `House+0x1605C` equality result for explicit teams, but it lacks focused acceptance tests for same-team mutual alliance, sentinel non-alliance, and start/team independence. The shell UI is still incomplete for selected-mode gating: Team Game should omit/repair `None`; FFA/Coop inactive AI defaults should use `-2`; Battle/TeamGame inactive disabled AI rows should default Team D (`3`).

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes / scope gate | verified | this report top lines | none |
| Team label table `0x004E5AC0` | verified | Ghidra decompile | none |
| Team combo population `0x004E5B60` | verified | Ghidra decompile | none |
| selected-mode `+0x2C/+0x30/+0x34` callbacks | verified | read-only bytes/disassembly `0x005D5DC0..0x005D5E08`, vtable memory `0x007EE184` | no Ghidra function boundary; no mutation needed |
| inactive AI default branch `0x006ADC20` / `0x006AE6E0` | verified | Ghidra decompile | none |
| Start Game final packing `0x006ACEE0` | verified | Ghidra decompile plus bytes `0x006AD4C0..0x006AD4EF` | none |
| `ScenarioClass__Create_Houses` writes | verified | Ghidra decompile `0x00687F10` | none |
| explicit start consumer `0x005D6BE0` | verified | read-only disassembly `0x005D6BE0..0x005D6C2F` | no Ghidra function boundary; no mutation needed |
| selected mode `+0x88` invocation | verified | `ScenarioClass__Post_Map_Init @ 0x00686990` decompile | none |
| same-team alliance pass `0x005D74A0` | verified | read-only disassembly `0x005D74A0..0x005D7549`, vtable memory `0x007EE184 + 0x88` | no Ghidra function boundary; no mutation needed |
| `HouseClass__MakeAlly` effects | verified | Ghidra decompile `0x004F9B70` | deeper notification/EVA timing out of scope |
| WOL/network Team combo branch | deferred | online branch visible in `0x004E5B60` | out-of-scope for offline Skirmish |
| non-Battle `0x1605C` mode variants | deferred | prior reports identify alternate `0x005C3220` path | out-of-scope |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - What exact offline Team combo values are packed? -> Optional `-2` None plus explicit teams `0..3`; no standard offline `-1` Team row.` (evidence: `0x004E5B60`, `0x005D5DE0..0x005D5E08`)
- `[RESOLVED] OQ-02 - What controls Team None insertion? -> selected mode vtable `+0x2C`, backed by `MustAlly +0x3F`; Team Game suppresses None.` (evidence: caller `0x004E5B60`, disassembly `0x005D5DC0..0x005D5DCD`)
- `[RESOLVED] OQ-03 - What controls inactive AI team default? -> selected mode `AlliesAllowed +0x3C`; true selects Team D (`3`), false selects `-2`.` (evidence: `0x006ADC20`, `0x006AE6E0`, `0x005D5DD0..0x005D5DDD`)
- `[RESOLVED] OQ-04 - Where does Start Game write Team versus Start? -> Start writes node `+0x5B` / `DAT_00A8B2DC`; Team writes node `+0x63` / `DAT_00A8B2FC`.` (evidence: `0x006ACEE0`, `0x006AD4C7..0x006AD4E6`, `0x006AD60C..0x006AD699`)
- `[RESOLVED] OQ-05 - Which House fields receive those values? -> `House+0x16058` receives explicit start; `House+0x1605C` receives team/alliance adjunct.` (evidence: `0x00687F10`)
- `[RESOLVED] OQ-06 - Does `House+0x1605C` affect start assignment? -> No; Battle `+0x80` reads `House+0x16058` and writes the start table.` (evidence: disassembly `0x005D6BE0..0x005D6C2F`)
- `[RESOLVED] OQ-07 - How is same-team alliance handed off? -> `ScenarioClass__Post_Map_Init` calls selected mode `+0x88`; Battle binds that to `0x005D74A0`, which mutual-allies equal non-sentinel `House+0x1605C` pairs.` (evidence: `0x00686990`, vtable memory `0x007EE184`, disassembly `0x005D74A0..0x005D7549`)
- `[RESOLVED] OQ-08 - Which sentinels are ignored by the alliance pass? -> outer `-2` and `-1`; inner sentinels cannot match because outer sentinels are skipped.` (evidence: `0x005D74D4..0x005D74E2`)
- `[RESOLVED] OQ-09 - Is the resulting alliance one-way? -> No; matching pairs call `HouseClass__MakeAlly` in both directions.` (evidence: `0x005D750B..0x005D751A`)
- `[RESOLVED] OQ-10 - Does Rust have the handoff shape? -> Mostly yes for explicit team alliances; `launch_alliance_map` builds bidirectional alliances for matching `LaunchTeam::Team(_)` and ignores `None`.` (evidence: `src/app_skirmish.rs:324..351`)
- `[RESOLVED] OQ-11 - Does Rust keep start and team independent? -> Yes structurally; separate fields feed `assign_launch_starts` and `launch_alliance_map`.` (evidence: `src/app_skirmish.rs:185..190`, `src/app_skirmish.rs:324..405`)
- `[RESOLVED] OQ-12 - What Rust tests remain missing? -> No focused app-level regression currently asserts same-team mutual alliance, sentinel non-alliance, or start/team independence through `apply_skirmish_launch_session`.` (evidence: `rg` over `src/app_skirmish.rs` only finds color/name/start tests, no alliance tests)
- `[DEFERRED] OQ-13 - Full WOL/network Team combo semantics.` (category: out-of-scope; reason: target is offline Skirmish, and `0x004E5B60` has a separate `g_GameMode == 3 || 4` branch; next-step-if-pursued: investigate `0x004E5CB0` and WOL row ownership)
- `[DEFERRED] OQ-14 - Exact non-Battle mode object semantics for alternate `0x1605C` readers.` (category: out-of-scope; reason: standard offline Battle-style handoff is fully resolved; next-step-if-pursued: enumerate all selected-mode vtables that bind `0x005D74A0` or `0x005C3220`)
- `[DEFERRED] OQ-15 - Player-facing alliance notification/EVA timing inside `MakeAlly`.` (category: out-of-scope; reason: this report claims launch alliance state, not notification presentation timing; next-step-if-pursued: dedicated `MakeAlly` notification trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Equal explicit `House+0x1605C` values `0..3` produce mutual alliances before normal play | `0x00686990`, vtable `0x007EE184 + 0x88`, disassembly `0x005D74A0..0x005D751A`, `0x004F9B70` | mostly implemented; focused regression missing | `src/app_skirmish.rs::launch_alliance_map`, `apply_skirmish_launch_session` | Preserve bidirectional alliance creation for matching `LaunchTeam::Team(_)` slots | Local and one AI on Team A start allied to each other; AI on Team B remains enemy | Do not make same-team alliance one-way or defer it until after gameplay begins |
| Team sentinels `-2` and `-1` do not auto-ally through the native equality pass | `0x005D74D4..0x005D74E2`; Team validator `0x005D5DF5..0x005D5E08` | behavior currently matches for negative values via `LaunchTeam::None`; distinction is collapsed | `src/skirmish_launch.rs::LaunchTeam`, `src/ui/skirmish_shell/state.rs::launch_session` | Keep negative team values out of explicit alliance groups | Two slots left on Team None/default do not become allies just because both are default | Do not treat equal negative/default values as real teams |
| `House+0x16058` is explicit start and `House+0x1605C` is team/alliance adjunct | `0x00687F10`, `0x005D6BE0..0x005D6C2F`, `0x005D74A0..0x005D751A` | current Rust structurally separates start assignment and alliances; focused regression missing | `src/app_skirmish.rs::assign_launch_starts`, `src/app_skirmish.rs::launch_alliance_map` | Keep waypoint assignment independent from team grouping | Player chooses Start 2 and Team A: spawn uses Start 2 while alliance depends only on Team A peers | Do not collapse start and team into one launch field |
| Team Game suppresses Team `None`; selected-mode validator rejects `-2` when `MustAlly` is true | `0x004E5B60`, `0x005D5DC0..0x005D5E08`; stock mode defaults in `src/skirmish_modes.rs` | missing in visible UI; `combo_items` always returns `[-2,0,1,2,3]` | `src/ui/skirmish_shell/state.rs::combo_items`, mode-change repair/validation path | Team combo rows must depend on selected mode `must_ally`; invalid `-2` selections must be repaired/rejected | Select Battle: rows `None,A,B,C,D`; select Team Game: rows `A,B,C,D`, no packed `-2` | Do not model Team `None` as an unconditional static row |
| Inactive AI team default follows `AlliesAllowed`: false -> `-2`, true -> Team D (`3`) | `0x006ADC20`, `0x006AE6E0`, `0x005D5DD0..0x005D5DDD` | missing/unchecked; opponent default currently starts `-2` and is not refreshed by mode | `src/ui/skirmish_shell/state.rs` row activation and selected-mode refresh | Disabled AI rows should carry the native selected-mode team default | FFA/Coop inactive rows select `None`; Battle/Team Game inactive rows select Team D after mode refresh | Do not conflate disabled-row defaults with active launch alliance groups |
| Same-explicit-team Start validation blocks only when local team is explicit and all active AIs share it | `0x006ACEE0`, branch `0x006AD16C..0x006AD236` | implemented at data level | `src/ui/skirmish_shell/state.rs::launch_session` | Preserve skip when local team is negative and failure when all active opponents match explicit local team | Local Team None plus AI Team None launches; local Team A plus all active AIs Team A shows validation failure | Do not block merely because every row is sentinel/default |

Proposed focused tests:

- `skirmish_launch_same_explicit_team_creates_mutual_alliance`
- `skirmish_launch_team_sentinels_do_not_auto_ally`
- `skirmish_launch_start_position_and_team_are_independent`
- `team_game_must_ally_omits_team_none_combo_item`
- `inactive_ai_team_default_follows_allies_allowed`

## Negative Facts / Do Not Do

- Do not use `House+0x1605C` as the explicit start field. Active in YR: Yes; `0x005D6C12` reads `House+0x16058` for start assignment.
- Do not collapse start and team shell values. Active in YR: Yes; `0x006ACEE0` writes start and team to distinct destinations, then `0x00687F10` copies them to distinct House fields.
- Do not add a standard offline Team `Auto/-1` row. Active in YR: Yes; `0x004E5B60` inserts only optional `-2` plus `0..3`, and validator rejects `-1`.
- Do not ally every matching negative team value. Active in YR: Yes; `0x005D74DA..0x005D74E2` skips `-2` and `-1`.
- Do not make same-team alliances one-way. Active in YR: Yes; `0x005D750B..0x005D751A` calls `HouseClass__MakeAlly` in both directions.
- Do not use base `rulesmd.ini [MultiplayerDialogSettings] AlliesAllowed=no` directly as the Team combo gate. Active in YR: Conditional by selected mode object; the scoped reads are selected-object fields/callbacks.
- Do not treat the missing Ghidra function boundaries at `0x005D5DC0`, `0x005D6BE0`, and `0x005D74A0` as evidence gaps requiring mutation; read-only bytes, vtable memory, and caller evidence resolve the scoped behavior.

## Remaining Uncertainty

- WOL/network Team combo behavior is intentionally not claimed.
- Non-Battle selected-mode variants that use alternate `0x1605C` readers are not claimed.
- Player-facing alliance notification/EVA timing inside `HouseClass__MakeAlly` was not expanded; this report only claims launch-time alliance state.

## Sources

- Ghidra read-only decompile: `0x006ACEE0`, `0x004E5AC0`, `0x004E5B60`, `0x004E5D60`, `0x004E5940`, `0x006ADC20`, `0x006AE6E0`, `0x00687F10`, `0x00686990`, `0x004F9B70`.
- Ghidra read-only memory/disassembly decoded: `0x005D5DC0..0x005D5E08`, `0x005D6BE0..0x005D6C2F`, `0x005D74A0..0x005D7549`, vtable memory `0x007EE184..0x007EE224`.
- Prior docs checked: `SKIRMISH_HOUSE_0X1605C_TEAM_ADJUNCT_CONSUMER_GHIDRA_REPORT.md`, `SKIRMISH_HOUSE_0X1605C_TEAM_ALLIANCE_ADJUNCT_CONSUMER_GHIDRA_REPORT.md`, `SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md`, `SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md`, `SKIRMISH_TEAM_COMBO_SENTINEL_LABELS_AND_VALUES_GHIDRA_REPORT.md`.
- INI checked: `ini/mpmodesmd.ini`, `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/skirmish_launch.rs`, `src/skirmish_modes.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish.rs`, `src/app_init.rs`.
