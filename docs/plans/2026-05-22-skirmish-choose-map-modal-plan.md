# Skirmish Choose Map Modal Implementation Plan

> Execute task-by-task. Do not make the experimental shell the default in this plan.

**Goal:** Replace the current `Choose Map` map-cycling shortcut with a native-shaped retail modal backed by MPModes and source-ordered scenario records.

**Design Doc:** [docs/plans/2026-05-22-skirmish-choose-map-modal-design.md](2026-05-22-skirmish-choose-map-modal-design.md)

---

## Grounding Summary

Primary design:

- `docs/plans/2026-05-22-skirmish-choose-map-modal-design.md`

Reswarm reports:

- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODE_CATEGORY_0X6EB_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_SCENARIO_SOURCE_POPULATION_ORDER_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_ACCEPT_CANCEL_SIDE_EFFECTS_GHIDRA_REPORT.md`

Sibling reports:

- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_FLOW_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`

Current Rust surfaces:

- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/layout.rs`
- `src/ui/skirmish_shell/mod.rs`
- `src/app.rs`
- `src/app_list_maps.rs`
- `src/app_init.rs`
- `src/app_skirmish_shell_render.rs`
- `src/skirmish_launch.rs`

## Key Decisions

- Keep the path behind `dev_skirmish_shell_enabled`.
- Do not wire the modal to sorted `available_maps` as its backing store.
- Add a source-ordered chooser record list distinct from `MapMenuEntry`.
- Add an MPModes model before map filtering.
- Keep modal/listbox state render-agnostic in `ui/skirmish_shell`.
- Let app-level code own opening/closing the modal and preview cache invalidation.
- Preserve current main-menu map selector until the shell path can fully replace it.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Add | `src/skirmish_modes.rs` | Data-driven MPModes roster/default model. |
| Add | `src/skirmish_scenarios.rs` | Source-ordered chooser scenario records and filtering. |
| Modify | `src/lib.rs` | Export new app/shared modules if needed by tests. |
| Modify | `src/app_init.rs` | Add projection/helper compatibility between `SkirmishScenarioRecord` and `MapMenuEntry` if needed. |
| Modify | `src/app_list_maps.rs` | Keep legacy list; add reusable metadata parsing helpers for scenario records. |
| Modify | `src/ui/skirmish_shell/state.rs` | Add selected mode/record identity and Choose Map modal state/actions. |
| Modify | `src/ui/skirmish_shell/layout.rs` | Add dialog `0x6B` modal layout. |
| Modify | `src/ui/skirmish_shell/mod.rs` | Re-export modal/model-facing helpers. |
| Modify | `src/app.rs` | Own modes/records/modal state, route Choose Map open/accept/cancel. |
| Modify | `src/app_skirmish_shell_render.rs` | Render Choose Map modal once state/layout exists. |
| Modify | `src/skirmish_launch.rs` | Carry selected mode id/token and preserve `.sed` map token. |

## Parity-Critical Items

| Item | Verification |
|---|---|
| `ChooseMap0x5aa` no longer increments `selected_map_idx` | Unit test on `apply_action`/app routing. |
| Mode rows come from MPModes, selected by id | `choose_map_mode_combo_populates_stock_mpmodes_and_selects_by_id`. |
| Filtering uses `mode.map_filter`, not label/category | `choose_map_filters_by_selected_mpmode_game_modes`. |
| Empty `GameModes` only matches `standard` | `choose_map_empty_game_modes_matches_standard_only`. |
| Source order preserves `MISSIONSMD.PKT`, `*.PKT`, `*.YRO`, `*.YRM` groups | Builder fixture test. |
| No display-name sorting or duplicate collapse | Fixture with duplicate names out of order. |
| Modal layout uses dialog `0x6B` rects | `choose_map_modal_uses_resource_0x6b_control_rects`. |
| Cancel restores old selected mode/map | Modal state test. |
| Accept commits selected mode/map and rebuilds capacity rows | Modal accept state test. |
| Load failure restores previous committed selection | Injected loader/projection failure test. |
| `RandMap.Sed` is synthetic and mode-gated | Random sentinel tests. |

---

## Tasks

### Task 1: Add MPModes model

**Why:** Choose Map filtering is based on selected MPModes object fields, not UI labels or category names.

**Files:**

- `src/skirmish_modes.rs`
- `src/lib.rs`
- possibly `src/ui/skirmish_shell/state.rs`

**Steps:**

1. Add `SkirmishGameMode` with:
   - `id`
   - `ui_name_key`
   - `tooltip_key`
   - `override_file`
   - `map_filter`
   - `random_maps_allowed`
   - `allies_allowed`
   - `must_ally`
2. Add parser for `ini/mpmodesmd.ini` using the existing INI parser.
3. Parse rows from stock categories:
   - `Battle`
   - `ManBattle`
   - `FreeForAll`
   - `Unholy`
   - `Cooperative`
4. Do not expose stock Siege unless a `[Siege]` row exists.
5. Preserve native mode ordering by sorted numeric id.
6. For this first patch, model override defaults for known stock modes from parsed override files when accessible; otherwise keep explicit fallback defaults and mark missing override source in logs.

**Tests:**

- `parses_stock_mpmodesmd_roster`
- `stock_mpmodes_do_not_include_siege`
- `team_game_has_must_ally`
- `free_for_all_disables_allies`
- `battle_and_free_for_all_allow_random_maps`

**Checks:**

- `cargo test skirmish_modes --lib`

### Task 2: Add source-ordered scenario record model

**Why:** The chooser list is source ordered and record-identity based. `available_maps` is sorted and therefore unsuitable.

**Files:**

- `src/skirmish_scenarios.rs`
- `src/app_list_maps.rs`
- `src/app_init.rs`
- `src/lib.rs`

**Steps:**

1. Add `SkirmishScenarioRecord`, `SkirmishScenarioSource`, and `SkirmishScenarioKind`.
2. Include fields needed by current preview/load path:
   - `file_name`
   - `display_name`
   - `author`
   - `briefing`
   - `preview`
   - `multiplayer_start_waypoints`
   - `preview_source_bounds`
   - `game_modes`
   - `source_ordinal`
   - `kind`
3. Refactor current `read_map_menu_entry_from_ini` logic into reusable metadata helpers where practical.
4. Add fixture-driven builder tests for source-order behavior without needing retail assets.
5. Keep current `list_available_maps()` behavior unchanged for the legacy egui route.
6. Add a projection helper from concrete `SkirmishScenarioRecord` to `MapMenuEntry` for current preview/load consumers.

**Tests:**

- `scenario_records_preserve_source_order`
- `scenario_records_do_not_sort_by_display_name`
- `scenario_records_preserve_duplicate_display_names`
- `scenario_record_parses_game_modes`
- `scenario_record_projects_to_map_menu_entry`

**Checks:**

- `cargo test skirmish_scenarios --lib`
- `cargo test app_list_maps --lib`

### Task 3: Implement chooser filtering

**Why:** List membership must match retail before the modal is useful.

**Files:**

- `src/skirmish_scenarios.rs`
- `src/skirmish_modes.rs`

**Steps:**

1. Add:

   ```rust
   pub fn filter_records_for_mode(
       records: &[SkirmishScenarioRecord],
       mode: &SkirmishGameMode,
   ) -> Vec<usize>
   ```

2. Rules:
   - `RandomMapSentinel`: include only when `mode.random_maps_allowed`.
   - empty `game_modes`: include only when `mode.map_filter == "standard"`.
   - non-empty `game_modes`: include when any entry equals `mode.map_filter`, case-insensitive only if retail parsing proves/normalizes that way. Otherwise preserve exact normalized strings from parser.
3. Preserve input order by returning source-order indices.
4. Do not filter by display text or category name.

**Tests:**

- `choose_map_filters_by_selected_mpmode_game_modes`
- `choose_map_empty_game_modes_matches_standard_only`
- `choose_map_filter_preserves_source_order`
- `choose_map_filter_ignores_ui_label_and_category`
- `choose_map_filters_randmap_by_mode_random_allowed`

**Checks:**

- `cargo test choose_map_filter --lib`

### Task 4: Add modal layout `0x6B`

**Why:** The chooser is a separate dialog with its own controls, not setup dialog `0x102` controls.

**Files:**

- `src/ui/skirmish_shell/layout.rs`
- `src/ui/skirmish_shell/mod.rs`

**Steps:**

1. Add `ChooseMapModalLayout`.
2. Add rects for:
   - mode listbox `0x6EB`
   - map listbox `0x553`
   - Use Map `0x6C5`
   - Cancel `0x5C0`
   - Create Random Map `0x583`
   - title/static `0x694`
   - preview placeholder `0x468`
3. Use verified dialog `0x6B` pixel rects.
4. Apply the same screen/viewport origin policy as shell dialogs, but keep modal layout separate from setup layout.
5. Add hit-test helpers for modal listboxes and buttons.

**Tests:**

- `choose_map_modal_uses_resource_0x6b_control_rects`
- `choose_map_modal_hit_tests_use_map_cancel_random`
- `choose_map_modal_list_row_hit_test_maps_to_visible_row`

**Checks:**

- `cargo test choose_map_modal_layout --lib`

### Task 5: Add modal state and actions

**Why:** Highlighted chooser rows must not commit until Use Map; cancel must restore previous committed state.

**Files:**

- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/mod.rs`

