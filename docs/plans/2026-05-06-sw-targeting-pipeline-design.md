# Superweapon Click → Target → Fire UI Pipeline — Design

## Goal

Wire the UI side of superweapon launch: clicking a charged SW cameo on the
sidebar enters a targeting cursor mode; clicking a tactical-map cell emits
`Command::LaunchSuperWeapon`; right-click and Esc cancel.

## Architecture Context

**Sim side is fully wired.** `Command::LaunchSuperWeapon { sw_type_id,
target_rx, target_ry }` at `src/sim/command.rs:130` dispatches in
`world_commands.rs:917-1020` to per-kind handlers (LightningStorm,
IronCurtain, ForceShield, GeneticConverter, PsychicReveal, ParaDrop,
AmerParaDrop). Per-house instances live in
`Simulation::super_weapons: BTreeMap<owner_iid, BTreeMap<sw_type_iid,
SuperWeaponInstance>>` with `is_active`, `is_ready`, `is_suspended`. The
launch handler validates ready state and calls `reset_after_fire()` on
success. 1588 sim tests pass.

**Cameo render is wired.**
`crate::sim::superweapon::superweapon_views_for_owner` returns
`SuperWeaponView` (carrying interned `type_id`, `display_name` set to the
INI section name, `is_ready`, `is_online`, `sidebar_image`, `kind`).
`build_sidebar_view_with_spec` in `src/sidebar/sidebar_view.rs:53` prepends
SW entries to the Defense tab via `collect_build_entries`. Cameos render
with the GCLOCK2 progress overlay just like build cameos.

**Building placement is the structural template.** `armed_building_placement:
Option<String>` + `building_placement_preview: Option<BuildingPlacementPreview>`
in `AppState`. `apply_sidebar_action(SidebarAction::ArmPlacement(..))` arms
it; per-tick `update_building_placement_preview` recomputes the ghost cell;
`place_ready_building_at_cursor` reads the preview cell and emits
`Command::PlaceReadyBuilding`; right-click / Esc clear. `sync_armed_building_placement`
runs in `current_sidebar_view` to clear the mode if the building is no
longer ready.

**Cursors are loaded.** `CursorId` enum has `Nuke`, `Chronosphere`,
`IronCurtain`, `LightningStorm`, `Paradrop`, `ForceShield`,
`GeneticMutator`, `AirStrike`, `PsychicDominator`, `PsychicReveal`,
`SpyPlane`. `cursor_atlas.rs:250-340` already maps each to a frame range
in mouse.sha. Nothing to load — only need an `Action=` string → `CursorId`
table.

**Today's gap.** Clicking a charged SW cameo today emits
`SidebarAction::ArmPlacement("INTICON")` (since SW `type_id` is set to the
SidebarImage SHP name), which `sync_armed_building_placement` immediately
clears because "INTICON" is not a buildable. So clicking a charged SW does
nothing.

## Impact Analysis

### Touched files

- `src/app_types.rs` — new `TargetingMode` enum; new
  `CursorFeedbackKind::SuperWeaponTarget(CursorId)` variant.
- `src/app.rs` — replace `armed_building_placement: Option<String>` with
  `targeting_mode: Option<TargetingMode>`; add helper accessors.
- `src/sidebar/mod.rs` — `SidebarAction` gets `ArmSuperWeapon(String)` and
  `ClearSuperWeaponMode`; `SidebarItem` gets `is_superweapon: bool` and
  `super_weapon_section: Option<String>`; `hit_test` branches on the new
  flag.
- `src/sidebar/sidebar_view.rs` — `BuildEntry` gets the same two new
  fields; SW branch in `collect_build_entries` populates them and computes
  `is_armed` against the new targeting mode; `build_sidebar_view_with_spec`
  signature swaps the `armed_building: Option<&str>` parameter for
  `armed: Option<&TargetingMode>`.
- `src/app_input.rs` — `apply_sidebar_action` handles two new variants;
  Left/Right click and Esc handlers read/clear `targeting_mode`.
- `src/app_sidebar_render.rs` — `sync_armed_building_placement` becomes
  `sync_targeting_mode`, also handling SW invalidation; pass-through
  parameter to `build_sidebar_view_with_spec` updated.
- `src/app_cursor.rs` — new `super_weapon_cursor_id` table; new branch in
  `current_cursor_feedback_kind` ahead of the building-placement branch.
- `src/app_sim_tick.rs` — `update_building_placement_preview` reads
  `targeting_mode.as_building_placement()` instead of
  `armed_building_placement.as_deref()`.
- `src/app_commands.rs` — new `launch_super_weapon_at_cursor`; existing
  writers of `armed_building_placement = None` switch to clearing
  `targeting_mode`.
