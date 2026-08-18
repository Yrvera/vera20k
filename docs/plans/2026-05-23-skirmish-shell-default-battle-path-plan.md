# Skirmish Shell Default Battle Path Implementation Plan

> Execute only after user approval. This is an implementation plan, not the implementation.

**Goal:** Make the native offline Skirmish shell the default stock Battle-style player path through first playable frame, including shell validation, launch packing, house/start setup, MCV creation, and `UnitCount` start-unit budget behavior.

**Design Doc:** `docs/plans/2026-05-23-skirmish-shell-default-battle-path-design.md`

---

## Grounding Summary

- Current Rust now routes the native main-menu shell `SinglePlayer` action into the native Skirmish shell. The old "add explicit route first" portion of this plan is superseded.
- The native shell path already has `SkirmishShellState`, verified layout/render helpers, `SkirmishLaunchSession`, `pending_skirmish_launch_session`, and `apply_skirmish_launch_session`.
- Player-name editing, ordinary Start validation modals, selected mode carried in the launch session, mode-aware Team combo rows, and deficient-start fallback have all moved since this plan was first written.
- The current native launch-session path is better than the old two-MCV shortcut, but it still does not model the full native Battle-style first-playable-frame setup.
- The old egui/fallback path still uses `SkirmishSettings` and `seed_skirmish_opening_if_needed` when no native launch session is present; that path is not the parity route.
- Research verifies the standard offline Skirmish shell is dialog `0x102`, with Start `0x617`, Back `0x5C0`, native validation modals, player-name edit `0x6A0`, packed node/AI arrays, and post-shell scenario/start-unit consumers.
- This plan targets stock Battle/ManBattle-style offline Skirmish first. Full mode payload handling, Cooperative/Siege/Unholy custom callbacks, and RA2MD.INI persistence are explicit follow-ups.

## Key Decisions

- Treat the native shell route as already active from the native main menu. The remaining default-path gate is first-playable-frame correctness, not route creation.
- Keep the native shell as app state, not as egui UI or a dev overlay.
- Keep shell/UI state above `sim/`; `sim/` receives normalized deterministic game state only.
- Continue using `SkirmishLaunchSession` as app-level data rather than adding UI dependencies to map or sim modules.
- Implement Battle-style startup as staged app initialization: slot normalization, house creation, start assignment, standard MCV/base callback, then standard extra-unit budget.
- Use parsed rules data for BaseUnit and unit eligibility; do not hardcode AMCV/SMCV/PCV country branches in the parity path.
- Random country/color selection is now a blocking default-route correctness risk because the current code still uses wall-clock `SystemTime`; either block Random selections in the native default route or implement verified deterministic shell RNG before accepting default-route parity.
- Treat RA2MD.INI persistence as blocked on a separate focused investigation or implementation contract.

## Current Recheck - 2026-05-23

This section supersedes the broad task ordering below. The older task ledger remains for provenance and acceptance context, but implementation should start from the current slice here.

### Already Implemented Or Mostly Implemented

| Original task | Current status | Evidence |
|---|---|---|
| Task 1: explicit native shell route | Superseded by current route. Main-menu shell `SinglePlayer` now sets `main_menu_show_native_skirmish_shell = true`. | `src/app.rs` |
| Tasks 2-3: player-name edit state/render/input | Implemented enough for this default-path slice; remaining exact selection/focus choreography is deferred. | `src/ui/skirmish_shell/state/player_name.rs`, `src/app.rs`, render text/modal modules |
| Task 4: ordinary Start validation modal | Implemented for capacity, no-opponent, and same-explicit-team failures. Remaining work is visual/template/keyboard parity. | `src/app.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render/*` |
| Task 6: launch session expansion | Mostly implemented: selected mode, selected map token, player name, local/AI slots, and visible options are present. | `src/skirmish_launch.rs`, `src/ui/skirmish_shell/state/launch.rs` |
| Task 10: start assignment table | Partially implemented: explicit and auto starts are assigned from launch slots. Native random/farthest nuances may still need audit. | `src/app_skirmish.rs` |
| Task 11: deficient start fallback | Implemented as an 8x8 Track-passable scan and tested. Treat remaining exact native random/farthest selection as audit follow-up. | `src/app_skirmish.rs` |
| Mode-aware Team combo rows | Implemented after the original plan: Team Game omits `None`, FFA keeps it, inactive-row default follows `AlliesAllowed`. | `src/ui/skirmish_shell/state/combos.rs`, `state/player_name.rs`, `state/tests.rs` |

### Current Implementation Slice

Execute these in order. Do not start by replaying the old Tasks 1-6.

#### Slice 1: Remove Wall-Clock Random From The Default Route

**Status 2026-05-23:** Completed for the current launch surface. Country Random now returns `LaunchValidationError::RandomSelectionUnverified { slot }` instead of choosing from wall-clock time, and concrete country/color launches still succeed. Color Random does not currently persist into `SkirmishShellState`; the color sentinel row is ignored before launch, so there is no wall-clock color path to block yet.

