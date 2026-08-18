# Skirmish Choose Map Modal Design

## Goal

Replace the current Skirmish `Choose Map` in-place map cycling with the retail Yuri's Revenge modal chooser contract.

The target is player-visible parity for opening the chooser, listing maps in retail order, filtering by selected game mode, accepting/canceling selection, rebuilding dependent setup rows, and refreshing the selected-map preview.

This design does not implement random map generation internals or complete post-launch MPModes callbacks. It creates the shell/data-model structure needed for those later systems without guessing them.

## Current Rust Mismatch

Current Rust has a `ChooseMap0x5aa` control identity, but `apply_action` handles it by incrementing `selected_map_idx`:

```rust
state.selected_map_idx = (state.selected_map_idx + 1) % maps.len();
```

That is not the retail behavior. Retail hides the setup dialog, opens modal dialog `0x6B`, lets the player choose a mode/map pair from listboxes, then returns with accept or cancel semantics.

The current `available_maps: Vec<MapMenuEntry>` is also the wrong backing model for the retail chooser. It is built by scanning loose files and sorting by display name. Retail uses a source-ordered scenario record list and does not sort or collapse duplicate-looking records.

## Verified Retail Constraints

These facts come from the Skirmish Choose Map reswarm and sibling Ghidra reports.

### Modal Contract

- Setup dialog `0x102` command `0x5AA` opens modal dialog resource `0x6B`.
- Parent setup hides before the modal and returns after the modal result.
- Cancel/back control `0x5C0` closes with result `2`.
- Accept control `0x6C5` closes with result `1`.
- Cancel restores old selected mode/map globals, reloads the restored selected record, refreshes preview, then shows/repaints setup.
- Accept commits through `0x005E7160`, then parent recomputes map capacity, applies selected-mode clamp, rebuilds row/combo state, and only then shows setup.
- Accepted selected-record load failure restores old token/index and returns before normal label/preview refresh.
- Preview refresh replaces/loads a wrapper and invalidates the parent; later paint consumes it. Random-map records use `RandMap.img`.

### Modal Layout

- Choose Map is PE `RT_DIALOG` resource `0x6B`, `DIALOGEX`, `533x369`, 11 controls, `MS Sans Serif` 8 pt.
- Modal background path uses `MnScrnLCustomizeBattle.shp/.PAL`, not the setup dialog `MnScrnLCoopGameSetup` path.
- Key controls:
  - `0x6EB`: game-type listbox, rect `(77,78,130,211)`.
  - `0x553`: map listbox, rect `(225,78,130,211)`.
  - `0x6C5`: Use Map.
  - `0x5C0`: Cancel.
  - `0x583`: Create Random Map.
  - `0x694`: title/static.
  - `0x468`: preview placeholder.
- `0x6EB` and `0x553` are listboxes with style `0x50000151`, not dropdown combos. They lack `LBS_SORT`.

### MPModes And Filtering

- `0x6EB` rows come from the global MPModes object vector.
- Each `0x6EB` row stores the MPModes object pointer as item data.
- Initial `0x6EB` selection is by numeric mode id at `mode+0x28`, matched against the current selected-mode id; if missing, native falls back to the first mode.
- Filtering uses selected mode filter string at `mode+0x30`, not visible UI labels and not category section names.
- Empty map `GameModes` matches only selected mode filter `standard`, not every mode.
- `RandMap.Sed` bypasses normal `GameModes` comparison and dispatches selected mode random-map callback.

### Scenario Source Order

Retail source order for standard YR is:

1. `MISSIONSMD.PKT` `[MultiMaps]`
2. loose `*.PKT`
3. loose `*.YRO` embedded PKT
4. loose `*.YRM`

The builder does no display-name sort and no duplicate suppression. The modal preserves source append order after filtering.

No `MISSIONS.PKT` literal/source branch was verified in this standard YR builder.

### Random Map Sentinel