- `src/app_render_tests.rs` — rewrite the two existing
  `sync_armed_building_placement` tests to drive `sync_targeting_mode`
  with `TargetingMode`.

### Determinism

UI-only state. The single sim-touching write is `Command::LaunchSuperWeapon`
emitted via `schedule_command()`, which already enforces
`sim.input_delay_ticks` for net lockstep. No tick-order or state-hash
impact.

### Risk areas

- `SidebarItem` shape change: any literal constructor must add the new
  fields. Grep before editing.
- `CursorFeedbackKind` derive currently requires `Copy`; `CursorId` is
  `Copy`, so `SuperWeaponTarget(CursorId)` keeps the derive.
- 24 existing call sites of `armed_building_placement` need migration. All
  enumerated; the migration is mechanical via accessor helpers.

## Chosen Approach

**Approach 2 — unified `TargetingMode` enum.** Replace
`armed_building_placement: Option<String>` with a single
`targeting_mode: Option<TargetingMode>` field where the variant is either
`BuildingPlacement(String)` or `SuperWeapon(String)`. Mutual exclusion is
enforced by construction.

Rejected alternatives:
- *Parallel `Option<String>` fields* (Approach 1) — minimal blast radius
  but introduces a new "did I forget to clear the other one?" bug class.
- *Hard-code SW dispatch in `handle_sidebar_mouse_input`, bypass
  SidebarAction* (Approach 3) — hidden coupling, untestable through the
  public sidebar interface.

## Tiny-Detail Ledger

Each item is a parity-relevant detail the implementation must preserve.
Sources cited.

- **Click on charging SW does nothing** — no fire, no audio, no state
  change. `[doc: SUPERCLASS_SYSTEM_GHIDRA_REPORT.md §1]`
- **Right-click on SW cameo does nothing** (vs build cameos which cancel a
  queued item; SWs have no queue). `[doc: SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md]`
- **Click on ready SW arms targeting**; click again on the same cameo
  while armed clears it (toggle). `[matches src/sidebar/mod.rs:313-318]`
- **Targeting cursor sprite per `Action=` string**:
  - `Nuke` → `CursorId::Nuke`
  - `ChronoSphere` → `CursorId::Chronosphere`
  - `ChronoWarp` → `CursorId::Chronosphere`
  - `IronCurtain` → `CursorId::IronCurtain`
  - `LightningStorm` → `CursorId::LightningStorm`
  - `ParaDrop` → `CursorId::Paradrop`
  - `AmerParaDrop` → `CursorId::Paradrop`
  - `PsychicDominator` → `CursorId::PsychicDominator`
  - `SpyPlane` → `CursorId::SpyPlane`
  - `GeneticConverter` → `CursorId::GeneticMutator`
  - `ForceShield` → `CursorId::ForceShield`
  - `PsychicReveal` → `CursorId::PsychicReveal`
  - `IonCannon` → no mapping (TS-legacy, no YR SW uses it)
  `[ini: rulesmd.ini Action= grep + render/cursor_atlas.rs:250-340]`
- **Cursor sprite does NOT change based on cell validity** — same reticle
  on shroud, water, impassable terrain, enemy units. Sim decides.
  `[doc: SUPERWEAPON_LAUNCH_HANDLERS_REPORT.md]`
- **No range circle drawn on the tactical map** during targeting.
  `[gamemd reference behavior]`
- **Cursor frames animate at `interval_ms`** like every other software
  cursor. `[src/app_cursor.rs:412 current_software_cursor_frame]`
- **Click rejection on UI chrome / sidebar / minimap** — sidebar handler
  runs first and consumes the click before the targeting-fire check.
  `[matches src/app_input.rs:46-64]`
- **Right-click cancels targeting**, no fire, no sim command.
  `[matches src/app_input.rs:147-155]`
- **Esc cancels targeting** (same as right-click).
  `[matches src/app_input.rs:298-300]`
- **Auto-cancel when SW becomes not-ready** — granting building destroyed
  (`is_active=false`) or charge reset (`is_ready=false`).
  `[matches sync_armed_building_placement at app_sidebar_render.rs:148]`
- **Pause does NOT clear targeting** — sim doesn't tick, SW state can't
  change. `[matches src/app_input.rs:286-309]`
- **Mutual exclusion** with building placement enforced by `TargetingMode`
  enum. `[gamemd — only one targeting active at a time]`
- **`Command::LaunchSuperWeapon` execute_tick = current + input_delay_ticks**
  via `schedule_command()`. `[src/sim/command.rs CommandEnvelope]`
