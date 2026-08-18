# SkirmishLaunchSession Implementation Plan

> For Codex: execute only after user approval. This plan is intentionally implementation-only planning; do not write Rust code until approved.

**Goal:** Add a native-shaped `SkirmishLaunchSession` app-level contract and migrate standard skirmish startup toward session-driven Battle-mode consumers.

**Architecture:** The launch session lives above `sim/` as deterministic plain data. UI shell code produces it, app loading carries it, and app init translates it into `Simulation.game_options`, runtime houses, start assignments, AI registration, and startup MCV generation. `sim/` receives normalized state only and must not depend on `ui/`, `render/`, `sidebar/`, `audio/`, or `net/`.

**Design Doc:** `docs/plans/2026-05-22-skirmish-launch-session-design.md`

---

## Grounding Summary

- Verified research says Start Game packs launch/session state and exits the shell; spawn work begins in `ScenarioClass::Full_Init` and `Post_Map_Init`.
- Live Ghidra refresh confirmed `ScenarioClass__Full_Init @ 0x00686B20` clears `Scenario+0x1180`, calls `ScenarioClass__Create_Houses`, mode `+0x80`, then either `AssignStartingPoints` or mode `+0x84`.
- Live Ghidra refresh confirmed `ScenarioClass__Create_Houses @ 0x00687F10` builds human and AI houses from packed session/node/AI arrays, not from the map `[Houses]` roster order.
- Live Ghidra refresh confirmed `ScenarioClass__AssignStartingPoints @ 0x005EE9D0` assigns human-controlled houses first, then AI houses.
- Live Ghidra refresh confirmed `ScenarioClass__Gather_Start_Positions @ 0x00688380` uses waypoints `0..7` and generates deterministic-engine random fallback positions when starts are deficient.
- Live Ghidra refresh confirmed `ScenarioClass__Post_Map_Init @ 0x00686890` calls selected mode `+0x84` and then `FUN_005D6D80 @ 0x005D6D80` when a mode object exists.
- Live Ghidra refresh confirmed `FUN_005D6D80 @ 0x005D6D80` uses `DAT_00A8B270` as starting-unit budget input and loops non-special houses; exact Battle `+0x84` internals remain a research gate.
- Live Ghidra refresh confirmed `FUN_004A8EB0 @ 0x004A8EB0` reads `BuildOffAlly` for tactical building placement providers, not for startup house creation or spawn placement.
- Current Rust still routes shell start through `SkirmishSettings`, `GameScreen::Loading { map_name }`, `app_init::load_map(..., &SkirmishSettings)`, and `seed_skirmish_opening_if_needed`.
- Current Rust creates `HouseState` from map roster, then the skirmish helper reorders map houses by side and spawns MCVs through `pairings.take(2)`.
- `GameOptions` is already included in `Simulation::state_hash`; house fields that affect startup identity must either be included in the hash or deliberately stored only in app-level launch state before normalization.
- `ini/rulesmd.ini [MultiplayerDialogSettings]` has `Money=10000`, `UnitCount=10`, `TechLevel=10`, `GameSpeed=1`, `AIDifficulty=0`, `AIPlayers=0`, `BridgeDestruction=yes`, `Shroud=yes`, `Bases=yes`, `TiberiumGrows=yes`, `Crates=yes`, `HarvesterTruce=no`, `MultiEngineer=no`, `AlliesAllowed=no`, `ShortGame=yes`, `FogOfWar=no`, `MCVRedeploys=yes`, and `AllyChangeAllowed=yes`.
- `BuildOffAlly` is absent from supplied `rulesmd.ini`; the binary constructor default is enabled for standard YR skirmish.

## Key Technical Decisions

