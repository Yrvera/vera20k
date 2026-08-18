# Skirmish Launch Session Design

## Goal

Replace the current narrow skirmish launch shortcut with a native-shaped launch-session contract that preserves player rows, options, map selection, and Battle-mode startup inputs before scenario initialization consumes them.

## Architecture Context

Current Rust launches skirmish through a simplified path:

```text
SkirmishShellState
  -> SkirmishSettings
  -> app start_selected_skirmish / Loading
  -> app_init::load_map
  -> app_skirmish::seed_skirmish_opening_if_needed
```

The important current surfaces are:

- `src/ui/skirmish_shell/state.rs`: experimental native-like shell state, including local row data and opponent rows.
- `src/ui/main_menu.rs`: `SkirmishSettings`, the older narrow launch object used by both shell routes.
- `src/app.rs` and `src/app_transitions.rs`: app-level routing from menu/shell into map loading.
- `src/app_init.rs`: map/rules/art/terrain loading, map object spawning, map house roster parsing, initial AI setup.
- `src/app_skirmish.rs`: current skirmish startup helper, which selects multiplayer waypoints, reorders map-roster houses, and spawns at most two MCVs.
- `src/sim/game_options.rs`: deterministic game option state already hashed by `world_hash`, but currently not fed from the full shell launch contract.

The core mismatch is that `SkirmishShellState::launch_settings` collapses the native shell rows too early. Only one AI country survives, AI color/team/start/difficulty are lost, `UnitCount` and many options are not represented, and `seed_skirmish_opening_if_needed` caps startup at two houses with direct MCV placement.

The intended boundary is:

```text
ui/skirmish_shell
  produces SkirmishLaunchSession

app/app_init/app_skirmish
  validates and translates session data into scenario initialization inputs

sim
  receives deterministic houses, options, entities, alliances, and AI flags
```

`sim/` must not depend on `ui/`, `sidebar/`, `render/`, `audio/`, or shell dialog details. Owner-draw button IDs, preview widgets, and modal dialog lifecycle stay above the simulation boundary.

## Impact Analysis

This design changes the skirmish startup contract. It should be implemented as a staged migration so the old main-menu shortcut can either construct a minimal launch session or remain as a compatibility path until the shell route owns skirmish starts.

Touched modules:

- `src/ui/skirmish_shell/state.rs`: replace `launch_settings` output for the native shell route with `SkirmishLaunchSession`.
- `src/ui/main_menu.rs`: keep `SkirmishSettings` only as legacy UI state or migrate it to produce a session.
- `src/app.rs`: store/pass launch sessions through `GameScreen::Loading` or equivalent app transition state.
- `src/app_transitions.rs`: carry session data into `app_init::load_map`.
- `src/app_init.rs`: consume launch session when creating skirmish houses and initial game options.
- `src/app_skirmish.rs`: replace two-player seeding shortcut with session-driven house/start/start-unit helpers.
- `src/sim/game_options.rs`: set launch-time options from the session, including the verified `BuildOffAlly` default.
- `src/sim/house_state.rs`: may need fields or constructor inputs for color, start slot, team, difficulty, local/AI identity, and startup base metadata.

Future consumer work:

- `src/rules/object_type.rs`: parse typo-preserved `EligibileForAllyBuilding`.
- `src/sim/production/production_placement.rs`: consume `GameOptions::build_off_ally` for tactical building placement.

Risk areas:

- Deterministic house ordering and stable owner names/IDs.
- Local player owner selection and `g_PlayerPtr` equivalent behavior.
- AI setup currently based on map house roster rather than enabled shell rows.
- `base_center` and `waypoint_edge` are consumed by paradrop logic and must remain deterministic after native start assignment.
- Map `[Houses]` still matters for map-authored houses, neutral/special data, and map content; it must not remain the source of skirmish player slots.
- `world_hash` already includes `GameOptions`; launch-option changes can affect deterministic replay/state hashing.

## Chosen Approach

Use a native-shaped `SkirmishLaunchSession` as the stable app boundary.

The session records the player-visible launch contract from the shell before any map/scenario initialization starts. `app_init` and `app_skirmish` then translate that session into deterministic game options, runtime houses, start assignments, and Battle-mode startup entities.

This approach is preferred over expanding `SkirmishSettings` in place because `SkirmishSettings` already represents an approximate client menu contract. Keeping it as the core handoff would preserve the wrong mental model: "selected map plus first AI" instead of "packed skirmish session consumed by scenario init."