- **Command carries the SW INI section name** (e.g.,
  `"LightningStormSpecial"`) interned, NOT the SidebarImage SHP name. The
  sim's per-kind dispatch resolves type → kind via
  `rules.super_weapon(type_str).kind`.
  `[src/sim/world/world_commands.rs:925-948]`
- **`type_id` collision avoidance** — multiple SWs can share
  `SidebarImage=INTICON`. Targeting state stores the SW section name
  (unique), not the SHP name. `[ini: rulesmd.ini]`
- **Cursor hotspot per `SoftwareCursorSequence::hotspot`** — no per-SW
  override. `[render/cursor_atlas.rs]`
- **`AuxBuilding=` validation** (Nuke needs Silo) — already enforced
  sim-side via `is_active` (granting building must exist). UI does not
  duplicate. `[src/sim/superweapon/mod.rs:247-313]`

### Known parity drifts (accepted, deferred)

- **Two-click Chrono targeting** (`PreClick=yes` / `PostClick=yes` for
  ChronoSphere/ChronoWarp). Out of scope. Chrono cameos will arm and fire
  on a single click; the sim hits `other =>` and warns. Address when
  Chrono launch handlers ship sim-side.
  `[doc: SUPERCLASS_SYSTEM_GHIDRA_REPORT.md §3 0xED/0xEE]`
- **EVA "Select target" voice cue on arm** — gamemd plays a voice line
  when entering targeting mode. Not wired.
  `[doc: SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md]`
- **Cursor sprite while hovering the cameo of an armed SW** — UNKNOWN
  reference behavior. Current design falls back to default cursor over
  sidebar chrome. `[UNKNOWN — needs RE]`

## Design

### Components

#### `TargetingMode` enum (new, in `app_types.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetingMode {
    BuildingPlacement(String),  // building INI section name
    SuperWeapon(String),        // SW INI section name
}

impl TargetingMode {
    pub fn as_building_placement(&self) -> Option<&str>;
    pub fn as_super_weapon(&self) -> Option<&str>;
    pub fn is_building_placement(&self) -> bool;
    pub fn is_super_weapon(&self) -> bool;
}
```

#### `AppState` field (replaces existing)

```rust
pub(crate) targeting_mode: Option<TargetingMode>,
```

Plus accessor helpers on `AppState`:

```rust
pub fn armed_building_type(&self) -> Option<&str>;
pub fn armed_super_weapon_type(&self) -> Option<&str>;
```

#### `SidebarAction` (extension)

```rust
SidebarAction::ArmSuperWeapon(String),    // payload: SW section name
SidebarAction::ClearSuperWeaponMode,
```

#### `SidebarItem` (extension)

```rust
pub is_superweapon: bool,
pub super_weapon_section: Option<String>,
```

#### `CursorFeedbackKind` (extension)

```rust
SuperWeaponTarget(CursorId),
```

`cursor_id_for_feedback` passes through unchanged.

#### `super_weapon_cursor_id` (new helper in `app_cursor.rs`)

Static mapping from `Action=` string to `CursorId`. Returns `None` for
`IonCannon` and unknown strings (caller falls back to `CursorId::Default`).

#### `launch_super_weapon_at_cursor` (new in `app_commands.rs`)

```rust
pub(crate) fn launch_super_weapon_at_cursor(state: &mut AppState, section: &str);
```

Resolves cursor cell, schedules `Command::LaunchSuperWeapon`, clears
targeting mode.

#### `sync_targeting_mode` (replaces `sync_armed_building_placement`)

Per-frame sync called in `current_sidebar_view`. Validates that the armed
target (building or SW) still satisfies its preconditions (ready building
exists, or SW is active+ready). Clears `targeting_mode` and
`building_placement_preview` if not.

### Interfaces / Contracts

- `build_sidebar_view_with_spec` parameter `armed_building: Option<&str>`
  becomes `armed: Option<&TargetingMode>`.
- `hit_test` signature unchanged; behavior branches on
  `SidebarItem::is_superweapon`.
- `Command::LaunchSuperWeapon { sw_type_id, target_rx, target_ry }` —
  unchanged, used as-is.

### Data Flow

```
Sidebar render (current_sidebar_view)
  ↓ sync_targeting_mode validates current armed state
  ↓ build_sidebar_view_with_spec(..., armed: Option<&TargetingMode>)
  ↓   collect_build_entries marks SW entries (is_superweapon=true,
  ↓   super_weapon_section=Some(section), is_armed = matches)
  ↓ SidebarView { items: [..., SidebarItem { is_superweapon, ... }] }

