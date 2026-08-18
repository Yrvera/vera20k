# Skirmish MPModes Session Model Design

## Goal

Make offline Skirmish launch preserve the selected MPMode and use native mode
policy for Team rows/defaults, with mode common fields loaded from MIX-backed
`MP*.ini` overrides.

## Architecture Context

The current Skirmish shell already tracks the selected mode id in
`SkirmishShellState::selected_mode_id`, and Choose Map commits that id in
`App::commit_choose_map_selection`. The mode roster is parsed in
`src/skirmish_modes.rs` as `SkirmishGameMode`, including `override_file`,
`map_filter`, `random_maps_allowed`, `allies_allowed`, and `must_ally`.

The launch boundary is `src/ui/skirmish_shell/state.rs::launch_session`. It
currently receives shell state plus map entries only, then emits
`SkirmishLaunchSession { mode: SkirmishLaunchMode::Battle, ... }` regardless of
the selected mode. That session is saved in `AppState::pending_skirmish_launch_session`
and consumed by `src/app_skirmish.rs::apply_skirmish_launch_session` during map
initialization.

The data-loading boundary is already present through `AssetManager`, which can
look up archive-backed assets by filename. Existing rules/art loaders parse
archive bytes with `IniFile::from_bytes`. The Skirmish mode loader should follow
that pattern rather than growing a separate asset path.

This remains app/UI/session setup state. It should not introduce dependencies from
`sim/` to `ui/`, `render/`, `audio/`, or `net/`.

## Impact Analysis

Likely touched files:

- `src/skirmish_modes.rs`: replace stock filename switches with a mode override
  application path that can use `AssetManager`, while retaining a deterministic
  stock fallback for asset-less tests/startup.
- `src/app.rs`: initialize `skirmish_modes` from the startup asset manager when
  available.
- `src/skirmish_launch.rs`: change the Battle-only launch mode surface so the
  selected mode id and relevant metadata survive into the launch session.
- `src/ui/skirmish_shell/state.rs`: pass modes into `launch_session`, use selected
  mode data for launch packing, Team combo item population, and selected-mode
  repair/default behavior.
- `src/app_skirmish.rs`: add focused tests for same explicit team alliance,
  sentinel non-alliance, and start/team independence; behavior may remain mostly
  unchanged in this slice.

Primary risks:

- Existing tests call `launch_session(&shell, &maps)` and construct
  `SkirmishLaunchSession` with `SkirmishLaunchMode::Battle`; they need mechanical
  updates.
- Mode switching can leave stale `-2` Team selections in Team Game unless the
  shell repairs invalid selections.
- Asset-backed mode parsing must preserve current stock behavior when retail
  assets are unavailable, because unit tests and asset-less startup paths rely on
  `stock_skirmish_modes()`.
- Full non-Battle post-launch callbacks are not all researched; this design must
  preserve the selected mode identity without inventing unverified gameplay.

## Chosen Approach

Use an integrated data-driven slice:

1. Load common MPMode override fields from archive-backed `MP*.ini` files when an
   `AssetManager` is available.
2. Preserve selected mode id/data in `SkirmishLaunchSession`.
3. Make Team combo rows and AI Team defaults depend on the selected mode's
   `must_ally` and `allies_allowed` values.
4. Add tests that lock the launch/session contract and the already-mostly-correct
   alliance/start separation.

This approach is larger than a session-only patch, but it closes the visible
normal-play parity holes as one coherent change. It avoids a premature full
mode-callback framework; non-Battle `+0x80/+0x84` behavior remains a later
research-backed implementation.

## Tiny-Detail Ledger

- Selected mode id must survive Start; native copies `DAT_00A8B250` into
  `DAT_00A8B3C4` after selected-mode acceptance. Source:
  `SKIRMISH_SELECTED_MPMODE_START_LAUNCH_SESSION_GHIDRA_REPORT.md`,
  `0x006AD34B..0x006AD36B`.
- Choose Map commits selected mode object/id and selected map together. Source:
  `SKIRMISH_SELECTED_MPMODE_START_LAUNCH_SESSION_GHIDRA_REPORT.md`,
  `0x005E734F..0x005E7388`.