**Steps:**

1. Add selected mode identity to `SkirmishShellState`, initially Battle id `1`.
2. Add selected scenario record identity beside or replacing `selected_map_idx` for the shell path.
3. Add `ChooseMapModalState`:
   - saved mode id
   - saved record ordinal
   - highlighted mode id
   - highlighted record ordinal
   - filtered record ordinals
4. Add modal actions:
   - open
   - cancel
   - use map
   - select mode row
   - select map row
   - create random map
5. Change `SkirmishShellAction::ChooseMap` so state-level `apply_action` does not cycle maps.
6. Keep old `SelectMap(usize)` only for legacy/tests if still needed.

**Tests:**

- `choose_map_action_does_not_cycle_selected_map`
- `choose_map_open_saves_committed_selection`
- `choose_map_highlight_does_not_commit`
- `choose_map_cancel_restores_previous_selection`
- `choose_map_accept_commits_mode_and_record`

**Checks:**

- `cargo test skirmish_shell --lib`

### Task 6: Wire app state and initialization

**Why:** App-level code owns available data, preview cache, and screen routing.

**Files:**

- `src/app.rs`
- `src/app_init.rs`
- `src/app_list_maps.rs`
- `src/skirmish_modes.rs`
- `src/skirmish_scenarios.rs`