This approach is also preferred over blocking on full non-Battle reverse engineering. The launch contract shape is already verified. The unresolved selected-map loader and mode-specific `+0x84` body affect later consumers and are tracked as research gates rather than guessed.

## Tiny-Detail Ledger

- Start Game is not a spawn command. It packs session/node/options and exits the modal; spawn consumers begin in `ScenarioClass::Full_Init` and post-map init. Source: `SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`.
- The native shell packs full local/AI row data before launch: map token/index mirrors, local node record, seven AI row arrays, active AI count, random country/color resolution, trackbars, checkboxes, and forced launch flags. Source: `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, cited by the start-to-init trace.
- Runtime skirmish houses are created from packed session/node/AI state, not from map house roster ordering. Source: `ScenarioClass__Create_Houses @ 0x00687F10`, cited by `SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`.
- Explicit start preassignment uses the house start field (`House+0x16058`) and a scenario start table. `House+0x1605C` is team/adjunct, not start. Source: `SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`.
- Startup must not cap at two players. Native loops human count plus enabled AI count. Source: `ScenarioClass__Create_Houses @ 0x00687F10`; Rust mismatch at `app_skirmish.rs` current `take(2)`.
- Deficient waypoint handling is not "no spawn"; native gathers waypoints and can generate fallback passable starts with 8x8 clearance. Source: `ScenarioClass__Gather_Start_Positions @ 0x00688380`, cited by the trace.
- Start-unit/MCV generation is post-map behavior driven by selected mode callbacks and `UnitCount`/budget data. Source: `ScenarioClass__Post_Map_Init @ 0x00686890`, `FUN_005D6D80 @ 0x005D6D80`, cited by the trace.
- `BuildOffAlly` defaults enabled in standard YR unless rules or persisted settings override it. Source: `SKIRMISH_BUILDOFFALLY_FIRST_CONSUMER_GHIDRA_REPORT.md`.
- `BuildOffAlly` affects tactical building placement by allowing allied eligible provider buildings; it does not affect startup house creation or spawn placement. Source: `SKIRMISH_BUILDOFFALLY_FIRST_CONSUMER_GHIDRA_REPORT.md`.
- Allied placement providers require typo-preserved `EligibileForAllyBuilding=yes`, separate from `BaseNormal`. Source: `SKIRMISH_BUILDOFFALLY_FIRST_CONSUMER_GHIDRA_REPORT.md`.
- `[PreviewPack]` selected-map preview pixels are row-major RGB; preview decode/render is separate from launch plumbing. Source: `SKIRMISH_PREVIEWPACK_CHANNEL_ORDER_AND_MENU_CALLER_GHIDRA_REPORT.md`.
- Unknown: exact selected map filename/token loader between shell modal success and scenario file load. Source: `SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md` remaining uncertainty.
- Unknown: exact standard Battle `+0x84` MCV/start-unit callback body at `0x005D6C70`; `Post_Map_Init -> +0x84 -> FUN_005D6D80` is verified, but callback internals are not fully decoded. Source: `SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md` remaining uncertainty.

## Design

### Components

#### `SkirmishLaunchSession`

Add a launch-session data type outside `sim/`, preferably near app-level or shared UI/app handoff code. Candidate locations:

- `src/skirmish_launch.rs`
- `src/app_skirmish/session.rs`

The type should be plain deterministic data, with no egui, shell-control, preview, or rendering types.

Conceptual shape:

```rust
pub struct SkirmishLaunchSession {
    pub selected_map_idx: usize,
    pub selected_map_file: Option<String>,
    pub mode: SkirmishMode,
    pub local: SkirmishPlayerSlot,
    pub opponents: Vec<SkirmishAiSlot>,
    pub options: SkirmishLaunchOptions,
}
```

`selected_map_idx` preserves current Rust menu behavior. `selected_map_file` or a later map token field should become the scenario-load identity once the native token loader is traced. Until that RE is complete, the implementation can continue resolving through `available_maps[selected_map_idx]`, but the design should leave a named field for the native loader fact.

#### Player Slots

Player rows should preserve all shell-visible values before any app-level simplification:

```rust
pub struct SkirmishPlayerSlot {
    pub country: SkirmishCountry,
    pub color_index: u8,
    pub start_position: LaunchStartPosition,
    pub team: LaunchTeam,
}