- Create `src/skirmish_launch.rs` as the app-level contract module, and keep launch-owned country/player-row enums there instead of reusing legacy UI menu types. Confidence: high. Source: design doc and current app routing shape.
- Keep `SkirmishSettings` as a legacy egui menu compatibility type during the migration. Confidence: high. Source: current `src/ui/main_menu.rs` and design doc.
- Store `Option<SkirmishLaunchSession>` in `GameScreen::Loading` and pass it to `app_init::load_map`. Confidence: high. Source: current `GameScreen::Loading { map_name }` is the only loading carrier.
- Build runtime skirmish houses from launch slots for the native shell path, while preserving map roster data for neutral/special/map-authored content. Confidence: high. Source: `ScenarioClass__Create_Houses` and start-to-init trace.
- Treat selected-map token loading as a field in the contract but continue resolving through `selected_map_idx -> MapMenuEntry.file_name` for the first slice. Confidence: medium. Source: research reports mark exact token loader unresolved.
- Implement first-slice Battle startup as "one base unit/MCV placement attempt per active non-special player house" and do not claim full `UnitCount` parity until Battle `+0x84` is decoded. Confidence: medium. Source: `Post_Map_Init` and `FUN_005D6D80` are verified; Battle callback body remains unresolved.
- Put `build_off_ally` in launch options and `GameOptions` now; defer the tactical placement consumer patch to a separate slice. Confidence: high. Source: BuildOffAlly first-consumer report.

## Open Questions

### Resolved During Planning

- Does standard startup create houses from map roster order? No. Verified `ScenarioClass__Create_Houses` consumes packed session/node/AI arrays.
- Does `BuildOffAlly` affect startup? No verified startup reader; the first gameplay consumer is tactical placement helper `FUN_004A8EB0`.
- Is the current two-player shortcut acceptable as the final native shell path? No. Research and design both require all active player rows to be consumed.

### Deferred RE Gates

