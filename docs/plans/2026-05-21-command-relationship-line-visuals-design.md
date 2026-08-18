# Command And Relationship Line Visuals Design

## Goal

Implement RA2/YR battlefield line visuals as distinct parity systems while sharing only low-level raster helpers.

## Architecture Context

The current Rust implementation has an app-layer approximation in `src/app_target_lines.rs`. It records click commands into `AppState.target_lines`, keeps a 25-tick timer, and emits 1x1 `SpriteInstance` quads for command feedback lines. It is integrated through `src/app_render/build_instances.rs`, uploaded in `src/app_render/mod.rs`, and drawn as `"target_lines"` in `src/app_render/draw_passes.rs`.

This is the right architectural layer for render-only line construction: `sim/` must not depend on `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`. The app/render layer may read simulation state and build visual instances.

Current simulation state is sufficient for a first selected-action-line pass: `GameEntity.selected`, `GameEntity.attack_target`, `GameEntity.movement_target`, `AttackTarget.target`, `MovementTarget.path`, `MovementTarget.final_goal`, ownership, type, category, and position.

Factory rally lines need a simulation-state correction. Rust currently stores owner-level `HouseState.rally_point: Option<(u16, u16)>`, and production uses it for auto-move after spawn. The verified binary rally renderer reads a selected building's per-producer rally target at `TechnoClass+0x218`. Therefore this design adds per-structure rally target state to `GameEntity` and keeps `HouseState.rally_point` as owner-level production fallback until a broader production-rally migration is needed.

The later line families are documented well enough for architecture, but not all prerequisite gameplay systems exist. Planning path lines need `WaypointPathClass`-style state. Psychic Sensor enemy action lines need local Psychic Detection coverage. Mind-control links need CaptureManager / MCNode gameplay state.

## Impact Analysis

Primary files likely affected by implementation:

| Area | Path | Impact |
|---|---|---|
| App line overlay | `src/app_target_lines.rs` or renamed `src/app_line_overlays.rs` | Split current approximation into family-specific builders and shared raster helpers. |
| App state | `src/app.rs` | Continue storing app-owned selected-action-line timer/options state. |
| Render instance build | `src/app_render/build_instances.rs` | Build separate line-family instance vectors. |
| Render upload | `src/app_render/mod.rs` | Upload separate buffers if draw order requires them. |
| Render draw order | `src/app_render/draw_passes.rs` | Place selected lines, rally passes, and future families in explicit draw locations. |
| Sim entity state | `src/sim/game_entity.rs` | Add per-structure rally target state. |
| Sim command application | `src/sim/world/world_commands.rs` | Update per-producer rally targets from `Command::SetRally`. |
| Production rally fallback | `src/sim/production/production_queue.rs` | Keep existing owner-level rally behavior unless later implementation migrates production spawn to active producer rally. |
| Determinism hash | `src/sim/world/world_hash.rs` | Include per-producer rally target in entity hash. |
| Rules helpers | `src/rules/object_type.rs`, `src/rules/ruleset.rs` | Add helper for rally-line eligible producer if useful. |

Risk areas:

- Draw order can drift because the original has multiple line passes, not one UI overlay bucket.
- State source can drift if selected lines keep using recorded click endpoints instead of live attack/movement state.
- Per-producer rally state changes deterministic simulation state and must be serialized and hashed.
- A generic "line" abstraction would hide family-specific gates, timers, colors, and endpoint rules.
- Visual verification is required because line pixels, endpoint boxes, and phase patterns are player-visible.

## Chosen Approach

Use Approach B: one app-layer line overlay module with separate family builders.

The module should expose a grouping type, conceptually:

```rust
pub(crate) struct LineOverlayInstances {
    pub selected_action: Vec<SpriteInstance>,
    pub factory_rally_first: Vec<SpriteInstance>,
    pub factory_rally_second: Vec<SpriteInstance>,
    pub planning_path_first: Vec<SpriteInstance>,
    pub planning_path_second: Vec<SpriteInstance>,
    pub psychic_sensor_action: Vec<SpriteInstance>,
    pub mind_control_links: Vec<SpriteInstance>,
}
```

Each family owns its gameplay meaning and gates. Shared helpers are allowed only for low-level visual emission:

- project cell/entity endpoints to screen positions
- emit 3x3 endpoint boxes
- emit solid line pixels
- emit dashed line pixels from a fixed pattern
- convert palette/house color inputs to render tints

