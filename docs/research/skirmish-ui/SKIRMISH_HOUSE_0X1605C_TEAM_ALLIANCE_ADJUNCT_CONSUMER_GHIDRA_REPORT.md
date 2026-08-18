# Skirmish House+0x1605C Team Alliance Adjunct Consumer - Ghidra Research Report

**Address(es):** `0x00687F10`, `0x00686990`, `0x005D74A0`, `0x004F9B70`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** post-`ScenarioClass__Create_Houses` consumers of `HouseClass+0x1605C` in standard offline Skirmish/Battle-style launch, and whether the field affects teams, alliances, diplomacy, observers, AI targeting, or only bookkeeping.  
**Non-Scope:** Skirmish shell team combo population, Start validation UI, detailed start placement formulas, full non-Battle mode behavior, and post-launch runtime alliance-change UI.  
**Confidence:** High for the standard Battle-style writer, lifecycle invocation, equality consumer, and `HouseClass__MakeAlly` effects; Medium for non-Battle mode variants that share or replace this callback.  
**Active in YR:** Yes. Standard offline Skirmish uses the non-campaign `ScenarioClass__Full_Init` path and later `ScenarioClass__Post_Map_Init`, which invokes selected mode vtable `+0x88`; the Battle vtable binds `+0x88` to `0x005D74A0`.

## Working Notes

- Target question: Which systems consume `House+0x1605C` after `Create_Houses`, and does it affect Skirmish teams/alliances/diplomacy/observers/AI targeting or only bookkeeping?
- Non-goals: Do not redo shell team combo population or Start validation; do not expand into all MPModes; do not implement Rust.
- Evidence needed to mark COMPLETE: writer evidence from `Create_Houses`, lifecycle evidence that a consumer runs in standard Skirmish, consumer disassembly/decompile, `MakeAlly` side-effect proof, and Rust surface scan.
- Stop conditions: all static `+0x1605C` hits classified as behavioral, inactive/non-standard, or bookkeeping/debug; no unresolved standard-Battle consumer remains.

## 1. Overview

`House+0x1605C` is the Skirmish team/alliance adjunct, not the explicit start-position field. `ScenarioClass__Create_Houses @ 0x00687F10` writes it from node `+0x63` for humans and AI team array data for AI rows. The standard Battle-style consumer is the selected mode `+0x88` callback at `0x005D74A0`, invoked from `ScenarioClass__Post_Map_Init @ 0x00686990`; it mutual-allies non-special houses that share the same non-sentinel `0x1605C` value.

This field is player-visible through diplomacy/allied behavior. It does not create observers and does not assign starting positions. AI targeting is affected indirectly because `HouseClass__MakeAlly` mutates the ally bitset, clears the current enemy target when needed, rebuilds threat data, and triggers alliance recalculation paths.

## 2. Key Offsets

| Offset / source | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| node `+0x63` | local human team/adjunct item data copied to `House+0x1605C` | `0x006880FC..0x00688101` | Yes; `Create_Houses` in non-campaign Skirmish |
| AI array slot `+0x60` in the `DAT_00A8B29C` row walk, equivalent to `DAT_00A8B2FC[row]` | AI team/adjunct item data copied to `House+0x1605C` | `0x006881F5..0x006881FB` | Yes; active AI house loop |
| `House+0x16058` | explicit start-position field, included only as contrast | `0x005D6C12`, `0x005D6C2F` | Yes; Battle vtable `+0x80` |
| `House+0x34 -> HouseType+0x1A6` | special/non-player house filter for start and team callbacks | `0x005D74C6..0x005D74D0`, `0x005D74EB..0x005D74F6` | Yes |
| `House+0x1EC` | human/player-control flag used by alternate mode/debug alliance path, not by standard Battle `0x1605C` equality pass | `0x005C328F`, `0x005C33DC` | Conditional; different vtable path |

## 3. Core Logic

### Writer

`ScenarioClass__Create_Houses @ 0x00687F10` writes `House+0x1605C` during house construction:

- Human house path: `NodeNameTag__GetTeam()` writes `House+0x16058`, then node `+0x63` writes `House+0x1605C`.
- AI house path: AI start array data writes `House+0x16058`, then AI team array data writes `House+0x1605C`.
- For AI houses only, a team value other than `-1` sets `ScenarioClass+0x11E0 = 1`; this is a separate scenario flag, not a direct consumer of `House+0x1605C`.

Active in YR: Yes. `ScenarioClass__Full_Init @ 0x00686B20` calls `ScenarioClass__Create_Houses` in the non-campaign branch used by offline Skirmish.

### Standard Battle-style consumer

`ScenarioClass__Post_Map_Init @ 0x00686990` invokes selected mode callbacks after map/unit/base setup:

- if `DAT_00A8B23C != 0`, call selected mode vtable `+0x88`;
- then call selected mode vtable `+0x8C`.