- `RandMap.Sed` is a synthetic chooser record, not a loose map scan result.
- It is created/updated by the random-map command `0x583 -> 0x005E8590`.
- Once present, it appears in the normal chooser list only when selected MPModes allows random maps.
- The gate is selected mode vtable `+0x3C`, backed by mode byte `+0x34`, parsed from the `MPModesMD.ini` fifth column.
- Stock random-map-allowed modes are Battle id `1` and FreeForAll id `2`.
- Accept commits the ordinary scenario-record index; there is no negative or special random accept token.
- Launch copies `RandMap.Sed` into the scenario path; `.SED` detection routes into random map generation.

## Architecture Context

Current relevant code:

- `src/ui/skirmish_shell/state.rs`: shell state, hit testing, combo/dropdown state, launch packing, and the current wrong `ChooseMap` cycling behavior.
- `src/ui/skirmish_shell/layout.rs`: setup dialog `0x102` layout helpers.
- `src/app_skirmish_shell_render.rs`: setup shell sprite/text construction and selected preview texture cache.
- `src/app_list_maps.rs`: current loose map discovery and metadata extraction.
- `src/app_init.rs`: `MapMenuEntry` and load-time map metadata structures.
- `src/skirmish_launch.rs`: data-only launch-session types.
- `src/app.rs`: routes shell mouse actions and owns `available_maps`.

Layering constraints:

- `sim/` must not depend on shell UI, render, sidebar, audio, or net.
- Retail control IDs and modal state belong in `ui/skirmish_shell` or app shell orchestration, not in `sim/`.
- Render code should draw from state/layout; it should not own selection truth.
- App-level map loading can continue to consume `selected_map_file`, but the chooser must move toward retail scenario record identity.

## Chosen Approach

Implement a native-shaped Choose Map modal in three internal slices:

1. Data model: MPModes roster plus source-ordered scenario records.
2. Modal state/layout/render: dialog `0x6B` with two listboxes and buttons.
3. Accept/cancel integration: commit or restore selected mode/map and rebuild dependent setup state.

This approach is preferred over a visual-only modal because the list contents and accept/cancel behavior are part of the visible parity. It is preferred over wiring the modal to current `available_maps` because that would preserve the wrong sort order, missing MPModes filtering, and duplicate suppression risk.

## Data Model

### `SkirmishGameMode`

Add a shell/app-level mode model, likely in a new module such as `src/skirmish_modes.rs` or `src/ui/skirmish_shell/modes.rs`:

```rust
pub struct SkirmishGameMode {
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

Source:

- Parse stock `ini/mpmodesmd.ini`.
- Preserve stock row order by inserted mode id sorting, matching the existing MPModes report.
- Merge `[MultiplayerDialogSettings]` defaults when override files are available.

Minimum first implementation fields:

- `id`
- `ui_name_key`
- `map_filter`
- `random_maps_allowed`
- `allies_allowed`
- `must_ally`

The rest can be carried as strings for later UI/tooltips.

### `SkirmishScenarioRecord`

Add a source-ordered chooser record distinct from `MapMenuEntry`:

```rust
pub struct SkirmishScenarioRecord {
    pub source_ordinal: usize,
    pub source: SkirmishScenarioSource,
    pub file_name: String,
    pub display_name: String,
    pub author: Option<String>,
    pub briefing: BriefingSection,
    pub preview: PreviewSection,
    pub multiplayer_start_waypoints: Vec<Waypoint>,
    pub preview_source_bounds: Option<PreviewSourceBounds>,
    pub game_modes: Vec<String>,
    pub min_players: Option<u8>,
    pub max_players: Option<u8>,
    pub official: bool,
    pub kind: SkirmishScenarioKind,
}
```

```rust
pub enum SkirmishScenarioKind {
    ConcreteMap,
    RandomMapSentinel,
}
```

`MapMenuEntry` can remain as a compatibility projection for existing loading and preview code. The chooser should not be driven directly by sorted `available_maps`.

### Source Discovery

Create a retail chooser list builder, separate from the current loose-file main-menu helper.

First implementation should support:

1. `MISSIONSMD.PKT` `[MultiMaps]` records when accessible from retail assets.
2. loose `*.PKT` `[MultiMaps]`.
3. loose `*.YRO` embedded PKT records.
4. loose `*.YRM` direct records.

Do not sort by display name. Do not deduplicate by display name or path. Preserve `source_ordinal`.

If one source type is not yet loadable from current asset APIs, the builder should make that explicit in logs/tests and still preserve the source-order structure for implemented sources.

## Modal State

Add explicit modal state to the app/shell state boundary:

```rust
pub struct ChooseMapModalState {
    pub saved_mode_id: i32,
    pub saved_record_ordinal: usize,
    pub highlighted_mode_id: i32,
    pub highlighted_record_ordinal: Option<usize>,
    pub filtered_record_ordinals: Vec<usize>,
}
```

The highlighted list row is not committed until Use Map.

Opening the modal:

1. Save current selected mode id and selected scenario ordinal.
2. Initialize `highlighted_mode_id` from current selected mode id, falling back to first selectable mode.
3. Build `filtered_record_ordinals` using the selected mode.
4. Highlight the current selected record by record identity/ordinal if it appears in the filtered list.
5. Switch app shell route to Choose Map modal view.

Cancel:

1. Restore saved selected mode id and selected scenario ordinal.
2. Rebuild filtered/setup state from the restored record.
3. Refresh preview for the restored selection.
4. Return to setup shell.

Accept:

1. Require a highlighted record.
2. Commit selected mode id and scenario ordinal.
3. Recompute selected-map capacity.
4. Apply selected-mode clamp when modeled.
5. Rebuild enabled/visible row controls and start-position choices.
6. Refresh setup map/mode labels.
7. Invalidate/rebuild selected preview cache.
8. Return to setup shell.

Accepted load failure:

1. Restore saved selected mode id and selected scenario ordinal.
2. Skip normal label/preview refresh for the failed selection.
3. Return to setup shell in the restored state.

## Filtering

Filter records from the full source-ordered list:

```text
for record in records.source_order():
  if record.kind == RandomMapSentinel:
    include if selected_mode.random_maps_allowed
  else if record.game_modes.is_empty():
    include if selected_mode.map_filter == "standard"
  else:
    include if record.game_modes contains selected_mode.map_filter