Mouse click on cameo
  → handle_sidebar_mouse_input
  → hit_test → SidebarAction::ArmSuperWeapon(section)
              | SidebarAction::ClearSuperWeaponMode
  → apply_sidebar_action sets/clears state.targeting_mode

Cursor over tactical map while SuperWeapon mode active
  → current_cursor_feedback_kind
  → super_weapon_cursor_id(action) → CursorId
  → CursorFeedbackKind::SuperWeaponTarget(id)
  → cursor_id_for_feedback → renders the per-SW reticle

Left-click on tactical map while SuperWeapon mode active
  → handle_mouse_input (Left release)
  → launch_super_weapon_at_cursor(state, section)
  → screen_point_to_world_cell → (rx, ry)
  → schedule_command(Command::LaunchSuperWeapon { sw_type_id, rx, ry })
  → state.targeting_mode = None

Right-click / Esc while SuperWeapon mode active
  → state.targeting_mode = None (no command emitted)

Per-tick (sync_targeting_mode)
  ↓ if SuperWeapon(section) && (!is_active || !is_ready) → clear
  ↓ if BuildingPlacement(type) && type not in ready_buildings → clear
```

### Error Handling

- Unknown `Action=` string → cursor falls back to `CursorId::Default`.
  No error logged; future modders may add new SW types and this should
  degrade gracefully.
- SW becomes not-ready while targeting → sync clears silently. No log
  spam; this is a normal occurrence (granting building destroyed mid-aim).
- Click on tactical while SW becomes ineligible between cursor-frame and
  click-frame → sim's `Command::LaunchSuperWeapon` dispatch already
  validates `is_active && is_ready` and logs a warn if not. UI doesn't
  pre-validate.

### Testing Strategy

Unit tests:
- `sidebar/mod.rs::tests` — extend `hit_test` tests:
  - SW item ready+not-armed, left-click → `ArmSuperWeapon(section)`
  - SW item ready+armed, left-click → `ClearSuperWeaponMode`
  - SW item not-ready, left-click → `None`
  - SW item, right-click → `None`
- `sidebar/sidebar_view.rs::tests` — extend builder tests:
  - SW entry has `is_superweapon=true`, `super_weapon_section=Some(..)`
  - `is_armed` reflects matching `TargetingMode::SuperWeapon(..)`
- `app_render_tests.rs` — rewrite two existing tests to use
  `sync_targeting_mode` with `TargetingMode`. Add three new tests:
  - SW armed, SW still active+ready → preserved
  - SW armed, SW becomes not-ready → cleared
  - SW armed, SW becomes inactive (granting building lost) → cleared
- `app_cursor.rs::tests` — table test for `super_weapon_cursor_id`:
  every YR-active Action string maps to a non-default cursor;
  `IonCannon` and unknowns return `None`.

No new sim-side tests needed — `Command::LaunchSuperWeapon` dispatch and
per-kind handlers are already covered by the 1588 existing tests.

### Determinism Considerations

Targeting state is local to one client. The only sim-touching emission is
the `Command::LaunchSuperWeapon` envelope, which is already lockstep-safe
via `schedule_command()`. No new state-hash inputs.

## Architectural Decisions

### Patterns followed

- `SidebarAction` enum extension — same pattern as `ArmPlacement` /
  `ClearPlacementMode`.
- Per-tick sync of UI-armed-state against current sim state — same pattern
  as `sync_armed_building_placement`.
- `schedule_command` for net-deterministic emission — same pattern as
  every other order command.
- Cursor selection via lookup table → `CursorFeedbackKind` → `CursorId`
  — same pattern as edge-scroll and capability cursors.

### Patterns deviated from

- Replaces `armed_building_placement: Option<String>` with
  `targeting_mode: Option<TargetingMode>`. Rationale: mutual-exclusion-by-
  construction. Cost: 24 mechanical call-site updates. Justified because
  the alternative (parallel `Option<String>` fields) opens a "forgot to
  clear the other" bug class that grows worse as more targeting modes are
  added.

### Tech debt

None introduced. The migration is a state-shape refactor with no
behavioral diff for building placement.

## Alternatives Considered

### Approach 1 — Parallel `armed_super_weapon: Option<String>` field

Smallest blast radius; touches ~10 sites instead of ~24. Rejected because
it leaves the mutual-exclusion invariant maintained by hand at every set
site, which is fragile as more targeting modes appear.

### Approach 3 — Hard-code SW arm in `handle_sidebar_mouse_input`,
skip `SidebarAction`

Bypasses the public sidebar interface, leaves SW handling untestable
through `hit_test`, duplicates `collect_build_entries` logic via side-
channel re-query. Rejected as a hidden-coupling anti-pattern.
