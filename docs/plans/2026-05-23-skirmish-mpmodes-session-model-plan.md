# Skirmish MPModes Session Model Implementation Plan

> For Codex: execute only after user approval. This plan is implementation
> planning only; do not write Rust code until approved.

**Goal:** Preserve the selected offline Skirmish MPMode through launch/session
handoff, drive Team row behavior from selected-mode policy, and replace the
stock filename switch for common MPMode override fields with asset/MIX-backed
`MP*.ini` parsing.

**Architecture:** `skirmish_modes` owns data loading and common mode policy,
`ui::skirmish_shell` owns shell state and launch packing, `skirmish_launch` owns
plain app-level session data, and `app_skirmish` consumes normalized launch slots.
`sim/` must not depend on UI, render, sidebar, audio, or net.

**Design Doc:** [docs/plans/2026-05-23-skirmish-mpmodes-session-model-design.md](2026-05-23-skirmish-mpmodes-session-model-design.md)

---

## Grounding Summary

- Native Choose Map commits the selected MPMode object pointer/id and selected map
  together; Start later uses that selected object. Source:
  `SKIRMISH_SELECTED_MPMODE_START_LAUNCH_SESSION_GHIDRA_REPORT.md`,
  `0x005E734F..0x005E7388`, `0x006AD2BA..0x006AD34B`.
- Native Start mirrors selected mode id from `DAT_00A8B250` into launch state
  `DAT_00A8B3C4`; current Rust hardcodes Battle. Source:
  `SKIRMISH_SELECTED_MPMODE_START_LAUNCH_SESSION_GHIDRA_REPORT.md`,
  `0x006AD34B..0x006AD36B`.
- Stock local mode ids are `1..9`; binary Siege support exists but no stock
  offline `[Siege]` row exists in `ini/mpmodesmd.ini`.
- Common MPMode override construction reads exactly four
  `[MultiplayerDialogSettings]` keys from each row's `MP*.ini` file:
  `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`,
  and `MustAlly`. Source:
  `SKIRMISH_MPMODES_MIX_OVERRIDE_PAYLOADS_GHIDRA_REPORT.md`,
  `0x005D5CA7..0x005D5D11`.
- Native defaults before override are tournament flags true,
  `AlliesAllowed=true`, and `MustAlly=false`; if `MustAlly && !AlliesAllowed`,
  native clears `MustAlly`. Source:
  `SKIRMISH_MPMODES_MIX_OVERRIDE_PAYLOADS_GHIDRA_REPORT.md`,
  `0x005D5BEA..0x005D5D11`.
- `MustAlly` controls Team `None`; Team Game omits `None(-2)` and exposes only
  `A-D`. `AlliesAllowed=false` does not remove `A-D`; FFA/Coop still expose Team
  rows. Source:
  `SKIRMISH_MPMODES_MUSTALLY_ALLIESALLOWED_ROW_BEHAVIOR_GHIDRA_REPORT.md`.
- Inactive AI Team default follows `AlliesAllowed`: false -> `-2`, true -> Team
  D `3`. Source:
  `SKIRMISH_MPMODES_MUSTALLY_ALLIESALLOWED_ROW_BEHAVIOR_GHIDRA_REPORT.md`,
  `0x006ADC20`, `0x006AE6E0`.
- Start and Team are separate values. Explicit start maps to `House+0x16058`;
  Team/alliance maps to `House+0x1605C`. Equal non-sentinel teams mutual-ally;
  sentinels `-2` and `-1` do not. Source:
  `SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md`.
- Selected-mode callback false return is not enough to reject Start; native blocks
  only on `false && output_dword == 0x617`. Source:
  `SKIRMISH_SELECTED_MODE_START_VALIDATION_CONTRACT_GHIDRA_REPORT.md`.

## Key Technical Decisions

- **Use a data-carrying launch mode, not a closed Battle-only enum.** Store at
  least the selected mode id and common fields in `SkirmishLaunchSession`.
  Confidence: high. Source: selected-mode launch report and current Rust scan.