- Stock local roster exposes ids `1..9`; no stock offline Siege row exists even
  though the binary registers Siege support. Source: `ini/mpmodesmd.ini`,
  `SKIRMISH_SELECTED_MPMODE_START_LAUNCH_SESSION_GHIDRA_REPORT.md`.
- Start ordinary validation precedes selected-mode callback. Source:
  `SKIRMISH_SELECTED_MODE_START_VALIDATION_CONTRACT_GHIDRA_REPORT.md`,
  `0x006AD05B..0x006AD2D2`.
- Selected-mode false return alone is not blocking; native blocks only on
  `false && output_dword == 0x617`. Source:
  `SKIRMISH_SELECTED_MODE_START_VALIDATION_CONTRACT_GHIDRA_REPORT.md`,
  `0x006AD2D5..0x006AD346`.
- Generic selected-mode modal body comes from the output object; `0x469` is
  `TXT_OK`, not the message body. Source:
  `SKIRMISH_SELECTED_MODE_START_VALIDATION_CONTRACT_GHIDRA_REPORT.md`.
- Common MPMode object reads exactly four override keys:
  `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`,
  and `MustAlly`. Source: `SKIRMISH_MPMODES_MIX_OVERRIDE_PAYLOADS_GHIDRA_REPORT.md`,
  `0x005D5CA7..0x005D5D11`.
- Native common defaults are tournament flags true, `AlliesAllowed=true`,
  `MustAlly=false`; contradictory `MustAlly && !AlliesAllowed` clears
  `MustAlly`. Source: `SKIRMISH_MPMODES_MIX_OVERRIDE_PAYLOADS_GHIDRA_REPORT.md`,
  `0x005D5BEA..0x005D5D11`.
- `AllyChangeAllowed`, money/unit/tech/speed, checkboxes, `FogOfWar`, and
  `MCVRedeploys` are not common MPMode object fields. Source:
  `SKIRMISH_MPMODES_MIX_OVERRIDE_PAYLOADS_GHIDRA_REPORT.md`, `0x00671EA0`.
- Team `None` item data is `-2`; explicit Team A-D values are `0..3`; there is
  no standard offline Team `Auto/-1` row. Source:
  `SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md`,
  `0x004E5B60`.
- `MustAlly=true` suppresses Team `None`; `AlliesAllowed=false` does not remove
  Team A-D rows. Source:
  `SKIRMISH_MPMODES_MUSTALLY_ALLIESALLOWED_ROW_BEHAVIOR_GHIDRA_REPORT.md`.
- Inactive AI Team default is Team D `3` when allies are allowed and `-2` when
  not. Source:
  `SKIRMISH_MPMODES_MUSTALLY_ALLIESALLOWED_ROW_BEHAVIOR_GHIDRA_REPORT.md`,
  `0x006ADC20`, `0x006AE6E0`.
- Same-explicit-team Start validation blocks only when the local team is explicit
  and all active AIs share that explicit team. Source:
  `SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md`,
  `0x006ACEE0`.
- Start and Team are distinct launch values: explicit start maps to
  `House+0x16058`; team/alliance adjunct maps to `House+0x1605C`. Source:
  `SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md`,
  `0x00687F10`.
- Equal explicit `House+0x1605C` values mutual-ally before normal play; sentinel
  values `-2` and `-1` are skipped. Source:
  `SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md`,
  `0x005D74A0..0x005D7549`.

## Design

### Components

`skirmish_modes`

- Keep `SkirmishGameMode` as the common mode object model for shell/session use.
- Add a small common override model internally or as methods on
  `SkirmishGameMode`:
  - native defaults: `allies_allowed=true`, `must_ally=false`;
  - tournament booleans may be parsed and stored only if needed for roster gating,
    but must not be confused with offline launch/team behavior;
  - apply `MustAlly && !AlliesAllowed => MustAlly=false`.
- Add an asset-backed constructor such as `skirmish_modes_from_assets(assets:
  &AssetManager) -> Vec<SkirmishGameMode>`.
- Keep `stock_skirmish_modes()` as an asset-less deterministic fallback. It can
  continue using the verified stock defaults if asset bytes are unavailable.

`skirmish_launch`