**Why:** The native shell is now reachable from the normal native main menu. Current Random country resolution uses `SystemTime`, which is non-deterministic and not verified against `gamemd.exe`.

**Files:**

- `src/ui/skirmish_shell/state/launch.rs`
- `src/ui/skirmish_shell/state/combos.rs`
- `src/skirmish_launch.rs`
- tests in `src/ui/skirmish_shell/state/tests.rs`

**Steps:**

1. Stop resolving Random country/color with wall-clock time in the default native launch path.
2. Until the native shell RNG contract is verified, return `LaunchValidationError::RandomSelectionUnverified { slot }` for any Random side/color selection that reaches Start.
3. Map that error to a temporary visible validation modal with a clear body such as `Random side/color selection is not available yet.` and the existing OK dismissal behavior. This is non-native guard text and must be replaced after the shell RNG contract is verified; do not leave Start silent or permanently disabled.
4. Keep tests deterministic and explicit: concrete country/color launches succeed; Random launch does not silently choose a side/color.

**Tests:**

- `skirmish_launch_rejects_random_country_until_rng_verified`
- `skirmish_launch_rejects_random_color_until_rng_verified` once color Random has launch-state plumbing
- `skirmish_concrete_country_color_launch_session_still_succeeds`

#### Slice 2: Replace Hardcoded MCV Candidate Selection With Parsed BaseUnit

**Status 2026-05-24:** Completed for the current standard launch path. `apply_skirmish_launch_session` now chooses the opening MCV by iterating parsed `[General] BaseUnit` in rules order and checking the candidate object's `Owner`, `RequiredHouses`, and `ForbiddenHouses` against the selected launch country. The old hardcoded country candidate ordering is no longer used by this path.

**Why:** Native standard Battle startup chooses the opening MCV from `[General] BaseUnit` and type owner/side masks, not from `LaunchCountry::opening_mcv_candidates`.

**Files:**

- `src/app_skirmish.rs`
- `src/skirmish_launch.rs`
- `src/rules/ruleset.rs`
- `src/rules/object_type.rs`

**Steps:**

1. Use the already-parsed `RuleSet::base_unit_types` list as the candidate order.
2. First verify whether `ObjectType` already exposes the owner/house-mask data needed to match the native side mask. If it does not, add that parser/surface before changing MCV selection.
3. Match candidates to the launch house side using parsed owner/house-mask semantics.
4. Remove `LaunchCountry::opening_mcv_candidates` from the parity path; keep it only for legacy fallback if still needed.
5. Add a test proving rule order wins over country hardcoding.

**Tests:**

- `skirmish_baseunit_vector_selects_side_matching_mcv`
- `skirmish_baseunit_selection_uses_rules_order`
- `skirmish_launch_does_not_use_country_hardcoded_mcv_for_parity_path`
- `skirmish_baseunit_selection_respects_required_and_forbidden_houses`

#### Slice 3: Implement Standard MCV Callback Placement

**Status 2026-05-24:** Implemented first-pass standard selected-mode behavior in the Rust launch path. Assigned starts now populate each launch house's `base_center`/`waypoint_edge` before MCV placement; `Bases=no` skips MCV creation while preserving house/start state; `UnitCount=0` does not suppress Bases-enabled MCV creation; and blocked assigned MCV cells use a deterministic direct-then-fallback placement helper. Remaining non-exact parity: the fallback helper does not yet replay gamemd's randomized direction/jitter order from `FUN_00688ED0`, though it uses the verified direct-first search shape and radius range.

**Why:** Current `apply_skirmish_launch_session` directly spawns one MCV at each assigned waypoint. Native standard mode runs an MCV/base callback, respects `Bases`, places at the base center, and then searches a radius-1 fallback.

**Files:**

- `src/app_skirmish.rs`
- `src/sim/world/world_spawn.rs`
- placement/passability helpers as needed

**Steps:**

1. Split launch application into named stages if needed: normalize slots, create houses, assign starts/base centers, run standard MCV callback, then run extra-unit budget.
2. Respect `session.options.bases`; `UnitCount=0` must not suppress MCV creation.
3. Try direct placement first, then deterministic radius-1 fallback as close as current spawn APIs allow.
4. Return/report placement failures without hiding them.

**Tests:**

- `skirmish_unit_count_zero_spawns_mcv_only_when_bases_enabled`
- `skirmish_bases_off_skips_standard_mcv_callback`
- `skirmish_assigned_start_sets_house_base_cell_before_mcv_spawn`
- `skirmish_mcv_start_uses_radius_fallback_when_start_cell_blocked`

#### Slice 4: Implement UnitCount Budget And Standard Extra Units