For the standard Battle vtable base `0x007EE184`, binary `.rdata` reads:

| Vtable slot | Target |
|---|---|
| `+0x7C` | `0x005D6790` |
| `+0x80` | `0x005D6BE0` |
| `+0x84` | `0x005D6C70` |
| `+0x88` | `0x005D74A0` |
| `+0x8C` | `0x005D7570` |

`0x005D74A0` loops unordered house pairs:

1. Outer house loop walks `g_HouseClass_Array`; if no houses exist, it returns true.
2. It skips special houses through `HouseType+0x1A6`.
3. It reads outer `House+0x1605C`.
4. Outer value `-2` or `-1` suppresses the entire inner scan for that outer house.
5. Inner loop starts at the next house index, so each unordered pair is considered once.
6. It skips special inner houses.
7. It compares outer `House+0x1605C` to inner `House+0x1605C`.
8. On equality, it calls `HouseClass__MakeAlly @ 0x004F9B70` twice: outer -> inner, then inner -> outer.

Active in YR: Yes for standard Battle-style Skirmish. Evidence: `ScenarioClass__Post_Map_Init @ 0x00686990` calls selected mode `+0x88`; Battle vtable `0x007EE184 + 0x88` points to `0x005D74A0`; disassembly `0x005D74D4..0x005D751A` reads `+0x1605C` and calls `MakeAlly`.

### `MakeAlly` effects

`HouseClass__MakeAlly @ 0x004F9B70` is not bookkeeping-only:

- It early-outs if `HouseClass__Is_Enemy(otherHouse)` is false.
- It sets the caller's ally bit for `otherHouse->ArrayIndex`.
- It calls `HouseClass__AI_BuildThreatMap`.
- If the caller's `EnemyHouseIndex` equals the newly allied house index, it clears it to `-1`.
- In live non-campaign contexts, it can recalculate alliances and trigger player-facing alliance notification/EVA side effects depending on current-player and rules gates.
- It finishes with `FUN_004F42F0(0)`.

Active in YR: Yes. `HouseClass__MakeAlly` contains explicit `g_GameMode != 0`, `g_GameMode == 0`, `g_GameMode == 4`, and `g_MapEditorMode` branches; the ally bit mutation itself is unconditional after the enemy check.

## 4. Other Static `+0x1605C` Hits

The retail `gamemd.exe` static scan found nine little-endian `0x1605C` displacements:

| Address | Classification | Evidence | Active in YR |
|---|---|---|---|
| `0x00688103`, `0x006881FD` | writer in `Create_Houses` | disassembly/decompile `0x00687F10` | Yes |
| `0x005D74D6`, `0x005D74FF`, `0x005D7505` | standard Battle-style equality alliance consumer | disassembly `0x005D74A0..0x005D751A` | Yes |
| `0x005D755F` | tiny index accessor returning `g_HouseClass_Array[index]->+0x1605C`; no direct call or vtable pointer found in the static scan | direct-call and pointer scan found no references to `0x005D7550` | Not proven |
| `0x005C326B`, `0x005C33B8` | alternate vtable path at `0x005C3220`; reads `0x1605C` for logging/diagnostic text, but its alliance calls are gated by human/non-human control, not by `0x1605C` equality | pointer `0x005C3220` appears once at `.rdata 0x007EE304`, not standard Battle vtable `+0x88` | Conditional; non-standard mode path |
| `0x0064E266` | sync/debug dump prints house fields, including team and start values | decompile `FUN_0064DEA0`; writes sync text, not gameplay state | Conditional; diagnostic/bookkeeping |

## 5. Integration Points

The standard order is:

1. `ScenarioClass__Full_Init @ 0x00686B20` creates houses and writes `+0x16058/+0x1605C`.
2. Start assignment uses the selected mode `+0x80` and/or `+0x84` paths and consumes `House+0x16058`, not `+0x1605C`.
3. `ScenarioClass__Post_Map_Init @ 0x00686990` later invokes selected mode `+0x88`.
4. Battle `+0x88 -> 0x005D74A0` turns equal non-sentinel `+0x1605C` groups into mutual alliances.
5. `HouseClass__MakeAlly` mutates diplomacy and related targeting/threat state.

Observer handling is separate. The earlier Start-to-spawn report found observer marker handling through node `+0x6B` and `DAT_00AC1198`; no observer branch in this slice reads `House+0x1605C`.

## 6. Current Rust Implementation Status

Current Rust has most of the shape required for this specific Battle-style alliance pass:

| Rust surface | Status against this slice | Evidence |
|---|---|---|
| `src/skirmish_launch.rs::LaunchTeam` | represents negative shell values as `None` and non-negative values as `Team(u8)` | `LaunchTeam::from_shell_value` |
| `src/ui/skirmish_shell/state.rs::launch_session` | preserves local and AI team values into `SkirmishLaunchSession` | `team: LaunchTeam::from_shell_value(...)` |
| `src/app_skirmish.rs::launch_alliance_map` | creates bidirectional alliances for matching `LaunchTeam::Team(_)` slots and ignores `None` | `launch_alliance_map` |
| `src/app_skirmish.rs::apply_skirmish_launch_session` | installs `sim.house_alliances` before start spawning | call to `launch_alliance_map` |

Observed Rust deltas:

- No dedicated focused test currently asserts the native same-team alliance contract in `launch_alliance_map`.
- Rust collapses all negative shell team values to `LaunchTeam::None`; this matches the standard Battle equality pass for `-2` and `-1`, but future UI/state work should preserve the distinction if another mode or shell path needs it.
- Rust's loop checks all ordered pairs rather than native unordered pairs, but the resulting alliance map is idempotent and bidirectional; no observable mismatch was found for the current data structure.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Create_Houses` writer for humans | verified | `0x006880FC..0x00688101` | none |
| `Create_Houses` writer for AI rows | verified | `0x006881F5..0x006881FB` | none |
| standard Battle vtable `+0x88` binding | verified | `.rdata 0x007EE184 + 0x88 -> 0x005D74A0` | none |
| lifecycle invocation of selected mode `+0x88` | verified | `ScenarioClass__Post_Map_Init @ 0x00686990`, assembly `0x00686AD4..0x00686AF2` | none |
| `0x005D74A0` equality consumer | verified | disassembly `0x005D74D4..0x005D751A` | none |
| `HouseClass__MakeAlly` side effects | verified | decompile `0x004F9B70` | deeper visual/EVA timing out of scope |
| standard start preassignment contrast | verified | `0x005D6C12`, `0x005D6C2F` | none |
| alternate path `0x005C3220` | touched-not-exhausted | disassembly `0x005C3220..0x005C34E0`; vtable pointer at `0x007EE304` | non-Battle mode semantics out of scope |
| tiny accessor `0x005D7550` | touched-not-exhausted | static scan found no direct caller or pointer | runtime/debug-only use, if any |
| sync/debug dump reader | verified as bookkeeping | `FUN_0064DEA0`, read near `0x0064E264` | none |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is `House+0x1605C` the standard Battle start-position field? -> No; `House+0x16058` is read by the `+0x80` start preassignment callback.` (evidence: `0x005D6C12`, `0x005D6C2F`)
- `[RESOLVED] OQ-2 - Which standard Battle-style system consumes `House+0x1605C` after `Create_Houses`? -> The selected mode `+0x88` callback at `0x005D74A0`, invoked from `ScenarioClass__Post_Map_Init`, consumes it for same-team mutual alliances.` (evidence: `0x00686AE4`, `0x007EE20C`, `0x005D74D4..0x005D751A`)
- `[RESOLVED] OQ-3 - Which values are ignored by the standard equality pass? -> Outer values `-2` and `-1` are skipped; special houses are skipped on both sides.` (evidence: `0x005D74C6..0x005D74E2`, `0x005D74EB..0x005D74F6`)
- `[RESOLVED] OQ-4 - Is the resulting alliance one-way? -> No; matching pairs call `MakeAlly` both directions.` (evidence: `0x005D750B..0x005D751A`)
- `[RESOLVED] OQ-5 - Does it affect diplomacy/targeting or only bookkeeping? -> It affects diplomacy and targeting indirectly through `MakeAlly`: ally bitset, threat map, and enemy-house index updates.` (evidence: `0x004F9B70`)
- `[RESOLVED] OQ-6 - Does it create observers? -> No scoped observer consumer reads `+0x1605C`; observer handling remains tied to separate node/session markers from prior reports.` (evidence: no `+0x1605C` observer branch in static hit classification; prior `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-7 - Are there bookkeeping-only readers? -> Yes; sync/debug dump prints `+0x1605C` without mutating gameplay state.` (evidence: `FUN_0064DEA0`, `0x0064E264`)
- `[DEFERRED] OQ-8 - What exact non-Battle mode object uses `0x005C3220` at vtable pointer `0x007EE304`?` (category: out-of-scope; reason: the target is standard offline Skirmish/Battle-style consumer behavior; next-step-if-pursued: enumerate MPModes object construction and vtable assignment)
- `[DEFERRED] OQ-9 - Does the unreferenced `0x005D7550` accessor ever run through dynamic code or debug tooling?` (category: needs-runtime-debugger; reason: static direct-call/pointer scan found no reference; next-step-if-pursued: runtime breakpoint/watchpoint on `0x005D7550`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Equal non-sentinel team adjunct values create mutual alliances before normal play | `0x005D74A0`, `0x004F9B70` | mostly implemented; needs focused regression test | `src/app_skirmish.rs::launch_alliance_map`, `src/skirmish_launch.rs::LaunchTeam` | Keep per-slot team separate from start position and build bidirectional alliances for matching explicit teams | Local player and one AI both on Team 1 start allied; another AI on Team 2 remains enemy | Do not use start-position choices as team/alliance data |
| `-2` and `-1` are non-team sentinels for the standard equality pass | `0x005D74D4..0x005D74E2` | behavior currently matches via `LaunchTeam::None`; distinction not preserved | `src/skirmish_launch.rs`, `src/ui/skirmish_shell/state.rs` | Do not auto-ally rows whose team is Auto/None sentinel | Two slots with default team sentinel do not become allied at launch | Do not treat equal negative/default values as real teams |
| `House+0x16058` remains the start field and `House+0x1605C` remains the alliance adjunct | `0x005D6C12`, `0x005D6C2F`, `0x005D74D4` | partially implemented in separate launch start/team fields | `src/app_skirmish.rs::assign_launch_starts`, `src/app_skirmish.rs::launch_alliance_map` | Keep start assignment and alliance grouping independent | Player chooses Start 2 and Team 1: spawn uses Start 2 while alliances depend only on Team 1 peers | Do not let same-team values affect waypoint assignment or vice versa |

Proposed Rust test names:

- `skirmish_launch_same_explicit_team_creates_mutual_alliance`
- `skirmish_launch_team_sentinels_do_not_auto_ally`
- `skirmish_launch_start_position_and_team_are_independent`

## Negative Facts / Do Not Do

- Do not model `House+0x1605C` as the Battle explicit-start field. Evidence: `0x005D6C12` reads `House+0x16058`; `0x005D6C2F` writes the scenario start table.
- Do not ally default/sentinel teams just because their values match. Evidence: `0x005D74DA..0x005D74E2` skips outer `-2` and `-1`.
- Do not implement a one-way same-team alliance. Evidence: `0x005D750B..0x005D751A` calls `HouseClass__MakeAlly` in both directions.
- Do not route observer state through `House+0x1605C`. Evidence: this slice's static `+0x1605C` hits are writer/alliance/debug/accessor paths; observer marker evidence remains in prior node `+0x6B` path.
- Do not treat the sync/debug dump reader as gameplay behavior. Evidence: `FUN_0064DEA0` writes `SYNC*.TXT` diagnostics and only prints the field.

## Remaining Uncertainty

- Exact non-Battle mode semantics for the `0x005C3220` vtable path are out of scope.
- Static analysis found no caller for `0x005D7550`; a runtime breakpoint would be needed to prove it is never used.
- Exact timing of player-facing alliance notification/EVA side effects inside `MakeAlly` was not expanded, because the target was the `0x1605C` consumer and launch-time alliance state.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`
  - Replace OQ-6 with: `[RESOLVED] House+0x1605C is consumed by the standard Battle-style selected-mode +0x88 callback at 0x005D74A0, invoked from ScenarioClass__Post_Map_Init. Equal non-sentinel values produce bidirectional HouseClass__MakeAlly calls; -2 and -1 do not auto-ally.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`
  - Replace claims that `NodeNameTag+0x63` / `House+0x1605C` is a start location with: `House+0x16058 is the standard Battle explicit-start field; House+0x1605C is the team/alliance adjunct consumed by the selected-mode +0x88 alliance callback.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md`
  - Split mixed "Start position / Ally" wording into: `+0x16058 is start preassignment for the verified Battle consumer; +0x1605C is the team/alliance adjunct for same-team mutual alliances.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_HOUSE_0X1605C_TEAM_ADJUNCT_CONSUMER_GHIDRA_REPORT.md`
  - Rust status is stale: current Rust now has `LaunchTeam` and `launch_alliance_map`; the remaining immediate gap is focused regression coverage and preserving the sentinel/start/team separation.

## Sources

- Ghidra read-only decompile: `ScenarioClass__Create_Houses @ 0x00687F10`, `ScenarioClass__Full_Init @ 0x00686B20`, `ScenarioClass__Post_Map_Init @ 0x00686990`, `HouseClass__MakeAlly @ 0x004F9B70`, `FUN_0064DEA0`.
- Retail binary disassembly/static scan: `0x005D74A0..0x005D7550`, `0x005D6BE0..0x005D6C40`, `0x005C3220..0x005C34E0`, `0x0064E220..0x0064E2A0`.
- Retail binary vtable read: `0x007EE184 + 0x88 -> 0x005D74A0`.
- Prior docs checked: `skirmish-ui/SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_HOUSE_0X1605C_TEAM_ADJUNCT_CONSUMER_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_START_TEAM_CONTROL_DESTINATION_NAMING_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md`, `LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md`.
- Rust scan: `src/skirmish_launch.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish.rs`, `src/sim/world/mod.rs`.