Implementation should proceed in phases:

1. Selected unit action/target lines.
2. Factory rally lines with per-producer rally target state.
3. Planning/queued waypoint path lines after planning path state exists.
4. Psychic Sensor enemy action lines after Psychic Detection coverage exists.
5. Mind-control links after CaptureManager / MCNode state exists.

## Tiny-Detail Ledger

Selected unit action/target lines:

- Selected action lines are gated by `[Options] UnitActionLines`, mirrored to `DAT_00843108`. Source: `SELECTED_UNIT_ACTION_TARGET_LINE_BASELINE_GHIDRA_REPORT.md`; `RA2.INI:12`.
- Duration is `0x19` / 25 frames after timer start. Source: `SELECTED_UNIT_ACTION_TARGET_LINE_BASELINE_GHIDRA_REPORT.md`.
- Endpoint priority is `ArchiveTarget` first, then `NavQueue.Items[Count - 1]`, then `NavCom`. Source: `SELECTED_UNIT_ACTION_TARGET_LINE_BASELINE_GHIDRA_REPORT.md`.
- Stock selected output draws two clipped 3x3 endpoint boxes offset `(-2,-2)` plus one clipped solid line. Source: `SELECTED_UNIT_ACTION_TARGET_LINE_BASELINE_GHIDRA_REPORT.md`.
- Attack color uses palette index `8`; move color uses palette index `3`. Source: `SELECTED_UNIT_ACTION_TARGET_LINE_BASELINE_GHIDRA_REPORT.md`.
- Stock selected path does not use dashed mode because the live caller pushes zero arguments. Source: `SELECTED_UNIT_ACTION_TARGET_LINE_BASELINE_GHIDRA_REPORT.md`.
- Mobile technos use the real path; buildings use the empty stub. Source: `SELECTED_UNIT_ACTION_TARGET_LINE_BASELINE_GHIDRA_REPORT.md`.

Factory rally lines:

- Selected factory rally lines use `FUN_006DA9D0`, not `Tactical::DrawLine3D`. Source: `FACTORY_RALLY_POINT_LINE_CALLER_COLOR_GATE_GHIDRA_REPORT.md`.
- Gates are selected local building, owner is local player, eligible producer/repair/cloning predicate, and non-null per-building rally target. Source: `FACTORY_RALLY_POINT_LINE_CALLER_COLOR_GATE_GHIDRA_REPORT.md`.
- Eligibility is `Factory=UnitType`, `Factory=InfantryType`, `Cloning=yes`, or `UnitRepair=yes`. Source: `FACTORY_RALLY_POINT_LINE_CALLER_COLOR_GATE_GHIDRA_REPORT.md`; `ini/rulesmd.ini`.
- Target source is per-building `TechnoClass+0x218`, not just owner-level `HouseState.rally_point`. Source: `FACTORY_RALLY_POINT_LINE_CALLER_COLOR_GATE_GHIDRA_REPORT.md`.
- Color uses owner house RGB. Phase is `(0x7FFFFFFF - g_CurrentFrameCounter) % 0xF`. Pattern is `DAT_00842930`. Source: `FACTORY_RALLY_POINT_LINE_CALLER_COLOR_GATE_GHIDRA_REPORT.md`.
- Original draw order has two tactical calls around object/bracket rendering. Source: `FACTORY_RALLY_POINT_LINE_CALLER_COLOR_GATE_GHIDRA_REPORT.md`.

Planning/queued waypoint lines:

- Planning path lines use `FUN_006DAD60`, not `Tactical::DrawLine3D`. Source: `PLANNING_QUEUED_WAYPOINT_LINES_AND_FLAGS_GHIDRA_REPORT.md`.
- Planning path overlay uses `WaypointPathClass`, not `FootClass::NavQueue`. Source: `PLANNING_QUEUED_WAYPOINT_LINES_AND_FLAGS_GHIDRA_REPORT.md`.
- It draws all adjacent stored waypoint segments, with optional loop closure. Source: `PLANNING_QUEUED_WAYPOINT_LINES_AND_FLAGS_GHIDRA_REPORT.md`.
- Tactical marker uses `MOUSE.SHA` action index `0x3C`; `FLAGFLY.SHP` is separate. Source: `PLANNING_QUEUED_WAYPOINT_LINES_AND_FLAGS_GHIDRA_REPORT.md`.
- `MaxWaypointPathLength=15`. Source: `ini/rulesmd.ini:424`.