**Why:** The Unit Count slider is a money-like budget over eligible starting-unit types. It is not a literal unit count and is currently not consumed by startup generation.

**Files:**

- `src/app_skirmish.rs`
- `src/rules/*`
- `src/sim/world/world_spawn.rs`

**Steps:**

1. Build eligible unit and infantry candidate lists from rules data.
2. Require `AllowedToStartInMultiplayer`, tech level `<=` house tech level, and side/house-mask intersection.
3. Exclude `BaseUnit` entries from the appropriate candidate list.
4. Compute the verified rounded average-cost budget.
5. Place extra units using the verified radius-4 fallback intent.
6. Implement or explicitly block leftover-credit behavior before declaring UnitCount parity.

**Tests:**

- `skirmish_start_unit_budget_filters_spawnable_tech_and_house_mask`
- `skirmish_start_unit_budget_excludes_baseunit_entries`
- `skirmish_positive_unit_count_spawns_extra_starting_units`
- `allowed_to_start_in_multiplayer_parses_and_defaults_yes`

**Status 2026-05-24:** First-pass implemented for the standard Battle launch path. Rust now parses `AllowedToStartInMultiplayer`, computes the verified rounded average-cost budget across eligible vehicle/infantry types, excludes `[General] BaseUnit` entries, filters by tech and launch-country ownership masks, preserves MCV creation when `UnitCount=0`, and places deterministic extra units near each assigned base center with radius-4 fallback intent.

**Remaining non-exact pieces before declaring full UnitCount parity:** selected-mode RNG/pick stream, exact native candidate pick order, shared budget/leftover-credit mutation, exact initial mission/veterancy side effects, and exact placement scanner order inside the radius-4 helper.

### Current Non-Goals

- Do not redo player-name edit, validation modal, status-help, or Team combo work except for focused bug fixes found during implementation.
- Do not implement full random country/color RNG without a verified native RNG contract.
- Do not implement full Cooperative/Siege/Unholy custom mode callbacks in this Battle-path slice.
- Do not claim screenshot-perfect modal/chooser art parity from this slice.

## Historical File Map