- **Keep `stock_skirmish_modes()` as an asset-less fallback.** Tests and startup
  without a configured retail install still need deterministic stock data.
  Confidence: high. Source: current app startup shape.
- **Load common override fields through `AssetManager` when available.** Follow
  existing rules/art loader patterns and `IniFile::from_bytes`; do not introduce
  a separate asset lookup mechanism. Confidence: high. Source: asset manager and
  payload report.
- **Keep common MPMode fields separate from RulesClass dialog settings.** Do not
  fold `AllyChangeAllowed`, money, unit count, tech, speed, checkboxes,
  `FogOfWar`, or `MCVRedeploys` into `SkirmishGameMode`. Confidence: high.
  Source: payload report.
- **Pass mode data explicitly to shell helpers where practical.** Prefer
  `launch_session(state, maps, modes)` and mode-aware Team helper functions over
  hidden globals. Confidence: medium-high. Source: current shell render functions
  already receive `modes` in several places, but `combo_items` has many callers.
- **Do not implement unverified non-Battle post-launch callbacks in this patch.**
  Preserve selected mode identity and leave behavior hooks possible. Confidence:
  high. Source: design doc and deferred mode-callback research.

## Open Questions

### Resolved During Planning

- Is this multiplayer netcode? No. The verified path is offline Skirmish shell
  setup/session handoff.
- Should stock Siege be added? No. The binary registers Siege support, but stock
  local `ini/mpmodesmd.ini` has no offline Siege row.
- Does `AlliesAllowed=false` remove Team choices? No. It changes defaults; Team
  `A-D` remain visible.
- Is every selected-mode false return a modal? No. Native blocks only on
  `false && output_dword == 0x617`.

### Deferred Research / Follow-Up

- Exact non-Battle `+0x80/+0x84` post-launch callbacks.
- Generic custom/modded selected-mode rejection output handling beyond preserving
  the future surface.
- Create Random Map `RandMap.Sed` / `RandMap.img` / `.SED` launch path.
- Exact Unholy disabled-byte setup path.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | [src/skirmish_modes.rs](../../src/skirmish_modes.rs) | Add asset-backed MPMode roster/override loader, native common defaults, and tests. |
| Modify | [src/skirmish_launch.rs](../../src/skirmish_launch.rs) | Replace Battle-only mode truth with selected-mode launch data. |
| Modify | [src/ui/skirmish_shell/state.rs](../../src/ui/skirmish_shell/state.rs) | Use selected mode in launch packing; make Team rows/default repair mode-aware; update tests. |
| Modify | [src/ui/skirmish_shell/mod.rs](../../src/ui/skirmish_shell/mod.rs) | Re-export any changed shell helper signatures if needed. |
| Modify | [src/app.rs](../../src/app.rs) | Initialize asset-backed `skirmish_modes`; pass modes into `launch_session`; repair teams after Choose Map mode commit. |
| Modify | [src/app_skirmish.rs](../../src/app_skirmish.rs) | Add focused alliance/start-team independence tests; update session fixtures. |

## Interface Changes

### `src/skirmish_modes.rs`

Add an asset-backed load function:

```rust
pub fn skirmish_modes_from_assets(assets: &AssetManager) -> Vec<SkirmishGameMode>;
```

Expected behavior:

- Load `MPModesMD.ini` from assets if present; otherwise use bundled
  `ini/mpmodesmd.ini`.
- Parse stock categories in the current deterministic order.
- For every parsed row, initialize native common defaults.
- Try to load `mode.override_file` through `AssetManager`.
- If present and parseable, apply only the four verified common override keys.
- If override is missing or malformed, retain defaults.
- Apply native clamp: if `!allies_allowed`, force `must_ally=false`.

Add a small internal helper:

```rust
fn apply_common_override(mode: &mut SkirmishGameMode, ini: &IniFile);
```

Keep:

```rust
pub fn stock_skirmish_modes() -> Vec<SkirmishGameMode>;
pub fn parse_mpmodes_ini(ini: &IniFile) -> Vec<SkirmishGameMode>;
```

But remove direct dependence on `apply_known_stock_dialog_defaults` for the
asset-backed path. If the function remains for fallback stock tests, mark it as
fallback-only and keep it narrow.

### `src/skirmish_launch.rs`

Change the Battle-only mode type to carry selected mode data:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishLaunchMode {
    pub id: i32,
    pub ui_name_key: String,
    pub tooltip_key: String,
    pub override_file: String,
    pub map_filter: String,
    pub random_maps_allowed: bool,
    pub allies_allowed: bool,
    pub must_ally: bool,
}
```

Then:

```rust
impl From<&SkirmishGameMode> for SkirmishLaunchMode { ... }
```

If direct dependency from `skirmish_launch` to `skirmish_modes` creates an
unwanted module edge, use a constructor method in `skirmish_launch` that receives
plain fields, or a helper in `ui::skirmish_shell::state`. Do not put UI types in
`skirmish_launch`.

### `src/ui/skirmish_shell/state.rs`

Change:

```rust
pub fn launch_session(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
) -> Result<SkirmishLaunchSession, LaunchValidationError>;
```

to:

```rust
pub fn launch_session(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    modes: &[SkirmishGameMode],
) -> Result<SkirmishLaunchSession, LaunchValidationError>;
```

Resolve selected mode by `state.selected_mode_id`. The normal path must use the
selected id. Defensive fallback may use Battle id `1`, but tests should assert
stock selected ids resolve without fallback.

Make Team helpers mode-aware. Preferred shape:

```rust
pub fn combo_items(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    modes: &[SkirmishGameMode],
    id: SkirmishComboId,
) -> Vec<SkirmishComboItem>;
```

If this signature is too broad in one patch, add a focused Team helper and keep
existing call sites delegating through it. The final behavior still must use
selected mode policy.

Add a repair/default helper:

```rust
pub fn repair_teams_for_selected_mode(
    state: &mut SkirmishShellState,
    modes: &[SkirmishGameMode],
);
```

Expected behavior:

- If selected mode `must_ally` and `player_team == -2`, set local to Team A `0`.
- If selected mode `must_ally` and any opponent team is `-2`, set it to Team D
  `3`.
- For inactive AI rows, set team to Team D `3` when `allies_allowed=true`, else
  `-2`.
- Preserve explicit `0..3` selections that remain valid.
- Never create a Team `-1` row.

### `src/app.rs`

Startup:

```rust
let skirmish_modes = startup_asset_manager
    .as_ref()
    .map(crate::skirmish_modes::skirmish_modes_from_assets)
    .unwrap_or_else(crate::skirmish_modes::stock_skirmish_modes);