**Steps:**

1. Add `skirmish_modes: Vec<SkirmishGameMode>` to `AppState`.
2. Add `skirmish_scenario_records: Vec<SkirmishScenarioRecord>` to `AppState`.
3. Add `choose_map_modal: Option<ChooseMapModalState>` to `AppState` or embed modal state in `SkirmishShellState`.
4. Initialize modes and records in `AppState::new()` or the existing startup map-list path.
5. If record building fails, log and fall back to an empty chooser list without crashing.
6. Keep `available_maps` for egui menu compatibility.
7. Ensure preview texture cache can be invalidated when selected scenario record changes.

**Tests:**

- State-construction tests if `AppState` setup is hard to unit test.
- Lower-level tests should cover most behavior; app smoke test is optional.

**Checks:**

- `cargo check`

### Task 7: Route Choose Map open/cancel/accept

**Why:** Current `handle_skirmish_shell_action` swallows `ChooseMap` after `apply_action`; it must open the modal instead.

**Files:**

- `src/app.rs`
- `src/ui/skirmish_shell/state.rs`

**Steps:**

1. In `handle_skirmish_shell_action`, treat `SkirmishShellAction::ChooseMap` as app-level modal open.
2. Do not call state `apply_action` in a way that loses the action before app routing.
3. On modal cancel, restore saved selection and invalidate preview for restored record.
4. On modal accept:
   - commit mode/record;
   - rebuild selected-map capacity dependent row/start choices;
   - invalidate preview cache;
   - return to setup shell.
5. On accepted load/projection failure, restore saved mode/record and skip normal preview refresh for the failed selection.

**Tests:**

- `choose_map_app_open_sets_modal_state`
- `choose_map_app_cancel_restores_and_closes_modal`
- `choose_map_app_accept_commits_and_invalidates_preview`
- `choose_map_app_accept_failure_restores_saved_selection`