- Exact selected-map filename/token loader between shell modal success and scenario file load. Required before declaring selected-map parity.
- Exact standard Battle mode `+0x84` callback body at or around `0x005D6C70`. Required before declaring full starting-unit, budget, MCV placement, and deploy-queue parity.
- Native validation details for duplicate colors, duplicate explicit starts, same-team restrictions, random country/color resolution, and capacity acceptance. First slice should validate only facts represented by current Rust shell state and mark deeper validation as incomplete in tests/docs.
- Deficient waypoint fallback exact RNG integration. First slice may expose a typed unsupported/fallback result, but full parity requires deterministic RNG-backed passable start generation with the 8x8 clearance rule.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/skirmish_launch.rs` | App-level plain data contract, validation errors, legacy conversion helpers, option mapping. |
| Modify | `src/lib.rs` or `src/main.rs` module declarations | Export `skirmish_launch` for app/UI/app init use. |
| Modify | `src/ui/skirmish_shell/state.rs` | Produce `SkirmishLaunchSession` from full shell row state instead of collapsing to `SkirmishSettings`. |
| Modify | `src/ui/main_menu.rs` | Keep legacy settings and add conversion to a minimal `SkirmishLaunchSession` for egui fallback starts. |
| Modify | `src/ui/game_screen.rs` | Carry `Option<SkirmishLaunchSession>` through loading. |
| Modify | `src/app.rs` | Store shell-produced launch sessions and start loading with session data. |
| Modify | `src/app_transitions.rs` | Pass the optional session into `app_init::load_map`. |
| Modify | `src/app_init.rs` | Apply launch options, create runtime skirmish houses from session slots, register AI from session slots, and call session-driven startup helper. |
| Modify | `src/app_skirmish.rs` | Add session-driven house/start/startup helpers; keep old helper only for compatibility paths until removed. |
| Modify | `src/app_spawn_pick.rs` | Keep the existing spawn-pick shortcut explicitly legacy-only, or adapt any changed seeder signature so this caller does not silently break. |
| Modify | `src/sim/game_options.rs` | Fix standard YR `build_off_ally` default to true and add mapping tests. |
| Modify | `src/sim/house_state.rs` | Add deterministic launch metadata fields only if needed for color/start/team/difficulty after app-level normalization. |
| Modify | `src/sim/world/world_hash.rs` | Hash any new `HouseState` fields that affect player-visible deterministic state. |

## Interface Changes

Add `src/skirmish_launch.rs`:

```rust
//! App-level skirmish launch contract produced by shell/UI code and consumed by map init.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishLaunchSession {
    pub selected_map_idx: usize,
    pub selected_map_file: Option<String>,
    pub mode: SkirmishMode,
    pub local: SkirmishPlayerSlot,
    pub opponents: Vec<SkirmishAiSlot>,
    pub options: SkirmishLaunchOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishMode {
    Battle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchCountry {
    America,
    Korea,
    France,
    Germany,
    GreatBritain,
    Libya,
    Iraq,
    Cuba,
    Russia,
    Yuri,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishPlayerSlot {
    pub country: LaunchCountry,
    pub color_index: u8,
    pub start_position: LaunchStartPosition,
    pub team: LaunchTeam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishAiSlot {
    pub country: LaunchCountry,
    pub color_index: u8,
    pub start_position: LaunchStartPosition,
    pub team: LaunchTeam,
    pub difficulty: AiDifficulty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchStartPosition {
    Auto,
    Waypoint(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchTeam {
    None,
    Team(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiDifficulty {
    Easy,
    Normal,
    Hard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishLaunchOptions {
    pub starting_credits: i32,
    pub unit_count: i32,
    pub game_speed: i32,
    pub short_game: bool,
    pub super_weapons: bool,
    pub build_off_ally: bool,
    pub mcv_redeploy: bool,
    pub crates: bool,
    pub bases: bool,
    pub fog_of_war: bool,
    pub shroud: bool,
    pub bridges_destroyable: bool,
    pub ally_change_allowed: bool,
    pub multi_engineer: bool,
    pub harvester_truce: bool,
    pub tech_level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchValidationError {
    NoSelectedMap,
    NoEnabledOpponent,
    InvalidColorIndex { color_index: u8 },
    InvalidStartWaypoint { waypoint: u8 },
}
```

Change shell handoff:

```rust
pub fn launch_session(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
) -> Result<SkirmishLaunchSession, LaunchValidationError>;
```

Change loading handoff:

```rust
pub enum GameScreen {
    Loading {
        map_name: String,
        skirmish_launch: Option<SkirmishLaunchSession>,
    },
}

pub fn load_map(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    requested_map: Option<&str>,
    skirmish_launch: Option<&SkirmishLaunchSession>,
    legacy_settings: &SkirmishSettings,
    vxl_compute: Option<&mut VxlComputeRenderer>,
) -> Result<MapLoadResult>;
```

Add session-driven startup helpers in `src/app_skirmish.rs`:

```rust
pub(crate) fn apply_skirmish_launch_session(
    sim: &mut Simulation,
    map_data: &MapFile,
    rules: &RuleSet,
    height_map: &BTreeMap<(u16, u16), u8>,
    session: &SkirmishLaunchSession,
) -> SkirmishLaunchApplyResult;

pub(crate) struct SkirmishLaunchApplyResult {
    pub local_owner: Option<String>,
    pub active_player_count: usize,
    pub spawned_mcv_count: usize,
    pub unsupported_deficient_starts: bool,
}
```

## Sim Checklist

- New session types are outside `sim/`.
- `sim/` imports no `ui/`, `render/`, `sidebar/`, `audio/`, or `net/`.
- `GameOptions` fields are already hashed by `world_hash`.
- Any new `HouseState` fields for color, launch start, team, or AI difficulty must be serialized and hashed if they affect deterministic gameplay or player-visible state.
- Entity ordering remains `BTreeMap<u64, GameEntity>`.
- House ordering must be deterministic: local human first for launch slot construction, then enabled AI rows in shell row order, with interned owner names produced in that same order.
- Session-derived `HouseColorMap` must be built before any atlas rebuild that needs player remap colors.
- No floating point math is introduced in `sim/` logic.
- First slice does not alter `Simulation::advance_tick` ordering.

## Risk Areas

- `GameScreen::Loading` signature changes affect all load transitions.
- `app_init::load_map` is broad and may need a scoped extraction to keep file growth manageable.
- Changing house creation source can affect local owner selection, sidebar ownership, AI registration, alliances, fog initialization, and world hash.
- Current map entity spawning increments owned counts while spawning; session-created houses must exist before owned session entities spawn, or counts must be replayed after house insertion.
- Current atlas and palette setup uses map-roster `HouseColorMap`; session player colors must replace or merge this map before session MCVs and player-owned objects render.
- `build_off_ally` default changes state hashes; this is correct for parity but can update snapshot/hash expectations.
- Keeping the old egui fallback path must not silently preserve the native shell two-player shortcut.
- Existing debug spawn and spawn-pick paths should not fabricate partial native sessions unless explicitly testing skirmish startup.

## Parity-Critical Items

| Stage | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | Full row preservation in launch session | Player-selected AI country, color, team, difficulty, and start must survive Start Game. | Unit test shell state with multiple enabled AI rows. |
| 2 | GameOptions mapping and hashing | Options such as `BuildOffAlly`, `ShortGame`, `UnitCount`, `Bases`, and `MCVRedeploy` change gameplay and replay hash. | Unit test option mapping plus `state_hash` divergence for changed option. |
| 3 | Houses from launch slots | Player count, owner identity, side, and local owner must match shell rows rather than map roster accident. | Unit test four active players on a map roster with different order. |
| 4 | Explicit starts for local and AI | Chosen start positions are visible immediately at match start. | Unit test start assignment table maps slots to requested waypoints. |
| 5 | No two-player cap | Normal skirmish supports more than two active players. | Unit test four active slots get houses and MCV placement attempts. |
| 6 | Deficient starts are explicit | Returning "no spawn" silently is a visible parity bug. | Unit test marks unsupported/fallback condition until deterministic fallback is implemented. |

---

## Staged Migration Order

### Stage 1: Define the App-Level Contract

**Files:**
- Create `src/skirmish_launch.rs`
- Modify module declaration in `src/main.rs` or `src/lib.rs`

**Work:**
1. Define `SkirmishLaunchSession`, slot types, launch option type, launch-owned country/team/start/difficulty enums, and `LaunchValidationError`.
2. Add `impl Default for SkirmishLaunchOptions` using standard YR defaults from `rulesmd.ini` plus verified constructor default `build_off_ally: true`.
3. Add UI-to-launch conversion helpers, for example `LaunchCountry::from_legacy_menu_country(crate::ui::main_menu::SkirmishCountry)`, so the session module owns the launch contract and consumers do not need legacy menu country types.
4. Add `SkirmishLaunchOptions::to_game_options(&self) -> crate::sim::game_options::GameOptions`.

**Focused tests:**
- `launch_options_default_build_off_ally_enabled`
- `launch_options_map_to_game_options_preserves_hashed_fields`
- `launch_start_position_rejects_waypoint_above_seven`

**Verification command:**
- `cargo test skirmish_launch -- --nocapture`

### Stage 2: Pack Shell Rows Into `SkirmishLaunchSession`

**Files:**
- Modify `src/ui/skirmish_shell/state.rs`
- Modify tests in the same file

**Work:**
1. Extend `SkirmishShellState` with a local `player_team: i32` field because the current shell only has opponent team data.
2. Extend `SkirmishShellOpponent` with `difficulty: AiDifficulty` or a shell-local difficulty enum that maps to `crate::skirmish_launch::AiDifficulty`.
3. Change `SkirmishShellState::default()` to allocate seven opponent rows in deterministic row order, with only the first row enabled if the current UI should still visually start as 1v1.
4. Replace native shell Start path usage of `launch_settings` with `launch_session(&state, maps)`.
5. Preserve `selected_map_idx`, `selected_map_file`, local country, local color, local start, local team, all enabled AI rows, AI color, AI start, AI team, and AI difficulty.
6. Convert existing integer team fields into `LaunchTeam::None` for `0` or negative values, and `LaunchTeam::Team(value as u8)` for positive values within valid UI range.
7. Keep `launch_settings` only as a legacy compatibility helper until egui fallback migration is complete.

**Focused tests:**
- `skirmish_shell_packs_all_enabled_rows_into_launch_session`
- `skirmish_shell_ignores_disabled_opponents_in_session_consumers`
- `skirmish_shell_default_has_seven_deterministic_opponent_rows`
- `skirmish_shell_packs_player_team_and_ai_difficulty`
- `skirmish_shell_launch_session_rejects_missing_map`
- `skirmish_shell_launch_session_rejects_no_enabled_opponent`

**Verification command:**
- `cargo test skirmish_shell -- --nocapture`

### Stage 3: Carry Session Through Loading

**Files:**
- Modify `src/ui/game_screen.rs`
- Modify `src/app.rs`
- Modify `src/app_transitions.rs`
- Modify `src/app_init.rs` signature only in this stage

**Work:**
1. Change `GameScreen::Loading` to include `skirmish_launch: Option<SkirmishLaunchSession>`.
2. Split app start helpers into `start_selected_skirmish_legacy` and `start_skirmish_session`.
3. Native shell Start calls `launch_session`, stores `GameScreen::Loading { map_name, skirmish_launch: Some(session) }`, and no longer collapses into `SkirmishSettings`.
4. Egui fallback path may build a minimal session from `SkirmishSettings`, or pass `None` and stay on legacy helper for one slice. The native shell path must use `Some(session)`.
5. Update transition fallback construction for `MapLoadResult` without changing gameplay.

**Focused tests:**
- Add or update pure tests where existing app transition functions are testable.
- Compile check catches all loading enum construction sites.

**Verification command:**
- `cargo check`

### Stage 4: Apply Launch Options Into Simulation

**Files:**
- Modify `src/sim/game_options.rs`
- Modify `src/app_init.rs`
- Modify `src/sim/world/world_hash.rs` only if new options are added beyond existing fields

**Work:**
1. Change `GameOptions::default().build_off_ally` to `true`.
2. When `skirmish_launch` is present, set `sim.game_options = session.options.to_game_options()` before runtime house creation and startup seeding.
3. Set `ai_players` to `session.opponents.len() as i32`.
4. Preserve `unit_count` in `GameOptions`; do not consume it as exact Battle budget until the Battle callback gate is resolved.

**Focused tests:**
- `game_options_default_build_off_ally_matches_standard_yr`
- `skirmish_launch_options_set_ai_player_count`
- `game_options_hash_changes_when_build_off_ally_changes`

**Verification command:**
- `cargo test game_options skirmish_launch_options -- --nocapture`

### Stage 5: Create Runtime Skirmish Houses From Session Slots

**Files:**
- Modify `src/app_skirmish.rs`
- Modify `src/app_init.rs`
- Modify `src/sim/house_state.rs` if launch metadata is stored on houses
- Modify `src/sim/world/world_hash.rs` if new deterministic house fields are added

**Work:**
1. Add an app-level normalized slot list: local first, then enabled AI rows in shell order.
2. Generate deterministic owner names from country names with collision-safe suffixing when duplicate countries are selected, for example `Americans`, `Americans_2`, `Russians`.
3. Build a session-derived `HouseColorMap` from the normalized player slots, mapping each generated owner name to the selected `color_index`.
4. Merge or preserve neutral/special/civilian map-roster colors, but session player colors must win for active skirmish owners.
5. In `app_init::load_map`, insert session player `HouseState`s before map-authored entities are spawned, or immediately after spawning call a focused recount helper that recomputes `owned_building_count` and `owned_unit_count` from `sim.entities`.
6. Clear or replace only skirmish player houses for the session-driven path; retain neutral/special/civilian map-authored houses.
7. Insert `HouseState::new` for each active player with side, country, human flag, starting credits, and tech level from the session.
8. Store color/team/start/difficulty either in an app-init side table consumed immediately or in new hashed `HouseState` fields if they must survive initialization.
9. Register AI players from session AI slots, not from map roster order.
10. Preserve map roster parsing for alliances and map-authored data, but do not use it as the source of active player slots on the native shell path.
11. Pass the session-derived `HouseColorMap` into `spawn_entities` and every later `build_entity_atlases` call for the native shell path.

**Focused tests:**
- `skirmish_create_houses_uses_launch_slots_not_map_roster`
- `skirmish_create_houses_preserves_local_then_ai_order`
- `skirmish_duplicate_countries_get_stable_owner_names`
- `skirmish_ai_registration_uses_enabled_session_rows`
- `skirmish_session_house_color_map_uses_selected_row_colors`
- `skirmish_session_house_counts_include_existing_map_entities`

**Verification command:**
- `cargo test skirmish_create_houses -- --nocapture`

### Stage 6: Assign Starts For All Active Slots

**Files:**
- Modify `src/app_skirmish.rs`
- Modify tests in the same file or an adjacent test module

**Work:**
1. Add `build_start_assignment_table(session, starts, active_houses)`.
2. Consume explicit `LaunchStartPosition::Waypoint(0..=7)` for local and AI rows.
3. Assign human slots before AI slots when choosing automatic starts.
4. Detect duplicate explicit start requests and return a typed validation/apply error unless current verified native validation permits them.
5. For deficient waypoints, set `unsupported_deficient_starts: true` and use a deterministic documented fallback policy only for the smoke-test path; full fallback requires the RE gate.
6. Set `HouseState.base_center` and `HouseState.waypoint_edge` for every successfully assigned active house.

**Focused tests:**
- `skirmish_explicit_start_table_assigns_human_then_ai`
- `skirmish_ai_explicit_start_is_not_ignored`
- `skirmish_duplicate_explicit_starts_are_rejected_for_first_slice`
- `skirmish_deficient_waypoints_return_explicit_unsupported_flag`
- `skirmish_base_center_and_waypoint_edge_set_for_all_assigned_houses`

**Verification command:**
- `cargo test skirmish_start_assignment -- --nocapture`

### Stage 7: Replace Native Shell Two-Player MCV Seeder

**Files:**
- Modify `src/app_skirmish.rs`
- Modify `src/app_init.rs`
- Regenerate entity atlases after session-driven startup, as current code already does after seeding

**Work:**
1. Add `generate_battle_startup_mcvs_for_session`.
2. Loop every active non-special session-created house; do not use `take(2)`.
3. Pick base unit type through existing data-driven `skirmish_mcv_type_for_house` equivalent adapted to normalized session house data and `RuleSet`.
4. Spawn one MCV at each assigned start for the first implementation slice.
5. Keep `UnitCount` in `GameOptions` and result metadata but do not invent extra unit formulas before the Battle callback RE gate.
6. Return `SkirmishLaunchApplyResult` with `local_owner`, `active_player_count`, and `spawned_mcv_count`.

**Focused tests:**
- `skirmish_startup_does_not_cap_two_players`
- `skirmish_startup_spawns_one_mcv_attempt_per_active_house`
- `skirmish_startup_preserves_local_owner_from_local_slot`
- `skirmish_unit_count_is_carried_but_not_claimed_as_consumed`

**Verification command:**
- `cargo test skirmish_startup -- --nocapture`

### Stage 8: Integration Check And Legacy Containment

**Files:**
- Modify `src/app_init.rs`
- Modify `src/app_skirmish.rs`
- Modify `src/ui/main_menu.rs` only if legacy egui fallback is migrated in this slice
- Modify `src/app_spawn_pick.rs` if `seed_skirmish_opening_if_needed` signature or behavior changes

**Work:**
1. Ensure `seed_skirmish_opening_if_needed` is not called for `Some(skirmish_launch)`.
2. Keep the legacy helper behind a clearly named compatibility path for `None`, or migrate egui fallback to build a minimal `SkirmishLaunchSession`.
3. Add log messages that distinguish `session-driven skirmish startup` from `legacy skirmish startup`.
4. Confirm native shell launch never uses the current two-player `take(2)` path.
5. Decide explicitly whether `app_spawn_pick.rs` remains legacy-only for this slice. If it remains legacy-only, keep its existing behavior compiling through a compatibility wrapper and add a comment naming it as outside the native shell session path.

**Focused tests:**
- `native_shell_launch_skips_legacy_two_player_seeder`
- `legacy_egui_launch_path_remains_explicit_until_migrated`
- `spawn_pick_legacy_seeder_still_compiles_or_is_explicitly_removed`

**Verification command:**
- `cargo check`
- `cargo test skirmish -- --nocapture`

---

## Acceptance Criteria For The First Implementation Slice

- Native skirmish shell Start creates a `SkirmishLaunchSession` containing selected map index/file, local slot, every enabled AI slot, and launch options.
- `GameScreen::Loading` carries the optional session into `app_init::load_map`.
- `sim.game_options` is populated from the session for the native shell path, including `build_off_ally: true` by default and `unit_count` preserved.
- Native shell startup creates runtime houses from session slots, not from map roster ordering.
- Native shell startup uses a session-derived `HouseColorMap`, so selected local and AI row colors are present in render remap/palette atlas inputs.
- Runtime session houses are inserted early enough, or owned counts are replayed after insertion, so any map-authored entities owned by active session players contribute to deterministic `HouseState` counts.
- Native shell startup creates more than two active player houses when more than one AI row is enabled.
- Native shell startup no longer uses the `pairings.take(2)` shortcut as its final path.
- Local owner comes from the local launch slot and AI registration comes from enabled AI session rows.
- Explicit local and AI starts flow into the assignment stage; unsupported duplicate/deficient cases fail or surface a typed unsupported result rather than silently spawning nothing.
- Every successfully assigned active house gets `base_center` and `waypoint_edge`.
- World hashing remains deterministic: changed `GameOptions` and any new deterministic `HouseState` fields affect `state_hash`; app-only launch data is not retained in `sim/`.
- No `sim/` module imports `ui/`, `render/`, `sidebar/`, `audio/`, or `net/`.
- Focused tests listed through Stage 8 pass, plus `cargo check`.
- The implementation documentation or test names explicitly state that selected-map token loader parity and exact Battle `+0x84` start-unit parity remain RE-gated.

## Sources & References

- Design doc: `docs/plans/2026-05-22-skirmish-launch-session-design.md`
- Research: `docs/research/skirmish-ui/SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`
- Research: `docs/research/skirmish-ui/SKIRMISH_BUILDOFFALLY_FIRST_CONSUMER_GHIDRA_REPORT.md`
- Research: `docs/research/skirmish-ui/SKIRMISH_PREVIEWPACK_CHANNEL_ORDER_AND_MENU_CALLER_GHIDRA_REPORT.md`
- Live Ghidra refresh: `ScenarioClass__Full_Init @ 0x00686B20`
- Live Ghidra refresh: `ScenarioClass__Create_Houses @ 0x00687F10`
- Live Ghidra refresh: `ScenarioClass__AssignStartingPoints @ 0x005EE9D0`
- Live Ghidra refresh: `ScenarioClass__Gather_Start_Positions @ 0x00688380`
- Live Ghidra refresh: `ScenarioClass__Post_Map_Init @ 0x00686890`
- Live Ghidra refresh: `FUN_005D6D80 @ 0x005D6D80`
- Live Ghidra refresh: `FUN_004A8EB0 @ 0x004A8EB0`
- INI: `ini/rulesmd.ini [MultiplayerDialogSettings]`
- INI: `ini/rulesmd.ini` typo-preserved `EligibileForAllyBuilding=yes` entries at `GACNST`, `NACNST`, `YACNST`, and `YACOMD`
- Current code: `src/ui/skirmish_shell/state.rs`
- Current code: `src/ui/main_menu.rs`
- Current code: `src/ui/game_screen.rs`
- Current code: `src/app.rs`
- Current code: `src/app_transitions.rs`
- Current code: `src/app_init.rs`
- Current code: `src/app_skirmish.rs`
- Current code: `src/app_spawn_pick.rs`
- Current code: `src/sim/game_options.rs`
- Current code: `src/sim/house_state.rs`
- Current code: `src/sim/world/world_hash.rs`