This file map belongs to the original broad plan. It is kept for ownership context only; the current implementation entry point is the "Current Implementation Slice" above.

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/app.rs` | Add a real native Skirmish shell app state, initially behind an explicit route; handle validation modal, player-name input, default-capable loading handoff, and the final default route flip after acceptance. |
| Modify | `src/ui/skirmish_shell/state.rs` | Add player-name edit state, modal state, native validation-to-modal mapping, richer launch packing, and random-resolution hooks. |
| Modify | `src/ui/skirmish_shell/layout.rs` | Preserve verified rects; add named helpers only where player-name/modal hit testing needs them. |
| Modify | `src/ui/skirmish_shell/mod.rs` | Re-export new shell state/actions/helpers. |
| Modify | `src/app_skirmish_shell_render.rs` | Render player-name edit, validation modal, and any remaining default-path shell controls/text. |
| Modify | `src/render/skirmish_shell_chrome.rs` | Add atlas entries only if required for edit/modal assets not already present. |
| Modify | `src/skirmish_launch.rs` | Expand launch contract with player name, mode id, native row/item data where needed, random-resolved slot data, options, and setup flags. |
| Modify | `src/app_init.rs` | Ensure the native launch-session path is used for default Skirmish loading and legacy egui seeding is not used by default. |
| Modify | `src/app_skirmish.rs` | Refactor launch application into native stages: launch houses, start table, MCV/base callback, UnitCount budget, placement fallback. |
| Modify | `src/sim/game_options.rs` | Preserve option defaults/effects needed by launch setup and first-frame behavior. |
| Modify as needed | `src/rules/*` | Expose parsed BaseUnit, owner/house masks, TechLevel, AllowedToStartInMultiplayer, and cost data to the app-level start generator. |
| Modify as needed | `src/sim/world/world_spawn.rs` | Expose deterministic placement helpers needed for native MCV/extra-unit placement fallback. |

## Historical Parity-Critical Items

This table belongs to the original broad plan. The "Current Implementation Slice" above is authoritative when it conflicts with these older rows; in particular, Random country/color is currently blocked rather than resolved.

| Item | Implementation Home | Verification |
|---|---|---|
| Main menu enters native shell by default | `app.rs` | App/state test or smoke test. |
| Existing egui setup remains debug fallback only | `app.rs`, `ui/main_menu.rs` | Manual fallback toggle or env test. |
| Player-name edit `0x6A0` is editable, capped, rendered, and committed | `state.rs`, render, app input | Unit tests plus screenshot/focus check. |
| Start validation failures show native text and keep shell alive | `state.rs`, `app.rs`, render | State/app tests for capacity, no opponent, same team. |
| Launch session carries all visible shell rows/options | `skirmish_launch.rs`, `state.rs` | `launch_session` tests with multiple AI rows. |
| Random country/color are resolved before house creation | `state.rs` or `skirmish_launch.rs` | Deterministic fake-RNG unit test. |
| Runtime houses come from launch slots, not playable map roster order | `app_skirmish.rs` | House creation test. |
| Explicit starts populate a start table and assign human before AI | `app_skirmish.rs` | Start assignment tests. |
| Deficient starts use fallback path, not no-spawn | `app_skirmish.rs` | Deficient map test. |
| MCV type comes from BaseUnit plus side masks | rules/app setup | BaseUnit selection test. |
| `UnitCount=0` still permits MCVs when `Bases=yes` | `app_skirmish.rs` | Startup generation test. |
| Extra units use budget and eligibility gates | rules/app setup | Candidate filter and budget tests. |
| Placement uses direct place then deterministic fallback | spawn/app setup | Blocked-start placement test. |

---

## Historical Task Ledger

The task list below predates the 2026-05-23 current recheck. Use it as a source of acceptance details and older rationale, not as the execution order. The current execution order is the "Current Implementation Slice" above.

### Task 1: Introduce A Real Native Skirmish Shell App State Behind An Explicit Route

**Why:** The current `dev_skirmish_shell_enabled` flag is both a debug toggle and route selector. The default-capable path needs a first-class app state before it becomes the normal player route.

**Files:**

- `src/app.rs`
- any file defining `GameScreen`

**Steps:**

1. Add a route/screen state for native Skirmish shell visibility, or split the current main-menu state into explicit main menu vs native shell substates.
2. Keep the existing dev/env toggle as a fallback override, not the primary player route.
3. Add an explicit app route into the native shell for implementation/testing, but do not yet make the normal main-menu Single Player action default to it.
4. Ensure Back `0x5C0` returns to the main menu instead of exiting the application unless verified retail behavior for the exact path says otherwise.
5. Keep rendering/input dispatch explicit for main menu shell, native skirmish shell, modal, loading, and in-game.

**Tests:**

- `skirmish_explicit_route_enters_native_shell`
- `skirmish_back_returns_to_main_menu_or_matches_verified_exit_contract`

**Checks:**

- `cargo test skirmish_explicit_route --lib`

### Task 2: Add Player-Name Edit State And Input

**Why:** The retail `0x6A0` control is editable and committed on Start. Current Rust renders a static `"Player"` label.

**Files:**

- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/layout.rs`
- `src/app.rs`
- `src/app_skirmish_shell_render.rs`

**Steps:**

1. Add `player_name` to `SkirmishShellState`, with a default matching the current native/global default used by Rust.
2. Add edit focus state for `0x6A0`, including hit testing against `layout.player_name`.
3. Route text input and relevant keyboard events while the edit has focus.
4. Enforce the 19-character cap from `EM_SETLIMITTEXT 0x13`.
5. Add caret state sufficient for a visible 2px focused caret.
6. Commit `player_name` into `SkirmishLaunchSession` on Start.

**Tests:**

- `skirmish_player_name_edit_accepts_text_and_start_commits_19_char_limit`
- `skirmish_player_name_edit_focus_changes_on_rect_click`
- `skirmish_player_name_edit_backspace_and_limit_match_contract`

**Checks:**

- `cargo test player_name --lib`

### Task 3: Render Player-Name Edit Visuals

**Why:** The edit must look like the native owner-draw edit, not a generic label.

**Files:**

- `src/app_skirmish_shell_render.rs`
- `src/ui/skirmish_shell/layout.rs`
- `src/render/skirmish_shell_chrome.rs` if an atlas entry is needed

**Steps:**

1. Render the edit primitive frame in the final rect `(58,59,151,23)` at 800x600, using existing layout output.
2. Render text with the verified left+2 edit inset and yellow shell color.
3. Render a 2px caret when focused and not selecting.
4. Avoid NewEdit-only behavior; this is ordinary `Edit` callback behavior.

**Tests:**

- `skirmish_player_name_edit_uses_binary_frame_inset_and_caret_rect`
- `skirmish_player_name_render_does_not_use_static_label_rect`

**Checks:**

- `cargo test skirmish_player_name --lib`
- Screenshot check at 800x600 after implementation.

### Task 4: Add Native Validation Modal State

**Why:** Validation failures are player-visible shell modals. Logging a warning is not parity.

**Files:**

- `src/ui/skirmish_shell/state.rs`
- `src/app.rs`
- `src/app_skirmish_shell_render.rs`
- CSF/localization helper modules if needed

**Steps:**

1. Add `SkirmishValidationModalState` with body text, OK text, and blocking flag.
2. Map `LaunchValidationError::MapCapacityExceeded` to `TXT_SCENARIO_TOO_SMALL`, formatted with capacity.
3. Map `NoEnabledOpponent` to `TXT_NEED_AT_LEAST_TWO_PLAYERS`.
4. Map `SameExplicitTeam` to `TXT_CANNOT_ALLY`.
5. Use `TXT_OK` for OK/control text.
6. Keep shell alive and re-enable Start after failure.
7. Add OK button hit handling and modal dismissal.

**Tests:**

- `skirmish_start_capacity_modal_uses_retail_csf_text`
- `skirmish_start_no_opponent_modal_uses_retail_csf_text`
- `skirmish_start_same_team_modal_uses_cannot_ally_text`
- `skirmish_start_failure_keeps_shell_active`

**Checks:**

- `cargo test skirmish_start_ --lib`

### Task 5: Complete Default-Path Shell Render Gaps

**Why:** The default screen cannot ship with persistent visible chrome/text drift that is already verified.

**Files:**

- `src/app_skirmish_shell_render.rs`
- `src/ui/skirmish_shell/layout.rs`
- `src/render/skirmish_shell_chrome.rs`

**Steps:**

1. Audit current code against the design ledger for right-panel static text, bottom-cap source clipping, parent/right-panel draw order, and first-paint `SDBTNANM` state.
2. Fix only default-path visible mismatches that are implementation-safe from existing docs.
3. Keep `>800` parent-background SHP skipped for fresh Skirmish.
4. Preserve PreviewPack RGB decode and `[Header]`-only live marker behavior.

**Tests:**

- `skirmish_right_panel_bottom_cap_uses_source_clip`
- `skirmish_semantic_draw_order_matches_default_first_paint`
- `skirmish_high_res_default_shell_keeps_no_parent_background_above_800`

**Checks:**

- `cargo test skirmish_shell_semantic --lib`
- Screenshot checks at 640x480, 800x600, 1024x768.

### Task 6: Expand `SkirmishLaunchSession`

**Why:** The current contract is narrower than gamemd.exe's Start packing and does not carry all player-visible setup data.

**Files:**

- `src/skirmish_launch.rs`
- `src/ui/skirmish_shell/state.rs`
- `src/app.rs`
- `src/app_init.rs`

**Steps:**

1. Add `player_name` to the local slot or session.
2. Add selected mode id/type for stock Battle/ManBattle handling.
3. Preserve native AI row item data separately from semantic difficulty if needed for testing/diagnostics.
4. Add random-choice representation and a resolved slot result.
5. Add all visible option mirrors already present in shell state: credits, game speed, unit count, short game, super weapons, build off ally, crates, MCV redeploy, bases, bridge destruction, shroud, fog, and related existing game options.
6. Represent forced launch flags only when a known first consumer exists; otherwise document them at the boundary.
7. Update constructors/defaults so current tests remain deterministic.

**Tests:**

- `skirmish_launch_session_preserves_player_name`
- `skirmish_launch_session_preserves_trackbars_and_checkboxes`
- `skirmish_launch_session_packs_all_enabled_rows`
- `skirmish_ai_row_item_data_preserves_native_difficulty_order`

**Checks:**

- `cargo test skirmish_launch --lib`

### Task 7: Resolve Random Country And Color Before Loading

**Why:** gamemd.exe resolves random assignments before scenario house creation consumes node/AI data.

**Files:**

- `src/ui/skirmish_shell/state.rs`
- `src/skirmish_launch.rs`
- possibly app-level deterministic RNG helper

**Steps:**

1. Replace the current wall-clock `SystemTime` random country helper with deterministic launch-session random resolution.
2. Add a research gate for the gamemd.exe RNG seed/source used by shell random country/color resolution before Random selections are accepted in the default parity route.
3. Until that gate is closed, either block Random selections with a visible unsupported/default-path guard or force the default path to concrete country/color choices in tests and UI defaults.
4. Resolve random country over the verified 0..9 country range only after the RNG gate is satisfied.
5. Resolve random color over 0..7 with uniqueness checks among already-used player colors only after the RNG gate is satisfied.
6. Store both requested and resolved values if useful for diagnostics.
7. Keep tests deterministic by injecting a fake/random sequence that models the verified RNG contract.

**Tests:**

- `skirmish_random_country_resolves_before_house_creation`
- `skirmish_random_color_uses_unique_0_to_7_assignment`
- `skirmish_random_resolution_is_deterministic_under_test_rng`
- `skirmish_default_path_blocks_random_selection_until_rng_contract_is_verified`

**Checks:**

- `cargo test random_color --lib`

### Task 8: Convert Start To Session-Driven Loading Only

**Why:** The default parity path must not fall back to `SkirmishSettings` and the old seeder.

**Files:**

- `src/app.rs`
- `src/app_transitions.rs`
- `src/app_init.rs`
- `src/ui/main_menu.rs`

**Steps:**

1. Ensure native shell Start stores `pending_skirmish_launch_session`.
2. Ensure loading always receives the launch session for default native shell starts.
3. Keep legacy egui setup converted to a minimal session if it remains accessible as fallback.
4. Prevent `seed_skirmish_opening_if_needed` from running on native session starts.
5. Add logging that clearly identifies session-driven startup vs legacy fallback.

**Tests:**

- `skirmish_native_start_sets_pending_launch_session`
- `skirmish_session_loading_bypasses_legacy_two_mcv_seeder`

**Checks:**

- `cargo test pending_skirmish --lib`

### Task 9: Refactor Launch House Creation

**Why:** Native `Create_Houses` creates player houses from launch node/AI arrays, not map playable-house roster order.

**Files:**

- `src/app_skirmish.rs`
- `src/app_init.rs`
- `src/sim/house_state.rs` if new state fields are needed
- `src/sim/world/world_hash.rs` if new deterministic house fields are added

**Steps:**

1. Split `apply_skirmish_launch_session` into named stages.
2. Populate neutral/special/non-player houses from the map roster first.
3. Populate launch player houses from local slot plus active AI slots.
4. Apply credits, color, country, side, human flag, AI difficulty, explicit start, and team.
5. Register AI players from launch AI slots, not from roster leftovers.
6. Preserve deterministic `BTreeMap` house iteration.
7. Recount existing map-authored object ownership after house replacement if needed.
8. If new house/team/start/difficulty fields persist in `Simulation`, include them in `Simulation::state_hash` or document why they are app-only and cannot affect deterministic gameplay.

**Tests:**

- `skirmish_create_houses_uses_launch_slots_not_map_roster`
- `skirmish_launch_houses_preserve_colors_credits_country_and_difficulty`
- `skirmish_ai_players_registered_from_active_slots`
- `skirmish_launch_house_state_fields_affect_state_hash_when_gameplay_visible`

**Checks:**

- `cargo test apply_skirmish_launch_session --lib`

### Task 10: Implement Battle-Style Start Assignment Table

**Why:** Explicit starts are consumed through a scenario start table and assigned human-first, then AI.

**Files:**

- `src/app_skirmish.rs`
- `src/map/waypoints.rs` if helper access is needed

**Steps:**

1. Build an app-level `StartAssignmentTable` equivalent to `ScenarioClass+0x1180`.
2. Preassign explicit `LaunchStartPosition::Position` values by row/house.
3. Gather waypoints 0..7 in native order.
4. Assign human-controlled houses first, then AI houses.
5. Assign remaining auto starts from unused waypoints in verified order.
6. Mark deficient starts when authored waypoints are insufficient.

**Tests:**

- `skirmish_explicit_start_table_assigns_human_then_ai`
- `skirmish_auto_start_uses_first_unused_native_waypoint_order`
- `skirmish_duplicate_explicit_start_marks_or_resolves_like_native_contract`

**Checks:**

- `cargo test start_assignment --lib`

### Task 11: Add Deficient Start Fallback

**Why:** Native YR generates fallback passable starts when waypoints are deficient; doing nothing is visible drift.

**Files:**

- `src/app_skirmish.rs`
- terrain/passability helpers as needed

**Steps:**

1. Implement a bounded fallback helper for deficient starts using the verified 8x8 passability intent.
2. Use deterministic RNG or existing simulation RNG plumbing, not wall-clock randomness.
3. Return whether fallback was used in `SkirmishLaunchApplyResult`.
4. If exact passability cannot be implemented in this slice, block default maps that need fallback and create a dedicated follow-up; do not silently no-spawn.

**Tests:**

- `skirmish_deficient_start_pool_uses_fallback_instead_of_no_spawn`
- `skirmish_deficient_start_fallback_reports_when_used`

**Checks:**

- `cargo test deficient_start --lib`

### Task 12: Implement BaseUnit MCV Selection

**Why:** Standard MCV type is selected from `[General] BaseUnit` and type owner/house masks, not hardcoded from country.

**Files:**

- `src/app_skirmish.rs`
- `src/rules/*`
- `src/skirmish_launch.rs` if old helper is removed or downgraded

**Steps:**

1. Expose parsed `[General] BaseUnit` from `RuleSet`.
2. Expose or compute type side/house masks for candidate filtering.
3. Select the first BaseUnit whose mask matches the house side.
4. Fall back only through documented data fallback, not hardcoded country order.
5. Update or remove `LaunchCountry::opening_mcv_candidates` from the parity path.

**Tests:**

- `skirmish_baseunit_vector_selects_side_matching_mcv`
- `skirmish_baseunit_selection_uses_rules_order`
- `skirmish_launch_does_not_use_country_hardcoded_mcv_for_parity_path`

**Checks:**

- `cargo test baseunit --lib`

### Task 13: Implement Standard MCV Placement Callback

**Why:** Standard Battle startup calls the MCV/base callback per active house.

**Files:**

- `src/app_skirmish.rs`
- `src/sim/world/world_spawn.rs`
- placement/passability helpers

**Steps:**

1. Run the standard MCV callback for every active non-special, non-observer launch house.
2. Respect `Bases=no` by skipping MCV creation while leaving UnitCount behavior correct.
3. Place at the assigned base center first.
4. If direct placement fails, call a deterministic fallback search equivalent to the verified radius-1 behavior as closely as current spawn APIs permit.
5. Report failed placements without crashing; tests should assert expected fallback success/failure.

**Tests:**

- `skirmish_unit_count_zero_spawns_mcv_only_when_bases_enabled`
- `skirmish_bases_off_skips_standard_mcv_callback`
- `skirmish_start_mcv_uses_direct_place_then_radius_one_fallback`

**Checks:**

- `cargo test skirmish_start_mcv --lib`

### Task 14: Implement UnitCount Candidate Filtering And Budget

**Why:** The UnitCount slider affects starting units through a money-like budget, not a literal unit count.

**Files:**

- `src/app_skirmish.rs`
- `src/rules/*`
- `src/sim/world/world_spawn.rs`

**Steps:**

1. Build eligible unit and infantry candidate lists from rules data.
2. Require `AllowedToStartInMultiplayer`.
3. Require type tech level `<=` house tech level.
4. Require house/side mask intersection.
5. Exclude BaseUnit entries from the appropriate candidate list.
6. Compute the rounded average-cost budget using the verified formula shape.
7. Keep `UnitCount=0` as budget zero, not as "no MCV".

**Tests:**

- `skirmish_start_unit_budget_filters_spawnable_tech_and_house_mask`
- `skirmish_start_unit_budget_excludes_baseunit_entries`
- `skirmish_unit_count_budget_uses_rounded_average_cost`
- `skirmish_unit_count_zero_has_no_extra_unit_budget`

**Checks:**

- `cargo test start_unit_budget --lib`

**Status 2026-05-24:** Implemented for the current standard launch path. Covered by `skirmish_start_unit_budget_filters_spawnable_tech_and_house_mask`, `skirmish_start_unit_budget_excludes_baseunit_entries`, and `allowed_to_start_in_multiplayer_parses_and_defaults_yes`.

### Task 15: Implement Standard Extra-Unit Placement

**Why:** First playable frame must include the native style of extra starting units when UnitCount is nonzero.

**Files:**

- `src/app_skirmish.rs`
- `src/sim/world/world_spawn.rs`

**Steps:**

1. Iterate active non-special, non-observer houses in deterministic order.
2. Spend the UnitCount budget with the standard candidate lists.
3. Place extra units around the base center using the verified radius-4 fallback intent.
4. Apply human vs AI initial mission assignment if the required mission constants already exist; otherwise add a focused follow-up rather than inventing behavior.
5. Implement the verified leftover-credit behavior before declaring UnitCount parity. If the current Rust economy surface cannot represent it correctly, stop this task and add a blocking implementation contract rather than accepting a silent first-frame credit drift.

**Tests:**

- `skirmish_extra_units_iterate_active_houses_in_deterministic_order`
- `skirmish_start_unit_uses_spiral_fallback_when_start_cell_blocked`
- `skirmish_extra_unit_budget_spawns_no_disallowed_types`
- `skirmish_extra_unit_budget_applies_leftover_credit_behavior`

**Checks:**

- `cargo test extra_unit --lib`

**Status 2026-05-24:** Partially implemented as a deterministic first pass. Extra units are seeded after base centers are assigned, including `Bases=no` launches when `UnitCount > 0`; MCV creation remains separately gated by `Bases`. The current spender intentionally avoids unbounded overspend in the deterministic path but is not yet an exact native selected-mode RNG/shared-budget reproduction. Covered by `skirmish_positive_unit_count_spawns_extra_starting_units` plus the focused app-skirmish test suite.

### Task 16: Wire Option Consumers Needed By First Playable Frame

**Why:** Start packing mirrors options that have immediate or early visible consumers.

**Files:**

- `src/skirmish_launch.rs`
- `src/sim/game_options.rs`
- `src/app_skirmish.rs`
- existing crate/superweapon/undeploy/defeat surfaces as applicable

**Steps:**

1. Ensure credits initialize every launch house.
2. Ensure game speed uses the selected shell value for local skirmish pacing.
3. Ensure `Crates` gates initial random crate placement for the first playable frame. If the crate system is not ready, block final acceptance on a crate implementation contract rather than leaving first-frame crates as a soft named gap.
4. Ensure `SuperWeaponsAllowed`, `MCVRedeploys`, `ShortGame`, and `BuildOffAlly` continue to flow into `GameOptions` or their verified consumers.
5. Do not route ShortGame through fog/special-flag semantics.

**Tests:**

- `skirmish_launch_options_apply_credits_to_all_houses`
- `skirmish_launch_options_preserve_game_speed`
- `skirmish_crates_option_gates_initial_crate_path`
- `skirmish_short_game_option_flows_to_defeat_option_not_fog`

**Checks:**

- `cargo test game_options --lib`

### Task 17: Add Default-Path End-To-End Tests

**Why:** The feature is a player flow, not just independent helpers.

**Files:**

- existing integration or app tests
- possible `tests/` fixtures

**Steps:**

1. Add a deterministic small map fixture with 2 starts.
2. Add a deterministic map fixture with 4+ starts.
3. Add a deficient-start fixture if fixture infrastructure permits.
4. Test Start from default shell with 1 human + 1 AI reaches loading with a launch session.
5. Test loaded simulation has the expected local/AI houses, colors, credits, starts, and at least the standard MCVs.
6. Test 3+ active player rows do not collapse to two MCVs.

**Tests:**

- `default_skirmish_start_reaches_first_playable_frame_with_session_houses`
- `default_skirmish_start_supports_more_than_two_active_slots`
- `default_skirmish_start_preserves_selected_colors_credits_and_starts`

**Checks:**

- `cargo test default_skirmish --test <integration-test-name>`

### Task 18: Visual Verification Pass

**Why:** Shell defaulting is player-visible and must be checked by pixels, not just unit tests.

**Files:**

- no code files unless screenshots expose defects
- screenshot output under an ignored diagnostics/logs path

**Steps:**

1. Start the app with native shell default.
2. Capture 640x480, 800x600, and 1024x768 shell screenshots.
3. Verify no blank/egui setup appears in default route.
4. Verify player-name edit, validation modal, right panel, preview, buttons, and dropdowns render in the expected layers.
5. Capture a Start validation modal for capacity/no-opponent/same-team.
6. Capture first playable frame for a 2-player Battle start and a 4-player Battle start.

**Checks:**

- Manual screenshot comparison against verified traces/docs.
- Run focused render tests after any screenshot-driven fixes.

### Task 19: Remove Or Reclassify Legacy Default Skirmish Setup

**Why:** After the native path passes, the egui setup must not remain the default parity route.

**Files:**

- `src/app.rs`
- `src/ui/main_menu.rs`
- docs/readme if needed

**Steps:**

1. Keep egui setup only as an explicit debug fallback, if still useful.
2. Flip the normal main-menu Single Player/Skirmish action into the native shell only after Tasks 1-18 acceptance checks pass.
3. Ensure normal player interactions cannot reach the old shortcut unintentionally.
4. Mark `seed_skirmish_opening_if_needed` as legacy fallback in comments, or split it away from the native path.
5. Remove stale "experimental Skirmish Shell" UI text from the normal player route.

**Tests:**

- `normal_main_menu_does_not_use_egui_skirmish_setup`
- `legacy_skirmish_setup_requires_explicit_debug_fallback`
- `default_route_flips_only_after_native_battle_acceptance_gate`

**Checks:**

- `cargo test main_menu --lib`

---

## Deferred Follow-Ups

These are real parity gaps, but they should not block the stock Battle default path unless implementation discovers a dependency.

- Exact RA2MD.INI read/write persistence for all Skirmish shell settings.
- Full selected-map token/filename loader parity beyond current file-name behavior.
- Full `MPModesMD.ini` override payload loading and rules-override application.
- Cooperative `+0x84`, Siege `+0xC8`, Unholy `+0xC8`, and other non-Battle custom callbacks.
- Exact selected-mode generic rejection behavior for modded/custom mode objects.
- Exact edit selection behavior and old-Edit `0x4B0` / `0x4AF` focus choreography.
- Final retail screenshot RGB sampling for edit/caret and some shell text colors.
- Exact gamemd.exe RNG seed/source for shell Random country/color, unless resolved before Random selections are enabled in the default route.
- Full initial crate placement is deferred from the current Battle-path slice unless explicitly scoped back in.

## Acceptance Gate

The feature is complete when all of these are true:

1. Starting from the native main menu reaches the native offline Skirmish shell, not the egui setup. This is already true in current Rust and should be preserved.
2. The native shell supports default Battle setup, visible validation errors, editable player name, map choice, controls, and Start/Back behavior.
3. Pressing Start with concrete country/color choices produces a launch session with all active rows and visible options preserved.
4. Pressing Start with unverified Random country/color selections does not use wall-clock randomness; it either blocks visibly or uses a later verified deterministic RNG contract.
5. Loading consumes the launch session, not the legacy seeder.
6. First playable frame contains launch-created local/AI houses, selected colors/credits, assigned starts, standard BaseUnit-derived MCVs, and UnitCount extras for stock Battle behavior.
7. A 3+ player setup creates 3+ player starts and does not collapse to two MCVs.
8. Focused tests and screenshots pass at 640x480, 800x600, and 1024x768.
9. Remaining non-Battle/persistence/modal-art/crate-placement gaps are named in follow-up docs or issues rather than hidden behind the default route.