**Checks:**

- `cargo test choose_map --lib`
- `cargo check`

### Task 8: Render Choose Map modal

**Why:** The modal must be visible and usable before interaction trace checks are meaningful.

**Files:**

- `src/app_skirmish_shell_render.rs`
- `src/render/skirmish_shell_chrome.rs`
- `src/ui/skirmish_shell/layout.rs`

**Steps:**

1. Load `MnScrnLCustomizeBattle.shp/.PAL` into the shell chrome atlas or a chooser-specific atlas field.
2. Add render path branch: if Choose Map modal is open, draw modal instead of setup shell.
3. Draw background.
4. Draw mode listbox rows from `skirmish_modes`.
5. Draw map listbox rows from filtered scenario records.
6. Draw selected highlight for both listboxes.
7. Draw Use Map, Cancel, and Create Random Map buttons.
8. Draw title/static text and preview placeholder.
9. Keep exact owner-drawn listbox row paint marked as a known gap.

**Tests:**

- Helper-level tests for row text clipping/selection indexes where practical.
- Rendering is mostly manual/screenshot verified.

**Checks:**

- `cargo check`
- Manual run with `RA2_DEV_SKIRMISH_SHELL=1`.

### Task 9: Implement modal input

**Why:** The modal must own mouse handling while open; setup controls should not receive clicks through it.

**Files:**

- `src/app.rs`
- `src/ui/skirmish_shell/state.rs`
- `src/ui/skirmish_shell/layout.rs`

**Steps:**

1. If modal is open, route mouse down/up to modal hit testing before setup shell hit testing.
2. Mode listbox click updates highlighted mode and rebuilds filtered map rows.
3. Map listbox click updates highlighted record.
4. Use Map button triggers accept.
5. Cancel button triggers cancel.
6. Create Random Map button creates or updates the `RandMap.Sed` sentinel when enough fields are implemented; otherwise keep the button disabled or log a clear not-yet-implemented message.

**Tests:**

- `choose_map_modal_mode_click_rebuilds_filtered_rows`
- `choose_map_modal_map_click_updates_highlight_only`
- `choose_map_modal_blocks_setup_button_clicks`
- `choose_map_modal_use_map_requires_highlighted_record`

**Checks:**

- `cargo test choose_map --lib`
- Manual click-through check.

### Task 10: Add random-map sentinel support

**Why:** The random map button and sentinel are part of the retail chooser contract, but random terrain generation can remain a later gate.

**Files:**

- `src/skirmish_scenarios.rs`
- `src/ui/skirmish_shell/state.rs`
- `src/app.rs`
- `src/app_skirmish_shell_render.rs`
- `src/skirmish_launch.rs`

**Steps:**

1. Add `SkirmishScenarioKind::RandomMapSentinel`.
2. Add helper to create/update one `RandMap.Sed` record.
3. Insert or update the sentinel without treating it as a loose-map scan result.
4. Filter sentinel by `mode.random_maps_allowed`.
5. Accept sentinel as an ordinary record identity.
6. Use `RandMap.img` preview when sentinel is selected if the asset is available.
7. Preserve `.sed` token in launch session, but clearly return a not-implemented launch error until random map generation is implemented.

**Tests:**

- `skirmish_random_map_command_adds_or_updates_single_sentinel_record`
- `skirmish_choose_map_filters_randmap_by_mode_random_allowed`
- `skirmish_randmap_accept_commits_ordinary_record_index`
- `skirmish_randmap_launch_preserves_sed_token`

**Checks:**

- `cargo test randmap --lib`

### Task 11: Carry selected mode into launch session

**Why:** The shell currently hardcodes `SkirmishLaunchMode::Battle`; chooser accept changes selected MPModes.

**Files:**

- `src/skirmish_launch.rs`
- `src/ui/skirmish_shell/state.rs`
- `src/app_skirmish.rs`

**Steps:**

1. Add selected mode id or a richer launch mode enum to `SkirmishLaunchSession`.
2. Keep Battle behavior as the only fully implemented startup mode if other callbacks remain incomplete.
3. For non-Battle modes, either:
   - start with known shared behavior and log missing mode-specific callbacks; or
   - reject start with a typed validation error until implemented.
