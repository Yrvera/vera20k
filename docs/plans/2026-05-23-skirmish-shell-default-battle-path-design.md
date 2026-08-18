# Skirmish Shell Default Battle Path Design

## Goal

Make the native offline Skirmish shell the default player path for stock Battle-style games through the first playable frame, without hiding known parity gaps behind the UI flip.

## Architecture Context

The current code already has most of the native shell split along the right boundaries:

- `src/ui/skirmish_shell/layout.rs` owns verified shell and Choose Map geometry.
- `src/ui/skirmish_shell/state.rs` owns shell state, hit testing, combo/dropdown behavior, option mutation, Start validation, and `SkirmishLaunchSession` packing.
- `src/app_skirmish_shell_render.rs` renders the native shell chrome, controls, preview, marker overlays, dropdowns, and Choose Map modal.
- `src/skirmish_launch.rs` defines the app-level launch contract, intentionally outside `sim/` so simulation does not depend on UI or render modules.
- `src/app.rs` currently gates the native shell behind `dev_skirmish_shell_enabled`; default Main Menu -> Single Player still routes into the egui setup.
- `src/app_init.rs` passes an optional `SkirmishLaunchSession` into map loading.
- `src/app_skirmish.rs` applies a launch session to create launch houses, assign starts, and spawn MCVs.

The important architectural constraint holds: `sim/` must not depend on `ui/`, `render/`, `audio/`, or app shell code. The design keeps shell state and launch packing above `sim/`; the lower gameplay-facing result is ordinary deterministic setup data consumed during map initialization.

Recent Rust has already moved past the old "two MCV" shortcut for the native shell path. When `pending_skirmish_launch_session` is present, `app_init::load_map` calls `apply_skirmish_launch_session`. The older `seed_skirmish_opening_if_needed` shortcut still remains on the default egui path and should not be the parity route.

Primary evidence:

- `docs/research/SKIRMISH_UI_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/SKIRMISH_SHELL_UI_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/skirmish-ui/SKIRMISH_SHELL_LAYOUT_POSITIONING_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/skirmish-ui/SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md`

## Impact Analysis

This design touches the default player path but preserves the existing module boundaries.

Touched modules:

- `src/app.rs`: route Main Menu -> offline Skirmish to the native shell instead of the egui setup, while retaining an emergency fallback or debug toggle until parity gates pass.
- `src/ui/skirmish_shell/state.rs`: add missing shell state for player name, modal validation errors, and richer launch packing where necessary.
- `src/ui/skirmish_shell/layout.rs`: keep existing verified rects; only extend if missing modal/edit fields require named rects.
- `src/app_skirmish_shell_render.rs`: finish player-name edit rendering, validation modal rendering, and any remaining shell text/control parity needed for the default path.
- `src/skirmish_launch.rs`: expand the launch contract to carry native-shaped local/AI rows, player name, mode id, selected map token/index mirrors where needed, option mirrors, random-resolution results, and forced launch flags.
- `src/app_init.rs`: keep the optional launch-session handoff, but ensure the session path is the only default Skirmish path.
- `src/app_skirmish.rs`: replace direct spawn-per-slot semantics with a staged Battle-style launch setup: create houses from launch slots, assign starts, set base centers, run standard MCV/base callback, then run `UnitCount` extra-unit budget generation.
- `src/sim/game_options.rs` and related game-option consumers: preserve already-modeled options and add only the deterministic game-state option effects needed by the verified Start branch.

Risk areas:

- Default routing is high visibility. A partial flip creates immediate player-facing drift.
- Start-unit generation touches entity creation, owner mapping, start placement, and deterministic RNG use.
- House creation and alliances are foundational; wrong owner/color/team state will leak into sidebar, shroud, AI, diplomacy, and victory checks.
- Validation failure UI must block Start and leave the shell alive; logging is not enough.
- Settings persistence through `RA2MD.INI` remains unresolved enough that it should not be silently implemented from inference.