- Replace the Battle-only truth with a selected-mode launch representation. The
  preferred shape is a data struct stored on `SkirmishLaunchSession`, for example:
  `SkirmishLaunchMode { id, ui_name_key, override_file, map_filter,
  random_maps_allowed, allies_allowed, must_ally }`.
- This avoids hardcoding a Rust enum variant for every current and modded row
  while preserving the native row id and common fields.
- If an enum remains, it must not be the only launch truth; the numeric id must
  still be present.

`ui::skirmish_shell::state`

- Change `launch_session` to accept `modes: &[SkirmishGameMode]`.
- Resolve `state.selected_mode_id` through `mode_by_id`; if absent, fall back to
  Battle id `1` only as a defensive stock fallback and log/return a validation
  error if the codebase already has a suitable recoverable error style.
- Build the launch mode field from the selected `SkirmishGameMode`.
- Keep ordinary validation order unchanged: selected map, capacity, no opponent,
  same explicit team, then pack.
- Add a future-facing selected-mode acceptance surface only as a neutral result
  type if needed by tests; do not implement speculative custom rejection behavior
  in this slice.

Team UI helpers:

- Add a selected-mode lookup helper for `SkirmishShellState`, returning the
  current mode or stock Battle fallback.
- Update Team `combo_items` to include `-2` only when `!mode.must_ally`, always
  followed by `0..3`.
- Add repair logic used after mode changes and row-state changes:
  - if `mode.must_ally` and a row's team is `-2`, set local player to Team A
    (`0`) and AI rows to Team D (`3`) unless a more precise existing selection
    should be preserved;
  - for inactive AI rows, default to `3` when `mode.allies_allowed`, otherwise
    `-2`;
  - do not introduce `-1` as a Team combo value.

`app`

- Initialize `skirmish_modes` with asset-backed mode loading when
  `startup_asset_manager` exists, otherwise fall back to `stock_skirmish_modes()`.
- Pass `&state.skirmish_modes` into `launch_session`.
- Keep `pending_skirmish_launch_session` as the handoff boundary.

`app_skirmish`

- Preserve current behavior that builds alliances from `LaunchTeam::Team(_)`
  equality and ignores `LaunchTeam::None`.
- Add focused regression tests rather than changing application behavior unless a
  test reveals a mismatch.

### Interfaces / Contracts

- `skirmish_modes_from_assets(&AssetManager) -> Vec<SkirmishGameMode>`:
  loads `MPModesMD.ini` from assets if present; falls back to bundled
  `ini/mpmodesmd.ini` if not. For each row, tries to load `override_file` from
  assets and applies the four verified common keys.
- `parse_mpmodes_ini` remains useful for tests and fallback, but should have a
  path that can receive an override resolver rather than always applying hardcoded
  filename defaults.
- `launch_session(state, maps, modes)` returns a `SkirmishLaunchSession` whose
  selected mode field carries at least the numeric id and common fields.
- `combo_items` either receives `modes` or uses a state-owned selected-mode policy.
  If changing the signature is too invasive, store the current selected-mode
  policy in `SkirmishShellState` during app-level mode commits. Prefer passing
  modes where existing render/app functions already have them; avoid hidden global
  state.

### Data Flow

Startup:

1. `AssetManager::new` loads archive stack.
2. App builds `skirmish_modes` through asset-backed `MPModesMD.ini` plus row
   `MP*.ini` overrides.
3. If assets are absent, app falls back to `stock_skirmish_modes()`.

Shell:

1. Choose Map commits `selected_mode_id`.
2. Mode commit runs Team repair/defaulting using selected `must_ally` and
   `allies_allowed`.
3. Team combo rendering uses selected mode policy for `None,A-D` vs `A-D`.
4. Start calls `launch_session(state, maps, modes)`.
5. `SkirmishLaunchSession` preserves selected mode data into pending launch.

Map init:

1. `apply_skirmish_launch_session` uses the session slots exactly as today for
   houses, colors, start assignment, and explicit-team alliances.
2. Future mode-specific hooks can branch from the preserved selected mode id
   without changing the shell contract again.

### Error Handling

- Missing override `MP*.ini`: use native defaults for the common fields and log at
  debug/warn level. Native constructor has defaults before reading the payload.