```

Start action passes modes into launch packing:

```rust
launch_session(
    &state.skirmish_shell_state,
    &state.skirmish_shell_maps,
    &state.skirmish_modes,
)
```

Choose Map commit:

- After assigning `state.skirmish_shell_state.selected_mode_id`, call
  `repair_teams_for_selected_mode`.
- Keep preview invalidation and selected-map commit behavior unchanged.

## Sim Checklist

- [x] No `sim/` dependency on `ui/`, `render/`, `sidebar/`, `audio/`, or `net/`.
- [x] New selected-mode data lives in app/session data, not simulation tick state.
- [x] No floating-point math added to simulation logic.
- [x] Existing `LaunchTeam::Team(_)` equality remains deterministic.
- [x] `EntityStore` / `BTreeMap` iteration is untouched.
- [x] No ECS or dependency changes.
- [x] If future mode-specific hooks add sim-visible state, they require a separate
  hash/determinism review.

## Risk Areas

- **Compile blocker already present:** unrelated dirty unit-rendering changes in
  `src/app_instances/units.rs` and `src/app_render/build_instances.rs` may block
  broad `cargo test --lib`. Use focused tests first and do not fix unrelated
  files unless asked.
- **Combo helper signature churn:** `combo_items` is used by rendering, hit tests,
  dropdown selection, and tests. Update all call sites consistently.
- **Team repair timing:** Repair must run after selected mode changes and before
  launch packing; otherwise Team Game can still pack impossible `-2`.
- **Fallback masking:** A Battle fallback for missing selected mode can hide a real
  roster bug. Tests must assert selected Team Game/FFA ids are preserved.
- **Asset lookup availability:** Startup may not have an asset manager in some
  test/dev paths. Keep fallback deterministic.
- **Over-modeling:** Do not add unresearched selected-mode gameplay behavior just
  because the selected mode id is now present.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | Common override fields loaded from `MP*.ini` | Team Game/FFA/Coop policy must come from data, not filename branches | `skirmish_modes` tests |
| 2 | Selected mode id/data in session | Team Game/FFA starts must not silently become Battle | `launch_session_preserves_selected_*` tests |
| 3 | Team `None` gated by `MustAlly` | Team Game visible row mismatch is player-visible in normal setup | Team combo tests |
| 4 | Inactive AI default follows `AlliesAllowed` | FFA/Coop vs Battle/Team Game rows differ before launch | row default tests |
| 5 | Start/team separation preserved | Prevents start-position and alliance drift | `app_skirmish` tests |
| 6 | Negative team sentinels do not ally | Prevents two `None` rows becoming allies | alliance tests |

---

## Tasks

### Task 1: Add common MPMode override parsing

**Why:** Native reads each row's override file through the archive/file path.
This removes the stock filename branch as the authoritative path.

**Files:**

- Modify: [src/skirmish_modes.rs](../../src/skirmish_modes.rs)

**Steps:**

1. Import `AssetManager` in `skirmish_modes.rs`.
2. Add a native-default initialization helper for common fields:
   `allies_allowed=true`, `must_ally=false`.
3. Add `apply_common_override(mode, ini)`:
   - read section `MultiplayerDialogSettings`;
   - apply only `AlliesAllowed` and `MustAlly` to existing fields for now;
   - optionally parse tournament booleans only if stored in `SkirmishGameMode`;
   - clear `must_ally` if `!allies_allowed`.
4. Add `skirmish_modes_from_assets(assets)`:
   - load `MPModesMD.ini` from `assets.get_with_source`;
   - fallback to bundled stock if absent or malformed;
   - parse roster;
   - for each mode, load `mode.override_file` and apply override if parseable;
   - keep defaults if override missing.
5. Keep `stock_skirmish_modes()` stable for asset-less tests. If needed, keep the
   verified stock fallback switch only for this function.

**Tests to add/update:**

- `mpmode_override_defaults_match_native_when_file_missing`
- `mpmode_override_clears_must_ally_when_allies_disabled`
- `mpmode_override_ignores_ally_change_allowed_for_common_mode`
- Existing stock tests for ids `1..9`, Team Game, FFA, no Siege.

**Verification command:**

```powershell
cargo test skirmish_modes --lib
```

### Task 2: Replace Battle-only launch mode with selected-mode data

**Why:** Native copies selected mode id into launch state. Current Rust discards
that id and always returns Battle.

**Files:**

- Modify: [src/skirmish_launch.rs](../../src/skirmish_launch.rs)
- Modify: [src/app_skirmish.rs](../../src/app_skirmish.rs) test fixtures

**Steps:**

1. Change `SkirmishLaunchMode` from enum `{ Battle }` to a data struct carrying
   selected mode fields.
2. Add constructor/conversion from `SkirmishGameMode` or plain fields.
3. Update `SkirmishLaunchSession` users and test fixtures to construct the new
   mode data.
4. Keep `LaunchTeam::from_shell_value` behavior unchanged for now, but add/keep
   tests proving `-2` and `-1` map to non-explicit team state if the current
   abstraction remains collapsed.

**Tests to add/update:**

- Update existing `test_session()` fixture in `app_skirmish.rs`.
- Update existing `launch_session_packs_selected_map_and_enabled_slots` expected
  mode assertion after Task 3.

**Verification command:**

```powershell
cargo test skirmish_launch --lib
```

### Task 3: Pass selected mode roster into `launch_session`

**Why:** Shell launch packing needs the committed selected mode, not just map and
row state.

**Files:**

- Modify: [src/ui/skirmish_shell/state.rs](../../src/ui/skirmish_shell/state.rs)
- Modify: [src/ui/skirmish_shell/mod.rs](../../src/ui/skirmish_shell/mod.rs) if
  exported signatures change
- Modify: [src/app.rs](../../src/app.rs)

**Steps:**

1. Change `launch_session` signature to accept `modes: &[SkirmishGameMode]`.
2. Resolve `state.selected_mode_id` through `mode_by_id`.
3. Build `SkirmishLaunchSession.mode` from the selected mode.
4. Keep ordinary validation order unchanged:
   selected map -> capacity -> no enabled opponent -> same explicit team ->
   packing.
5. Update `App::handle_skirmish_shell_action` to pass `&state.skirmish_modes`.
6. Update all `launch_session` tests to pass stock modes.

**Tests to add/update:**

- `launch_session_preserves_selected_team_game_mode_id`
- `launch_session_preserves_selected_ffa_mode_id`
- `launch_session_does_not_synthesize_stock_siege`
- Update `launch_session_packs_selected_map_and_enabled_slots` to assert selected
  mode id instead of Battle enum.

**Verification command:**

```powershell
cargo test launch_session --lib
```

### Task 4: Initialize app `skirmish_modes` from assets

**Why:** Runtime should use archive-backed `MPModesMD.ini` and `MP*.ini` payloads
when assets exist.

**Files:**

- Modify: [src/app.rs](../../src/app.rs)

**Steps:**

1. In `AppState::new`, build `skirmish_modes` after `startup_asset_manager` is
   created.
2. Use `skirmish_modes_from_assets` when asset manager exists.
3. Fall back to `stock_skirmish_modes` when no asset manager exists.
4. Preserve existing scenario-record and map-list initialization.

**Tests:**

- Unit coverage is mostly in `skirmish_modes.rs`; no app integration test is
  required unless a suitable app-state constructor test already exists.

**Verification command:**

```powershell
cargo test skirmish_modes --lib
```

### Task 5: Make Team combo rows mode-aware

**Why:** Team Game must not show `None`; FFA/Coop must still show `A-D`.

**Files:**

- Modify: [src/ui/skirmish_shell/state.rs](../../src/ui/skirmish_shell/state.rs)
- Modify: [src/app_skirmish_shell_render.rs](../../src/app_skirmish_shell_render.rs)
  if `combo_items` signature changes
- Modify: [src/ui/skirmish_shell/mod.rs](../../src/ui/skirmish_shell/mod.rs) if
  exports change

**Steps:**

1. Make Team combo item construction resolve the selected mode.
2. Return `[-2,0,1,2,3]` when `!must_ally`.
3. Return `[0,1,2,3]` when `must_ally`.
4. Keep Side, Color, Start, and AiType combo behavior unchanged.
5. Update all call sites of `combo_items` if modes are added to the signature.

**Tests to add/update:**

- `battle_team_combo_keeps_none_and_explicit_teams`
- `team_game_must_ally_omits_team_none_combo_item`
- `ffa_combo_keeps_explicit_teams_despite_allies_disabled`
- Update existing `team_combo_uses_verified_item_data_values` to name Battle
  specifically.

**Verification command:**

```powershell
cargo test combo --lib
```

### Task 6: Repair/default Team values on selected-mode changes and row-state changes

**Why:** Hiding `None` is not enough; stale `-2` values must not be packed for
Team Game, and inactive AI defaults differ by `AlliesAllowed`.

**Files:**

- Modify: [src/ui/skirmish_shell/state.rs](../../src/ui/skirmish_shell/state.rs)
- Modify: [src/app.rs](../../src/app.rs)

**Steps:**

1. Add `repair_teams_for_selected_mode(state, modes)`.
2. Call it after Choose Map commits `selected_mode_id`.
3. Call it after AI row-state changes if the changed row becomes inactive or if
   existing code already centralizes row-state changes.
4. For Team Game / `must_ally=true`:
   - local `-2` -> `0`;
   - opponent `-2` -> `3`.
5. For inactive AI rows:
   - `allies_allowed=true` -> `3`;
   - `allies_allowed=false` -> `-2`.
6. Preserve existing explicit `0..3` values when valid.

**Tests to add/update:**

- `team_game_mode_repair_removes_team_none_from_local_and_ai`
- `inactive_ai_team_default_follows_allies_allowed`
- `explicit_team_values_survive_mode_repair`

**Verification command:**

```powershell
cargo test team --lib
```

### Task 7: Add launch alliance/start-team regression tests

**Why:** Current Rust is believed to mostly match the native explicit-team
alliance handoff, but the behavior is not locked.

**Files:**

- Modify: [src/app_skirmish.rs](../../src/app_skirmish.rs)

**Steps:**

1. Add a test where local and one AI share explicit Team A and another AI is Team
   B; assert Team A pair is mutually allied and Team B is not.
2. Add a test where local and AI both use sentinel/none; assert they do not become
   allies because defaults/sentinels are not explicit teams.
3. Add a test where local has explicit Start 2 and Team A; assert start assignment
   is based on Start while alliance is based on Team.
4. Update existing test fixtures for the new `SkirmishLaunchMode` data struct.

**Proposed test names:**

- `skirmish_launch_same_explicit_team_creates_mutual_alliance`
- `skirmish_launch_team_sentinels_do_not_auto_ally`
- `skirmish_launch_start_position_and_team_are_independent`

**Verification command:**

```powershell
cargo test launch_alliance --lib
```

### Task 8: Focused formatting and test pass

**Why:** Keep verification scoped to the changed surfaces and avoid unrelated
dirty-worktree compile blockers where possible.

**Commands:**

```powershell
cargo test skirmish_modes --lib
cargo test launch_session --lib
cargo test combo --lib
cargo test team --lib
cargo test launch_alliance --lib
cargo fmt
```

If the unrelated unit-rendering compile blocker still prevents even focused
tests from compiling:

1. Capture the first unrelated compiler errors.
2. Confirm the errors are still in the known dirty files:
   `src/app_instances/units.rs` and `src/app_render/build_instances.rs`.
3. Do not fix or revert them unless the user explicitly asks.
4. Still run `cargo fmt --check` or `cargo fmt` for edited files if possible.

### Task 9: Manual parity check in native shell

**Why:** The most visible changes are shell/UI row behavior and launch identity.

**Scenario:**

1. Start native Skirmish shell with `RA2_DEV_SKIRMISH_SHELL=1`.
2. Open Choose Map.
3. Select Battle: Team combo shows `None,A,B,C,D`.
4. Select Team Game: Team combo shows `A,B,C,D`; no `None`.
5. Select FFA: Team combo shows `None,A,B,C,D`; inactive AI defaults to `None`.
6. Select Team Game and Start: session/log/test instrumentation should show mode
   id `9`, not Battle id `1`.
7. Select FFA and Start: session/log/test instrumentation should show mode id `2`.

Do not claim full non-Battle gameplay parity from this manual check; it verifies
the shell/session handoff and visible Team row behavior only.

## Stop Conditions

- A selected stock mode id cannot be resolved from `state.selected_mode_id` and
  `skirmish_modes`.
- Team Game still packs `-2` after mode repair.
- Asset-backed parsing changes stock ids or synthesizes Siege.
- Any patch introduces `sim/` dependency on UI/app/render modules.
- Focused tests fail for behavior not explained by the known unrelated compile
  blocker.