## Chosen Approach

Build the complete stock Battle path first:

```text
Main Menu
  -> native offline Skirmish shell 0x102
  -> default Battle/ManBattle-style Start Game validation
  -> native-shaped launch session packing
  -> selected map load
  -> launch houses from session slots
  -> explicit/auto start assignment
  -> standard MCV/base callback
  -> standard UnitCount extra-unit budget
  -> first playable frame
```

This is preferred over simply flipping the native shell default because the shell and launch path are player-observable as one flow. A correct-looking setup screen that starts the wrong houses, wrong MCV types, wrong starting units, or wrong validation feedback is still a parity failure.

The scope is intentionally Battle-style stock offline Skirmish. Non-Battle custom mode callbacks, full `MPModesMD.ini` override payload application, and RA2MD.INI persistence are called out as explicit follow-ups unless they block the default Battle path.

## Tiny-Detail Ledger

- Offline Skirmish creates dialog `0x102`; Start return is `0x617`, Back is `0x5C0`. Start writes success only after validation, selected-mode acceptance, packing, preview teardown, and result-pointer write. Source: `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`.
- The shell remains modal on validation failures; Start is re-enabled and the result pointer is not written. Source: `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`.
- Child controls are positioned from dialog/resource rectangles and resize helpers, not proportional scaling. One-pixel fixups include `0x50C y-1`, option checkbox `x-1`, and player-name `x+1,w+1`. Source: `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`.
- Right panel draw order is top cap, repeated tile, optional `SDBTNANM` frame 10, bottom cap, lower strip. `SDBTM` bottom cap is source-clipped, not destination-scaled. Source: `SKIRMISH_SHELL_LAYOUT_POSITIONING_SYSTEM_MODEL_SYNTHESIS.md`.
- Fresh `>800` Skirmish does not draw the 800 parent background SHP; exact aggregate high-res screenshot parity remains a verification task. Source: `SKIRMISH_UI_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`, `SKIRMISH_GT800_BACKGROUND_TARGETED_TRACE_RECONCILIATION.md`.
- PreviewPack selected-map pixels are row-major RGB triples, not BGR. Source: `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`.
- Preview aspect fit uses integer per-mille truncation and half-scaled centering. Source: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md`.
- Live `STARTBUT.SHP` markers come from `[Header]` preview metadata, not gameplay `[Waypoints]`; marker draw is `(anchor_x-9, anchor_y-6)` and label draw is `(anchor_x-2, anchor_y-6)`. Source: `SKIRMISH_START_MARKER_CLIPPING_FOOTPRINT_GHIDRA_REPORT.md`, `SKIRMISH_UI_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`.
- Marker and numeric-label clipping is the destination surface/backbuffer clip, not the fitted preview rect. Source: `SKIRMISH_START_MARKER_CLIPPING_FOOTPRINT_GHIDRA_REPORT.md`.
- Combo dropdowns are custom `ComboDropWin` popups, not real listboxes; Choose Map lists are real owner-drawn listboxes and must remain separate. Source: `SKIRMISH_COMBODROPWIN_0060D540_FUNCTION_BOUNDARY_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`.
- AI row item data is `None=-1`, `Easy=2`, `Normal=1`, `Hard=0`; do not reuse naive difficulty enum order as native row data. Source: `SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md`.
- Color dropdown normal population is sentinel `-2`, then colors `0..7`; row 8 is not present. Source: `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`.
- Player-name control `0x6A0` is an ordinary `Edit`, final rect `(58,59,151,23)`, seeded from `DAT_00A8B380`, capped with `EM_SETLIMITTEXT 0x13`, read back through `0x4B3` into a 20-wide-char buffer, and committed to `DAT_00A8B380` on Start. Source: `SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`.
- Player-name edit paints an edit frame, left+2 text inset, yellow text from `DAT_00AC18A4`, and a two-pixel caret when focused and not selecting. Source: `SKIRMISH_PLAYER_NAME_EDIT_CONTROL_0X6A0_GHIDRA_REPORT.md`.
- Capacity failure uses `TXT_SCENARIO_TOO_SMALL` formatted with map capacity and `TXT_OK`; minimum-player failure uses `TXT_NEED_AT_LEAST_TWO_PLAYERS`; same-team failure uses `TXT_CANNOT_ALLY`. Source: `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`.
- Stock Battle/ManBattle selected-mode `+0x14` accepts unconditionally; FreeForAll and Cooperative accept with side effects; stock local Unholy false branch does not write output `0x617`; Siege is binary-supported but not stock local roster. Source: `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_MODAL_TEXT_AND_MODE_REJECTION_GHIDRA_REPORT.md`.
- Start branch packs local node data, active AI count, AI row arrays, compact launch table, random assignments, trackbars, checkboxes, and forced flags before exit. Source: `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`.
- Random country/color resolution happens before scenario house creation consumes node/AI data. Source: `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`.
- Explicit start preassignment uses `House+0x16058` and `ScenarioClass+0x1180`, not `House+0x1605C`. `House+0x1605C` is team/adjunct. Source: `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`.
- `Create_Houses` creates human and AI houses from node records and AI arrays, applies credits/color/difficulty/start/team, and sets the local player pointer. Source: `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`.
- Deficient starts call `Gather_Start_Positions` fallback with `FootClass__Find_Nearby_Passable_Cell` and 8x8 dimensions; no-spawn is wrong. Source: `SKIRMISH_GATHER_START_POSITIONS_DEFICIENT_WAYPOINT_FALLBACK_00688380_GHIDRA_REPORT.md`.
- `UnitCount` drives a rounded average-cost budget over eligible unit/infantry types; it is not a literal count. `UnitCount=0` still permits MCV creation when `Bases=yes`. Source: `SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md`.
- Standard MCV type comes from `[General] BaseUnit` plus side/house masks through `FUN_00505310`, not hardcoded country candidate order. Source: `SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md`.
- Extra-unit candidates require `AllowedToStartInMultiplayer`, `TechLevel <= house tech`, and house-mask intersection; BaseUnit entries are excluded from one candidate list. Source: `SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md`.
- Standard placement tries direct `Place`, then shared fallback `FUN_00688ED0` with radius 1 for MCVs and radius 4 for extra units. Source: `SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md`.
- `[MultiplayerDialogSettings]` defaults are YR `*md` values over base RA2 fallback; `GameSpeed=1`, `Money=10000`, `UnitCount=10`, `Crates=yes`, `ShortGame=yes`, `MCVRedeploys=yes`, `FogOfWar=no` are active defaults. Source: `SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`, `DEFAULT_SKIRMISH_FRAME_PACE_EXTENSION_GHIDRA_REPORT.md`, `ini/rulesmd.ini`.
- Exact RA2MD.INI persistence for all shell settings remains `UNKNOWN - needs RE/implementation handoff`; do not invent it. Source: `SKIRMISH_UI_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`.

## Design

### Components

#### Default Shell Route

Replace the default Main Menu -> Single Player path with a native Skirmish-shell entry when the user chooses the offline Skirmish route. Keep the existing egui setup as an explicit debug fallback until the native path passes its acceptance checks.

The app should expose a state distinction between:

- Main menu shell visible.
- Native Skirmish shell visible.
- Native Skirmish shell modal visible.
- Loading a `SkirmishLaunchSession`.
- In-game.

This should replace the current boolean split where `dev_skirmish_shell_enabled` doubles as both a debug toggle and a screen route.

#### Shell State

Extend `SkirmishShellState` with:

- `player_name: String`, capped at the native 19-character user-visible limit.
- edit focus/caret state for `0x6A0`.
- validation modal state with body text, OK text, and blocking behavior.
- packed selected-mode identity sufficient for stock Battle/ManBattle path.
- explicit Start disabled/re-enabled state if needed to match press/validation lifecycle.

Keep `SkirmishShellState` UI-only. It may build a launch-session data object, but it must not mutate `Simulation`.

#### Shell Render

Extend the native renderer with:

- player-name edit frame/text/caret path, not a static `"Player"` label.
- validation modal rendering using CSF-localized strings.
- any remaining right-panel static text, bottom-cap clipping, or text caller-rect fixes that block a correct default first screen.

Do not use egui widgets for the parity shell.

#### Launch Session

Expand `SkirmishLaunchSession` conservatively:

- selected map file/token and selected mode id/type.
- local player slot: name, country/random-resolved country, color/random-resolved color, start, team.
- AI slots: row kind/item data, country/random-resolved country, color/random-resolved color, start, team, difficulty.
- options: credits, game speed, unit count, short game, bases, bridge destruction, super weapons, build off ally, crates, MCV redeploy, fog/shroud/tiberium options as currently modeled and verified.
- forced flags that have known first consumers should be represented as named fields or documented constants on the launch setup path.

The session should carry resolved random outcomes at the point Rust exits the shell. That matches gamemd.exe, where random assignment resolution happens before `Create_Houses`.

#### Scenario Start Setup

Split current `apply_skirmish_launch_session` into explicit stages:

1. Build launch slots in deterministic local-then-AI order.
2. Populate non-player houses from map roster.
3. Populate player houses from launch slots, not playable map-roster order.
4. Apply credits, color, side/country, human/AI flag, difficulty, start field, and team field.
5. Build alliance map from teams and mode constraints.
6. Gather starts from map waypoints, with fallback start generation when deficient.
7. Build a start assignment table analogous to `ScenarioClass+0x1180`.
8. Assign base centers in native human-first/AI-second order for Battle-style starts.
9. Run standard MCV/base callback for every active non-special, non-observer house.
10. Run standard extra-unit budget generation from `UnitCount`.

This can live in `app_skirmish.rs` initially, but if it grows, split it into `app_skirmish/start_setup.rs` or similar app-level modules. Do not put UI concepts into `sim/`.

### Interfaces / Contracts

`SkirmishShellState::launch_session(...) -> Result<SkirmishLaunchSession, LaunchValidationError>` remains the shell-to-app boundary, but the error enum should map directly to modal text:

- `MapCapacityExceeded { capacity, requested_players }` -> `TXT_SCENARIO_TOO_SMALL`.
- `NoEnabledOpponent` -> `TXT_NEED_AT_LEAST_TWO_PLAYERS`.
- `SameExplicitTeam { team }` -> `TXT_CANNOT_ALLY`.
- selected-mode rejection remains separate and should only use the generic `0x469` OK branch when the verified native condition is met.

`SkirmishLaunchSession` is app-level input to map initialization. It should be serializable/debug-printable enough for tests, but it is not a save-game format.

`apply_skirmish_launch_session(...) -> SkirmishLaunchApplyResult` should keep returning player-visible setup outcomes, but the result should distinguish:

- houses created.
- assigned starts.
- MCVs spawned.
- extra units spawned.
- fallback starts used.
- unsupported/deferred mode behavior hit.

### Data Flow

Default flow:

```text
MainMenuShellAction::SinglePlayer
  -> app enters NativeSkirmishShell screen/state
  -> SkirmishShellState mutates from user input
  -> Start validates and either opens modal or packs SkirmishLaunchSession
  -> app stores pending_skirmish_launch_session and enters Loading
  -> load_map reads selected map by native token/file priority
  -> app_skirmish applies launch session during initialization
  -> Simulation starts with launch-created houses, starts, MCVs, and UnitCount extras