4. Do not silently launch Team Game/Duel/etc. as Battle while displaying another selected mode.
5. Ensure selected map token comes from committed scenario record, not old `available_maps[selected_map_idx]`.

**Tests:**

- `launch_session_packs_selected_mode_id`
- `launch_session_uses_committed_scenario_record_file`
- `launch_session_does_not_silently_treat_teamgame_as_battle`

**Checks:**

- `cargo test skirmish_launch --lib`

### Task 12: Manual visual and behavior verification

**Why:** This is a player-visible shell workflow; unit tests cannot prove pixel placement or interaction feel.

**Steps:**

1. Run with `RA2_DEV_SKIRMISH_SHELL=1`.
2. At `800x600`, open Skirmish shell and click Choose Map.
3. Verify setup shell hides/replaces with modal.
4. Verify modal background uses `MnScrnLCustomizeBattle`.
5. Verify mode and map listboxes appear at expected locations.
6. Verify Cancel restores prior setup selection.
7. Verify Accept changes map label/preview and row capacity immediately.
8. Verify Team Game map filtering differs from Battle.
9. Verify Random Map sentinel appears only in Battle/Free For All after Create Random Map path is available.

**Checks:**

- Save screenshots for `800x600`.
- Optional later screenshot for `1024x768` because high-res modal behavior remains a research gate.

---

## Suggested Patch Batches

### Batch A: Data Foundations

Tasks 1-3.

Deliverable: mode model, source-ordered record model, and filter tests. No UI changes except new data modules.

### Batch B: Modal State And Routing

Tasks 4-7.

Deliverable: clicking Choose Map opens/closes a modal state and no longer cycles maps. The modal may be minimally rendered or debug-rendered.

### Batch C: Modal Rendering And Input

Tasks 8-9.

Deliverable: visible modal with listboxes, buttons, row highlighting, and accept/cancel interactions.

### Batch D: Random Sentinel And Launch Contract

Tasks 10-11.

Deliverable: `RandMap.Sed` sentinel model and selected mode/map token carried into launch session without pretending random terrain generation is done.

### Batch E: Manual Verification

Task 12.

Deliverable: screenshot/manual parity notes and a remaining-gap list for trace-action after implementation.

## Stop Conditions

- Retail source-order builder cannot read `MISSIONSMD.PKT` or loose PKT data with current asset APIs: stop after fixture-backed model and decide whether to extend asset loading first.
- Modal accept would require silently launching non-Battle modes as Battle: stop and add typed validation instead.
- Random map generation is required to start a `.sed` token: preserve the token but block launch with a clear not-implemented error.
- Rendering `MnScrnLCustomizeBattle` is unavailable: keep state/input implementation, but mark visual verification blocked on asset loading.
- Any implementation attempts to add UI dependencies to `sim/`: stop and move that data to app/UI boundary.

## Do Not Do

- Do not keep `ChooseMap` as an in-place next-map button.
- Do not sort chooser records by display name.
- Do not collapse duplicate-looking records.
- Do not drive the chooser from legacy `available_maps`.
- Do not filter by `GUI:*` labels or MPModes category section names.
- Do not treat empty `GameModes` as match-all.
- Do not treat `0x6EB` as a dropdown combo.
- Do not use setup dialog `0x102` layout for modal `0x6B`.
- Do not use `MnScrnLCoopGameSetup.*` as the Choose Map modal background.
- Do not add `RandMap.Sed` as a permanent loose map.
- Do not encode random map as `None`, `-1`, or a special negative index.
- Do not expose stock Siege in offline Skirmish.

## Final Acceptance Criteria

- `ChooseMap0x5aa` opens a modal rather than cycling maps.
- Modal uses `0x6B` layout and `MnScrnLCustomizeBattle` background when assets are available.
- Mode list comes from MPModes and stores/selects by mode id.
- Map list preserves retail source order and filters by selected mode `map_filter`.
- Cancel restores prior selection.
- Accept commits selected mode/map and rebuilds row capacity before preview refresh.
- Legacy egui map selector remains functional.
- Unit tests cover MPModes parsing, scenario record ordering/filtering, modal state transitions, random sentinel filtering, and launch-session selected mode/map token packing.