Psychic Sensor enemy action lines:

- `DrawRadarActionLines @ 0x004DC340` is tactical-screen output, not minimap output. Source: `PSYCHIC_SENSOR_ENEMY_ACTION_LINES_RECHECK_GHIDRA_REPORT.md`.
- It is gated by local Psychic Detection coverage. Standard YR enables this through `[NAPSIS] PsychicDetectionRadius=15`. Source: `PSYCHIC_SENSOR_ENEMY_ACTION_LINES_RECHECK_GHIDRA_REPORT.md`; `ini/rulesmd.ini:13353`.
- It is not controlled by `UnitActionLines` and not controlled by the selected-action-line timer. Source: `PSYCHIC_SENSOR_ENEMY_ACTION_LINES_RECHECK_GHIDRA_REPORT.md`.
- It draws 3x3 endpoint dots plus an animated dashed line using owner house RGB and `timeGetTime()` phase. Source: `PSYCHIC_SENSOR_ENEMY_ACTION_LINES_RECHECK_GHIDRA_REPORT.md`.

Mind-control links:

- Mind-control links use `CaptureManagerClass::DrawLinks @ 0x00472160`, with helper `FUN_00704E40`. Source: `MIND_CONTROL_LINK_LINES_DRAWLINKS_GHIDRA_REPORT.md`.
- Visibility is gated by controller/host/victim selected state or the `MindControlAttackLineFrames` timer, not by "on-screen" visibility. Source: `MIND_CONTROL_LINK_LINES_DRAWLINKS_GHIDRA_REPORT.md`.
- `MindControlAttackLineFrames=20`. Source: `ini/rulesmd.ini:853`.
- Controller endpoint uses `[MIND] AlternateFLH0..4`; victim endpoint uses victim coords plus `TechnoType+0x3DC`. Source: `MIND_CONTROL_LINK_LINES_DRAWLINKS_GHIDRA_REPORT.md`; `ini/artmd.ini`.
- It is not controlled by `[Options] UnitActionLines`. Source: `MIND_CONTROL_LINK_LINES_DRAWLINKS_GHIDRA_REPORT.md`.

## Design

### Components

#### Line overlay module

Keep the current `src/app_target_lines.rs` initially, but reshape it around explicit family builders. A later rename to `src/app_line_overlays.rs` is acceptable only if done mechanically and with module imports updated in one task.

Responsibilities:

- app-owned selected-line timer and option gate
- family-specific instance builders
- shared endpoint-box and line-emission helpers
- no sim mutation

Non-responsibilities:

- pathfinding
- command resolution
- production rally semantics
- Psychic Detection coverage ownership
- CaptureManager / mind-control ownership

#### Per-producer rally target

Add a simulation field to `GameEntity`:

```rust
#[serde(default)]
pub rally_target: Option<(u16, u16)>,
```

Rules:

- Only structures should set this.
- It is deterministic simulation state.
- It must be included in `Simulation::state_hash`.
- It must be initialized to `None`.
- It should not introduce dependencies from `sim/` to render/app.

`Command::SetRally` should update the selected local eligible structures for the command owner when possible. `HouseState.rally_point` remains updated as an owner-level fallback for current production movement.

#### Rally eligibility helper

Add a helper near rules/object lookup, or inside the line overlay if it only reads rules:

```rust
fn is_rally_line_eligible(obj: &ObjectType) -> bool {
    matches!(obj.factory, Some(FactoryType::InfantryType | FactoryType::UnitType))
        || obj.unit_repair
        || obj.cloning
}
```

If `cloning` is not currently parsed, implementation must add `ObjectType.cloning: bool` from `Cloning=yes` before using this helper for parity.

#### Render buffers

Start with existing `"target_lines"` for selected action lines if that keeps the first phase small. For rally lines, add explicit buffers only when draw order needs them:

- `"factory_rally_first"`
- `"factory_rally_second"`

Planning, Psychic Sensor, and mind-control buffers can remain design-only until their prerequisite gameplay systems exist.

### Interfaces / Contracts

`build_line_overlay_instances(state, sw, sh)` should return family-grouped `SpriteInstance` vectors. It may read:

- `state.simulation`
- `state.rules`
- `state.house_color_map`
- `state.height_map`
- `state.bridge_height_map`
- `state.target_lines`
- local player owner helper

It must not mutate simulation state.

`TargetLineState` remains app-owned presentation state for the selected-action-line timer. It should not become authoritative gameplay state.

`GameEntity.rally_target` is authoritative simulation state for selected producer rally visuals. Current owner-level `HouseState.rally_point` remains an owner-level production fallback.

### Data Flow

Selected action line flow:

1. Player issues move/attack command.
2. App starts or refreshes the selected-action-line timer.
3. Sim applies command and updates attack/movement state.
4. Render reads selected mobile entities and chooses attack target before movement target.
5. Render emits endpoint boxes and one solid line.

Factory rally line flow:

1. Player selects one or more structures.
2. Player right-clicks destination.
3. `Command::SetRally` updates owner fallback rally point and selected eligible producers' `rally_target`.
4. Render scans selected local eligible structures.
5. Render emits rally line from structure to `rally_target` using owner color and rally phase.

Planning path flow:

1. Future planning system stores `WaypointPathClass`-equivalent paths.
2. Render draws all adjacent path segments and markers.
3. This is deferred until the planning path state exists.

Psychic Sensor flow:

1. Future sensor system computes local Psychic Detection coverage.
2. Render scans eligible enemy/non-human FootClass-like entities whose endpoint is inside coverage.
3. Render emits tactical dashed enemy action lines.
4. This is deferred until coverage exists.

Mind-control link flow:

1. Future mind-control system stores CaptureManager/MCNode-equivalent links.
2. Render applies selected-state/timer gates.
3. Render emits controller-to-victim lines.
4. This is deferred until CaptureManager state exists.

### Error Handling

Rendering should skip missing data rather than panic:

- no simulation: emit no line instances
- missing entity target: skip that line
- missing rules object: ineligible for rally line
- missing color map: use a conservative fallback only for development; parity implementation should use house color data
- missing palette conversion: keep selected-line implementation isolated so exact palette work can improve without changing family gates

### Testing Strategy

Unit tests:

- selected-line timer expires at 25 frames
- selected-line builder emits two 3x3 endpoint boxes plus body pixels for a simple horizontal line
- attack target wins over movement target
- movement final goal/path endpoint is used instead of stale command record
- rally eligibility helper accepts `Factory=InfantryType`, `Factory=UnitType`, `UnitRepair=yes`, and `Cloning=yes`
- `Command::SetRally` updates `GameEntity.rally_target` only for selected eligible structures owned by the command owner
- `GameEntity.rally_target` changes `Simulation::state_hash`

Visual verification:

- selected infantry/vehicle move click shows a short-lived solid line with endpoint boxes
- selected unit attack click uses attack color and target endpoint
- selected war factory/barracks with rally point shows owner-colored rally line
- selected non-factory building does not show a rally line
- `UnitActionLines=no` hides selected action lines but not future Psychic/MC/rally families

## Architectural Decisions

- Keep line family semantics separate. Shared helpers are pixel/raster helpers only. This prevents selected, rally, planning, Psychic, and MC lines from inheriting each other's gates or timers.
- Add per-producer rally target to `GameEntity`. This follows the binary's per-building source and avoids building more behavior on the current owner-level shortcut.
- Keep selected-action-line timer in app state. It is presentation feedback, not durable gameplay.
- Include per-producer rally target in deterministic state. It affects production behavior and visible selected-building output.
- Defer planning, Psychic Sensor, and MC implementations behind prerequisite systems. The design names their output contracts now so Phase 1 and Phase 2 do not block them.

## Alternatives Considered

### Generic line system

Rejected. A single semantic line abstraction would make the code look clean but would hide the exact differences the research found: selected lines, rally lines, planning paths, Psychic Sensor lines, and mind-control links use different state, gates, timing, colors, and draw helpers.

### Draw-phase-first rewrite

Deferred. A full pass-accurate render split is the strongest long-term parity model, but it is larger than needed for the first implementation. The chosen design keeps phase-aware buffers so draw order can be refined incrementally.

### Keep only owner-level rally point

Rejected for parity. The verified rally renderer reads a selected building's rally target. Owner-level rally state can remain as a production fallback, but selected-building rally visuals need per-producer state.