```

No `sim/` code should know about shell buttons, control ids, CSF labels, or modal behavior.

### Error Handling

Use typed validation errors for known Start failures. Convert them to localized shell modal text at the app/UI layer.

Use logs for internal unexpected failures only, not as the player-visible response to Start validation.

If a selected non-Battle mode reaches an unimplemented mode-specific callback, block Start with an explicit shell-visible unsupported-mode message during development rather than launching a known-drifting game. The default route should select Battle, so this is not part of the normal acceptance path.

### Testing Strategy

Shell/layout tests:

- `skirmish_default_route_enters_native_shell`
- `skirmish_player_name_edit_commits_19_char_limit`
- `skirmish_player_name_edit_uses_binary_frame_inset_and_caret_rect`
- `skirmish_start_capacity_modal_uses_retail_csf_text`
- `skirmish_start_no_opponent_modal_uses_retail_csf_text`
- `skirmish_start_same_team_modal_uses_cannot_ally_text`
- `skirmish_right_panel_bottom_cap_uses_source_clip`
- `skirmish_high_res_default_shell_keeps_no_parent_background_above_800`

Launch/session tests:

- `skirmish_start_packs_all_enabled_rows_into_launch_session`
- `skirmish_random_country_and_color_are_resolved_before_house_creation`
- `skirmish_ai_row_item_data_preserves_native_difficulty_order`
- `skirmish_launch_options_preserve_trackbars_and_checkboxes`
- `skirmish_mode_battle_accepts_without_generic_mode_modal`

Scenario/start tests:

- `skirmish_create_houses_uses_launch_slots_not_map_roster`
- `skirmish_explicit_start_table_assigns_human_then_ai`
- `skirmish_deficient_start_pool_uses_fallback_instead_of_no_spawn`
- `skirmish_baseunit_vector_selects_side_matching_mcv`
- `skirmish_unit_count_zero_spawns_mcv_only_when_bases_enabled`
- `skirmish_start_unit_budget_filters_spawnable_tech_and_house_mask`
- `skirmish_start_unit_uses_spiral_fallback_when_start_cell_blocked`

Verification:

- Run focused Rust unit tests for layout/state/session/start setup.
- Run `cargo test` for touched modules.
- Capture 640x480, 800x600, and 1024x768 screenshots of the shell once implemented.
- Use deterministic test maps for 2-player, 4-player, explicit starts, deficient starts, blocked starts, and UnitCount 0/10.

## Architectural Decisions

- Keep native Skirmish shell as real app state, not an egui panel or dev overlay.
- Keep `SkirmishLaunchSession` app-level and data-only; it is the bridge between UI and initialization.
- Keep Battle-style mode as the first default path because its Start acceptance and standard post-map callbacks are verified.
- Do not implement RA2MD.INI persistence from guesswork. Add a focused investigation or implementation contract first.
- Do not broaden to all MPModes before the default Battle route is correct. Cooperative, Siege, Unholy, and full override payload behavior are real, but not prerequisites for default Battle first-playable-frame parity.
- Do not keep hardcoded MCV country candidates in the parity start generator; use parsed `BaseUnit` plus side/house masks.

Tech debt accepted:

- The egui setup can remain as a debug fallback during migration. It should not remain the default parity route after this design is implemented.
- Some selected-mode fields may initially support only Battle/ManBattle. This is acceptable only if unsupported modes are not silently launched through the default path.

## Alternatives Considered

### Flip the Native Shell First

This would make the current dev-gated shell the default immediately and keep existing launch behavior. It is too risky for parity because the player would see a native shell but still get non-native house/start/unit outcomes and incomplete validation/modals.

### Full MPModes And Persistence Before Default

This would block default Battle on all local mode callbacks, override payload application, and full RA2MD.INI persistence. It is technically cleaner in the long run but delays the verified default Battle path on surfaces not needed for the normal first-playable-frame route.

### Continue Improving The Egui Setup

The egui setup is useful for debugging, but it cannot be the parity path. The research target is the retail shell with owner-draw controls, native layout, modal validation, and Start packing semantics.