pub struct SkirmishAiSlot {
    pub enabled: bool,
    pub country: SkirmishCountry,
    pub color_index: u8,
    pub start_position: LaunchStartPosition,
    pub team: LaunchTeam,
    pub difficulty: AiDifficulty,
}
```

Only enabled AI rows are consumed by scenario init, but preserving disabled rows in the UI state is fine. The launch session can either store only enabled opponents or store all rows with `enabled`; the consumer must use the native active AI count semantics.

`LaunchStartPosition` should distinguish Auto from explicit slot:

```rust
pub enum LaunchStartPosition {
    Auto,
    Waypoint(u8),
}
```

This replaces the current local-only start swap. Explicit starts for local and AI slots must flow into the start-assignment stage.

#### Launch Options

`SkirmishLaunchOptions` should be the shell/session representation. `GameOptions` remains the simulation representation.

Required fields for this stage:

- `starting_credits`
- `unit_count`
- `game_speed`
- `short_game`
- `super_weapons`
- `build_off_ally`
- `mcv_redeploy`
- `crates`
- `bases`
- `fog_of_war`
- `shroud`
- `bridges_destroyable`
- `ally_change_allowed`
- `multi_engineer`
- `harvester_truce`
- `tech_level`

Not every field needs a live shell widget in the first patch, but every field that already has a known native default or existing Rust `GameOptions` slot should have a clear defaulting path. `BuildOffAlly` must default to true for standard YR unless rules/session persistence overrides it.

#### Scenario Init Consumer

Introduce a session-driven setup path in `app_skirmish.rs`:

```text
apply_skirmish_launch_session
  -> build_game_options
  -> create_skirmish_houses
  -> build_start_assignment_table
  -> assign_starting_points
  -> generate_battle_startup_units
```

The first implementation should replace the player-visible shortcut pieces without forcing exact non-Battle callback behavior:

- Create all enabled player houses from launch slots.
- Preserve selected country, color, team, difficulty, and start position.
- Assign starts for all enabled player houses.
- Spawn starting MCVs/units for all enabled non-special houses through a Battle-mode helper.
- Preserve `base_center` and `waypoint_edge` from assigned starts.

Map roster parsing remains in `app_init.rs`, but for skirmish player houses it becomes supporting map data rather than the source of player slots.

### Interfaces / Contracts

#### UI to App

`SkirmishShellState` should expose:

```text
launch_session(&self, maps: &[MapMenuEntry]) -> Result<SkirmishLaunchSession, LaunchValidationError>
```

Validation belongs above sim and should catch:

- no selected map
- no enabled opponent when the mode requires an opponent
- duplicate explicit start slots if native validation rejects them
- duplicate or invalid colors if native validation rejects them
- invalid team/capacity states once those rules are fully traced

Known caveat: some native Start validation details are already researched in the handoff docs but not all are represented in current Rust UI. Implementation should start with the verified fields and name any remaining validation as incomplete rather than silently accepting parity drift.

#### App to Init

Map loading should accept an optional launch session rather than only `SkirmishSettings`:

```text
load_map(..., launch: Option<&SkirmishLaunchSession>, ...)
```

The exact signature can differ, but the data flow should make skirmish startup explicit. Campaign/debug/spawn-pick paths should not fabricate partial native sessions unless they are deliberately testing skirmish startup.

#### Init to Sim

`app_init` should translate the launch session into:

- `Simulation.game_options`
- `Simulation.houses`
- initial local owner
- AI setup flags/difficulty
- startup entities generated by the skirmish Battle-mode consumer

`sim/` receives already-normalized data. It should not parse shell rows or map menu entries.

### Data Flow

Target data flow:

```text
OwnerDrawButton::StartGame0x617
  -> SkirmishShellState::launch_session
  -> GameScreen::Loading { map_name, skirmish_launch }
  -> app_init::load_map(..., skirmish_launch)
  -> parse map/rules/art/terrain
  -> spawn map-authored entities
  -> create skirmish runtime houses from launch slots
  -> apply launch options to GameOptions
  -> assign starts from launch row start fields plus map waypoints
  -> Battle-mode startup units/MCVs for all active houses
  -> setup AI from launch AI slots
  -> InGame
```

This intentionally keeps the map preview pipeline separate:

```text
Choose Map / selected map
  -> decode PreviewPack row-major RGB
  -> shell preview texture