- Malformed override INI: ignore malformed payload and keep defaults, matching the
  current parser style of tolerating malformed lines.
- Missing selected mode id in `launch_session`: prefer a validation error if it
  can be surfaced cleanly; otherwise fall back to Battle id `1` with a warning.
  Tests should cover the normal stock path, not the defensive fallback.

### Testing Strategy

`src/skirmish_modes.rs`

- `asset_backed_mpmodes_reads_common_override_fields`
- `mpmode_override_defaults_match_native_when_file_missing`
- `mpmode_override_clears_must_ally_when_allies_disabled`
- `mpmode_override_ignores_ally_change_allowed_for_common_mode`
- Existing stock tests remain: ids `1..9`, no stock Siege, Team Game must-ally,
  FFA allies disabled.

`src/ui/skirmish_shell/state.rs`

- `launch_session_preserves_selected_team_game_mode_id`
- `launch_session_preserves_selected_ffa_mode_id`
- `launch_session_does_not_synthesize_stock_siege`
- `team_game_must_ally_omits_team_none_combo_item`
- `battle_team_combo_keeps_none_and_explicit_teams`
- `ffa_combo_keeps_explicit_teams_despite_allies_disabled`
- `inactive_ai_team_default_follows_allies_allowed`
- Existing same-explicit-team validation stays and should still pass.

`src/app_skirmish.rs`

- `skirmish_launch_same_explicit_team_creates_mutual_alliance`
- `skirmish_launch_team_sentinels_do_not_auto_ally`
- `skirmish_launch_start_position_and_team_are_independent`

Focused command after implementation:

- `cargo test skirmish_modes --lib`
- `cargo test launch_session --lib`
- `cargo test launch_alliance --lib`

Current known blocker: full `cargo test --lib` may still be blocked by unrelated
dirty unit-rendering compile failures in `src/app_instances/units.rs` and
`src/app_render/build_instances.rs`. Do not fix or revert those as part of this
slice unless explicitly asked.

## Architectural Decisions

- Keep MPMode parsing in `skirmish_modes`, not in UI or app. The app supplies
  assets; the module owns the data interpretation.
- Prefer a data-carrying launch mode over a closed Rust enum. Native and modded
  rows are data-driven, and the implementation must preserve ids without needing
  one enum variant per row.
- Do not implement unverified non-Battle post-launch callbacks in this slice. The
  session should preserve enough data for those future hooks, but behavior must
  wait for mode-specific research.
- Keep rules/dialog settings separate from common MPMode fields. `AllyChangeAllowed`
  and money/unit/speed belong to RulesClass/dialog defaults, not this mode object.
- Keep start assignment and team/alliance independent. This follows the current
  Rust structure and the verified native split between `House+0x16058` and
  `House+0x1605C`.

## Alternatives Considered

### Session-Only Patch

Carry selected mode id into `SkirmishLaunchSession` and stop there. This has a
smaller write surface, but it leaves visible Team Game and FFA/Coop row behavior
wrong and keeps the stock filename override switch.

### Full Mode Callback Framework

Introduce Rust equivalents for selected-mode `+0x14`, `+0x80`, `+0x84`, and
`+0x88` callbacks now. This is premature: the first-stage launch/session contract
is verified, but not every non-Battle post-launch callback body is implementation
ready. Building the framework now would create places for guesses.

### Keep Hardcoded Stock Overrides

Leave `apply_known_stock_dialog_defaults` and only patch UI/session consumers.
This matches stock rows today, but contradicts the verified native path where each
row's override filename is loaded through archives. It would keep modded/custom
MPModes broken and make later parser work more invasive.

## Follow-Up Work

- Mode-specific post-launch callbacks for Free For All, Cooperative, Unholy,
  ManBattle variants, and any custom/Siege exposure require separate
  implementation contracts before behavior is added.
- Generic selected-mode rejection modal for custom `false + 0x617` can be added
  after a small selected-mode result abstraction exists; stock local modes should
  not invent that modal.
- Create Random Map remains separate: `RandMap.Sed`, `RandMap.img`, seed/options,
  and `.SED` launch branch are not part of this design.