```

Do not filter by:

- visible `GUI:*` strings;
- MPModes category section names such as `Battle` or `ManBattle`;
- display name;
- sorted map index.

The optional `Official` gate found in retail should be represented in the model, but can remain inactive until the corresponding runtime mode is verified for the Rust shell path.

## Layout And Render

Add a Choose Map modal layout module or section, separate from setup `0x102` layout:

```rust
pub struct ChooseMapModalLayout {
    pub screen: RectPx,
    pub dialog_rect: RectPx,
    pub mode_list: RectPx,
    pub map_list: RectPx,
    pub use_map_button: RectPx,
    pub cancel_button: RectPx,
    pub create_random_button: RectPx,
    pub title: RectPx,
    pub preview: RectPx,
}
```

Use verified dialog `0x6B` control rects as the base. Apply the same shell host/viewport policy used by other shell dialogs, but keep this modal visually separate from setup.

Render responsibilities:

- Draw `MnScrnLCustomizeBattle.shp/.PAL` background when available.
- Draw listbox rows for `0x6EB` and `0x553`.
- Draw selected row highlight.
- Draw right-column owner-draw buttons for Use Map, Cancel, and Create Random Map.
- Draw preview placeholder/control `0x468`.
- Use shell text drawing for mode/map labels.

Open uncertainties:

- Exact owner-drawn listbox row paint is not fully verified.
- Whether `0x468` paints a live preview while browsing list selection remains uncertain.
- High-res `>800` modal background behavior needs runtime screenshot confirmation.

First implementation may render listbox rows with the existing shell text system and a simple verified-style selected row highlight, but must keep the row geometry and item-data semantics correct.

## App Integration

App state should own both:

- the full source-ordered scenario record list;
- a projection or compatibility list for existing selected-preview/loading paths.

Candidate fields:

```rust
pub(crate) skirmish_modes: Vec<SkirmishGameMode>,
pub(crate) skirmish_scenario_records: Vec<SkirmishScenarioRecord>,
pub(crate) choose_map_modal: Option<ChooseMapModalState>,
```

`available_maps` can remain for legacy main-menu map selection until the chooser path is migrated. The experimental shell should resolve selected record to a `MapMenuEntry`-like projection for current preview/load consumers.

Change `SkirmishShellAction::ChooseMap` so it no longer mutates `selected_map_idx`. It should bubble to `app.rs`, which opens the modal.

## Launch Integration

`SkirmishLaunchSession` currently carries `SkirmishLaunchMode::Battle` and `selected_map_file`. After this design:

- Add selected mode id/filter to launch session or expand `SkirmishLaunchMode` beyond hardcoded Battle.
- Continue to pass concrete map file names for normal maps.
- For `RandMap.Sed`, preserve the `.sed` token as a selected scenario file/token, but do not claim full random terrain generation parity until the RMG path is implemented.

Do not encode random map as `None`, `-1`, or an out-of-band selected-map index.

## Tests

Unit tests:

- `choose_map_mode_combo_populates_stock_mpmodes_and_selects_by_id`
- `choose_map_filters_by_selected_mpmode_game_modes`
- `choose_map_empty_game_modes_matches_standard_only`
- `choose_map_source_order_preserves_pkt_yro_yrm_order`
- `choose_map_does_not_sort_by_display_name`
- `choose_map_preserves_duplicate_records`
- `choose_map_modal_uses_resource_0x6b_control_rects`
- `choose_map_accept_commits_mode_and_map_cancel_restores_both`
- `choose_map_accept_rebuilds_rows_before_preview_refresh`
- `choose_map_accept_load_failure_restores_previous_map`
- `skirmish_choose_map_filters_randmap_by_mode_random_allowed`
- `skirmish_random_map_command_adds_or_updates_single_sentinel_record`

Focused command checks:

- `cargo test skirmish_shell`
- `cargo test choose_map`
- `cargo test skirmish_launch`

Manual screenshot checks after render implementation:

- `800x600` setup -> click Choose Map -> modal appears with `MnScrnLCustomizeBattle`.
- Cancel returns to previous setup selection.
- Accept different map changes map text/preview and row capacity immediately.
- Team Game filters to `teamgame` maps.
- Battle/Free For All can show generated `RandMap.Sed` sentinel after Create Random Map; other stock modes do not.

## Implementation Order

1. Add MPModes model/parser tests.
2. Add `SkirmishScenarioRecord` and source-ordered builder tests.
3. Project existing map metadata into scenario records without changing launch behavior.
4. Add Choose Map modal state and action routing; remove in-place cycling.
5. Add modal layout tests for dialog `0x6B`.
6. Render modal background, listboxes, buttons, and labels.
7. Implement mode/map filtering and highlight behavior.
8. Implement cancel restore.
9. Implement accept commit, capacity rebuild, preview cache invalidation.
10. Add random-map sentinel record and mode-gated filtering.
11. Wire launch/session selected mode id and `.sed` token handling.

## Negative Facts / Do Not Do

- Do not keep `ChooseMap` as an in-place next-map button.
- Do not drive the retail chooser from sorted `available_maps`.
- Do not sort chooser records by display name.
- Do not collapse duplicate records.
- Do not filter by visible labels or MPModes category names.
- Do not treat missing `GameModes` as match-all.
- Do not treat `0x6EB` as a dropdown combo.
- Do not copy setup dialog `0x102` controls into modal `0x6B`.
- Do not use `MnScrnLCoopGameSetup.*` for the Choose Map modal background.
- Do not add `RandMap.Sed` as a permanent loose-map scan result.
- Do not commit random map as a special negative index.
- Do not decode `[PreviewPack]` for `RandMap.Sed`; native uses `RandMap.img`.
- Do not expose stock Siege in offline Skirmish just because binary support exists.

## Remaining Research Gates

- Exact owner-drawn row paint for listboxes `0x6EB` and `0x553`.
- Whether the modal preview placeholder updates live while browsing list selection.
- Exact localized random-map default display text.
- Random terrain generation formulas after the `.SED` route.
- High-res modal screenshot parity.
- Official-map gate runtime condition for the Rust shell path.

None of these block the core modal/data-model implementation, but they should remain explicit acceptance gaps rather than guessed behavior.