```

Preview decode can help select/display maps, but it does not define gameplay starts.

### Error Handling

Use typed launch validation errors at the UI/app boundary. These are user-facing enough that the shell can decide whether to keep the dialog open, disable Start, or show feedback later.

Use `anyhow` only at app-level loading boundaries where map/rules/assets already propagate contextual errors.

Do not let invalid or under-specified launch sessions silently fall back to the current two-MCV smoke-test path. That path is useful for dev tests, but it should not be the standard native shell launch behavior.

### Testing Strategy

Focused tests should cover the contract before full visual parity:

- `skirmish_shell_packs_all_enabled_rows_into_launch_session`: local plus multiple enabled AI rows preserve country/color/team/start/difficulty.
- `skirmish_launch_options_default_build_off_ally_enabled`: standard default session maps to `GameOptions { build_off_ally: true }`.
- `skirmish_create_houses_uses_launch_slots_not_map_roster`: player house count/order comes from launch rows, not map roster order.
- `skirmish_explicit_start_table_assigns_human_then_ai`: explicit starts for local and AI rows are consumed for all active houses.
- `skirmish_startup_does_not_cap_two_players`: four enabled players receive houses and startup placement attempts.
- `skirmish_auto_start_deficient_waypoints_is_marked_unimplemented_or_fallback`: until fallback random starts are implemented, the test should make the gap explicit rather than silently returning no local owner.

Later placement tests, after the BuildOffAlly consumer patch:

- `build_off_ally_enabled_accepts_allied_eligible_provider`
- `build_off_ally_requires_eligibile_for_ally_building`
- `build_off_ally_off_keeps_own_base_provider`

### Determinism

The launch session must be deterministic data:

- stable player row order
- stable enabled-AI filtering order
- stable color/start/team representation
- no hash maps in launch ordering
- random start fallback must use the engine's deterministic RNG path, not host randomness

If launch session data affects simulation state, it must either be represented in `GameOptions`, `HouseState`, entities, or other hashed sim data. The launch object itself does not need to remain in `sim/` after initialization unless replay/debug tooling requires it.

## Architectural Decisions

- The launch session is an app-level contract, not a sim type. This preserves the rule that `sim/` does not depend on UI or shell code.
- `SkirmishSettings` should stop being the core native shell handoff. It can remain for the older egui main menu route during migration.
- Runtime skirmish houses should come from launch slots. Map house roster stays available for map-authored house data, neutral/special content, and compatibility, but it is not the source of player slots for native skirmish launch.
- Battle mode is the first concrete startup consumer because standard offline skirmish uses this path and it has enough verified evidence to begin. Non-Battle mode callbacks remain a research-gated extension.
- `BuildOffAlly` belongs in launch options and `GameOptions`, but its visible gameplay effect belongs to tactical building placement, not startup.

Tech debt accepted during staging:

- The exact selected-map token loader may initially continue using Rust's current `selected_map_idx -> MapMenuEntry.file_name` path. This is marked as a research gate, not considered final parity.
- Exact Battle `+0x84` start-unit callback internals may initially be represented by a narrow Battle startup helper constrained by current evidence. Any unknown formulas must be named in tests/docs rather than guessed.

## Alternatives Considered

### Expand `SkirmishSettings`

This would add rows/options to the existing struct. It is lower-churn at first but keeps the wrong abstraction as the central handoff. The current struct is an approximate menu settings object and already encourages collapsing player rows too early.

Rejected as the primary design.

### Fully Re-Investigate Every Mode Before Any Design

This would avoid unknowns in later startup consumers, but it blocks the already-verified launch contract. The unresolved facts are downstream consumer details, not blockers for carrying correct session data.

Rejected as a prerequisite. Keep targeted RE gates for selected-map loader and mode callback internals.

### Keep Current Two-MCV Seeder And Add Options Around It

This would be a quick smoke-test improvement but leaves obvious player-visible parity holes: more than two players, AI row identity, explicit AI starts, UnitCount, fallback starts, and native house creation source.

Rejected because it preserves the current mismatch.

## Follow-Up Research Gates

Before claiming full startup parity:

1. `/re-investigate skirmish selected map token to scenario loader after 0x617 modal success`
2. `/re-investigate skirmish Battle mode +0x84 start unit and MCV placement callback body`

These are not prerequisites for implementing the launch-session contract, but they are prerequisites for declaring full standard-skirmish startup parity.

