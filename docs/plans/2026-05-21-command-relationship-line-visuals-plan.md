# Command And Relationship Line Visuals Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Implement RA2/YR battlefield line visuals as distinct parity systems while sharing only low-level raster helpers.

**Architecture:** The line visuals are split between deterministic rally target state in `sim/` and presentation-only overlay construction in the app/render layer. `sim/` stores per-producer rally targets and applies command contracts; app/render reads simulation state and builds selected action and rally line sprites without mutating gameplay state.

**Design Doc:** `docs/plans/2026-05-21-command-relationship-line-visuals-design.md`

---

## Grounding Summary

- Existing docs identify five line families: selected action lines, factory rally lines, planning waypoint lines, Psychic Sensor enemy action lines, and mind-control link lines.
- Selected action lines are verified at `TechnoClass::DrawActionLines @ 0x004DC060` and `ActionLines::DrawLine @ 0x007049C0`; they use a 25-frame timer, endpoint boxes, and one solid line in stock YR.
- Factory rally visuals are verified at `FUN_006DA9D0`; they read per-building rally target state, not only owner-level house rally state.
- Planning waypoint visuals are verified at `FUN_006DAD60`; they need `WaypointPathClass`-equivalent state and are outside this first implementation sequence.
- Psychic Sensor enemy lines are verified at `DrawRadarActionLines @ 0x004DC340`; they are tactical-screen lines gated by local Psychic Detection coverage.
- Mind-control links are verified at `CaptureManagerClass::DrawLinks @ 0x00472160` and `ShouldDrawLinks @ 0x00472640`; they need CaptureManager/MCNode-equivalent state.
- Current Rust has `src/app_target_lines.rs`, a command-recorded approximation that stores endpoints in app state and draws one overlay buffer named `"target_lines"`.
- Current Rust has owner-level `HouseState.rally_point`, used by production spawn movement, but no per-producer `GameEntity.rally_target`.
- `GameEntity.selected` is documented as app-owned presentation/input state, so this plan extends `Command::SetRally` to carry selected producer IDs instead of making sim infer producers from selection.
- `ObjectType` already parses `Factory=`, `UnitRepair=`, and `PsychicDetectionRadius=`, but does not parse `Cloning=`.
- INI keys driving this work include `rulesmd.ini` `Factory=InfantryType`, `Factory=UnitType`, `UnitRepair=yes`, `Cloning=yes`, `MaxWaypointPathLength=15`, `WaypointAnimationSpeed=10`, `PsychicDetectionRadius=15`, `MindControlAttackLineFrames=20`, and `artmd.ini [MIND] AlternateFLH0..4`.
- The exact selected-line source coordinate should eventually be weapon/fire coords; current Rust can start from entity screen position until a fire-coordinate helper exists.
- The exact rally line palette shade should be verified by `/review-plan` because Rust currently exposes house color ramps rather than the original surface RGB lookup.

## Key Technical Decisions

- Use family-specific builders under the existing app target-line module first: this preserves the current integration point while avoiding a broad rename. **Confidence:** high
  - **Source:** design doc, current `src/app_target_lines.rs`, current `src/app_render/build_instances.rs`
- Add `GameEntity.rally_target: Option<(u16, u16)>` as deterministic state. **Confidence:** high
  - **Source:** `FACTORY_RALLY_POINT_LINE_CALLER_COLOR_GATE_GHIDRA_REPORT.md`, Ghidra `FUN_006DA9D0`
- Preserve `HouseState.rally_point` as owner-level production fallback in the first pass. **Confidence:** high
  - **Source:** current `src/sim/production/production_queue.rs`, design doc
- Extend `Command::SetRally` with `producer_ids: Vec<u64>` so command replay/network application does not depend on app-local selected state. **Confidence:** medium
  - **Source:** current `GameEntity.selected` comment in `src/sim/game_entity.rs`; inferred architecture correction
- Add `ObjectType.cloning` and a rally eligibility helper using `Factory=InfantryType`, `Factory=UnitType`, `UnitRepair=yes`, or `Cloning=yes`. **Confidence:** high
  - **Source:** Ghidra `0x00455DA0`, `FACTORY_RALLY_POINT_LINE_CALLER_COLOR_GATE_GHIDRA_REPORT.md`, `ini/rulesmd.ini`
- Replace command-recorded selected line endpoints with live simulation state resolution. **Confidence:** high
  - **Source:** `SELECTED_UNIT_ACTION_TARGET_LINE_BASELINE_GHIDRA_REPORT.md`, Ghidra `0x004DC060`
- Use current entity position as the first selected-line source point, isolated behind a helper that can switch to fire coords. **Confidence:** low
  - **Source:** inferred from current repo capability; flag for `/review-plan`
- Draw selected action lines with two 3x3 endpoint boxes and one solid line. **Confidence:** high
  - **Source:** `ACTIONLINES_DRAWLINE_007049C0_PIXEL_STYLE_GHIDRA_REPORT.md`, Ghidra `0x007049C0`
- Implement rally first/second buffers now, even if both initially share the same builder, so draw order can be refined without changing the public overlay contract. **Confidence:** medium
  - **Source:** `FACTORY_RALLY_POINT_LINE_CALLER_COLOR_GATE_GHIDRA_REPORT.md`, current render pool pattern

Low-confidence decisions that need `/review-plan` verification before code execution:

- Selected action line source coordinate uses entity screen position until a fire/FLH coordinate helper exists.
- Rally line tint uses the current Rust house-color ramp; exact original palette shade/RGB should be checked against the binary report and renderer constraints.

## Open Questions

### Resolved During Planning

- **Should rally visuals use owner-level `HouseState.rally_point` only?** No. Binary rally rendering reads per-producer state, so `GameEntity.rally_target` is required.
- **Should `sim/` read `GameEntity.selected` to decide which producers receive rally targets?** No for this plan. The command should carry producer IDs, keeping replay/network behavior explicit.
- **Should Psychic Sensor lines be controlled by `UnitActionLines`?** No. They are Psychic Detection gated and separate from selected action lines.
- **Should selected unit line thickness be two parallel lines?** No. The corrected evidence says endpoint boxes plus one solid line.

### Deferred to Implementation

- **Exact selected-line source coordinate:** Execution can start with `entity.position.screen_x/screen_y`, but parity review should replace it with a fire-coordinate helper when that state exists.
- **Exact rally tint shade:** Implementation should use existing house color data first; `/review-plan` should verify the original surface RGB mapping before visual acceptance.
- **Planning, Psychic, and mind-control line execution:** The plan records their contracts, but first execution covers selected action lines and factory rally lines because the required planning path, Psychic Detection coverage, and CaptureManager state are not present.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/object_type.rs` | Parse `Cloning=yes` and expose rally-line eligibility from rules data. |
| Modify | `src/sim/game_entity.rs` | Store per-producer rally target state on entities. |
| Modify | `src/sim/world/world_hash.rs` | Include `rally_target` in deterministic state hash. |
| Modify | `src/sim/command.rs` | Extend `Command::SetRally` with explicit producer IDs. |
| Modify | `src/app_context_order.rs` | Populate `SetRally.producer_ids` from selected local structures. |
| Modify | `src/sim/world/world_commands.rs` | Apply owner fallback rally and per-producer rally targets. |
| Modify | `src/sim/replay.rs` | Update replay round-trip fixture for the command contract. |
| Modify | `src/app_target_lines.rs` | Split current approximation into timer state, family builders, and raster helpers. |
| Modify | `src/app_render/build_instances.rs` | Build grouped selected-action and rally line instances. |
| Modify | `src/app_render/mod.rs` | Upload new rally line buffers. |
| Modify | `src/app_render/draw_passes.rs` | Draw selected action and factory rally buffers in explicit order. |

## Interface Changes

- `ObjectType` gains `pub cloning: bool`.
- `ObjectType` gains `pub fn has_rally_line(&self) -> bool`.
- `GameEntity` gains `#[serde(default)] pub rally_target: Option<(u16, u16)>`.
- `Command::SetRally` changes from `{ owner, rx, ry }` to `{ owner, rx, ry, producer_ids }`.
- `TargetLineState` remains app-owned, but stores timer and option state rather than endpoint records.
- `UiInstances` gains factory rally vectors or a grouped line overlay field, depending on the smallest compatible change in `build_instances.rs`.
- Render pool gains `"factory_rally_first"` and `"factory_rally_second"` buffers.

## Sim Checklist

- [ ] All new sim math is integer/cell coordinate only; no `f32`/`f64` in `sim/`.
- [ ] `GameEntity.rally_target` is included in deterministic state hash.
- [ ] `sim/` changes do not import `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`.
- [ ] Tick ordering is unchanged: `SetRally` applies during normal command dispatch.
- [ ] `producer_ids` are sorted/deduplicated before sim mutation so duplicate IDs cannot make order-dependent behavior.
- [ ] Entity iteration order remains deterministic through `BTreeMap`.

## Risk Areas

- `Command::SetRally` contract changes affect replay JSON and the one app call site in `app_context_order.rs`.
- Adding a field to `ObjectType` can break test fixtures that construct `ObjectType` directly.
- Adding a field to `GameEntity` can break default construction and snapshot round trips if `serde(default)` is omitted.
- Target-line render changes touch dirty files in the current worktree; implementer must preserve unrelated edits.
- Selected-line source coordinate and rally tint are visible parity details with known uncertainty.
- Draw order must remain explicit because selected action lines, rally lines, selection brackets, and radius rings are separate player-visible overlays.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 4 | `SetRally` producer IDs | Player expects only selected eligible producers to show the new rally target | Unit test with selected war factory, barracks, non-factory, and enemy structure IDs |
| 5 | Selected line timer | Click feedback must disappear after the original 25-frame window | Unit test at ticks 24 and 25; compare to `0x004DC060` |
| 6 | Selected endpoint boxes + solid line | The corrected evidence says one solid line with endpoint boxes, not double thickness | Unit test counts 3x3 endpoint pixels; visual side-by-side |
| 7 | Live selected endpoint resolution | Lines should follow current attack/move state, not stale recorded click data | Unit test attack target priority over movement target |
| 8 | Rally eligibility | Non-producer buildings must not show rally lines; cloning vats and repair depots can | Unit tests for `Factory=`, `UnitRepair=`, and `Cloning=` |
| 9 | Rally owner color and phase | Rally lines are player-color tactical feedback and animate in stock YR | Visual check against selected factory in gamemd.exe |
| 10 | Draw order | Lines should not bury or cover selection brackets incorrectly | In-game screenshot check with selected factory and selected units |

---

## Tasks

### Task 1: Parse Cloning And Add Rally Eligibility

**Why:** Rally line eligibility depends on `Cloning=yes`, but current rules parsing does not expose it.

**Files:**
- Modify: `src/rules/object_type.rs`

**Pattern:** Existing bool parser fields such as `unit_repair`, `sensor_array`, and `psychic_detection_radius`.

**Step 1: Add the field near production/building capability fields**
```rust
/// Whether this building clones produced infantry (Cloning=yes in rules.ini).
pub cloning: bool,
```

**Step 2: Parse the INI key in `ObjectType::from_ini_section`**
```rust
cloning: section.get_bool("Cloning").unwrap_or(false),
```

Place it near `factory` or `unit_repair` so production/rally capability fields stay grouped.

**Step 3: Add the eligibility helper**
```rust
impl ObjectType {
    pub fn has_rally_line(&self) -> bool {
        matches!(
            self.factory,
            Some(FactoryType::InfantryType | FactoryType::UnitType)
        ) || self.unit_repair
            || self.cloning
    }
}
```

**Step 4: Add parser/helper tests in `src/rules/object_type.rs`**
```rust
#[test]
fn cloning_key_parses_and_participates_in_rally_lines() {
    let ini = IniFile::from_str("[YACLON]\nName=Cloning Vats\nStrength=1000\nCloning=yes\n");
    let section = ini.section("YACLON").unwrap();
    let obj = ObjectType::from_ini_section("YACLON", section, ObjectCategory::Building);
    assert!(obj.cloning);
    assert!(obj.has_rally_line());
}

#[test]
fn rally_line_accepts_infantry_vehicle_factories_and_repair() {
    let ini = IniFile::from_str(
        "[GAPILE]\nFactory=InfantryType\n\
         [GAWEAP]\nFactory=UnitType\n\
         [GADEPT]\nUnitRepair=yes\n",
    );
    let barracks = ObjectType::from_ini_section(
        "GAPILE",
        ini.section("GAPILE").unwrap(),
        ObjectCategory::Building,
    );
    let factory = ObjectType::from_ini_section(
        "GAWEAP",
        ini.section("GAWEAP").unwrap(),
        ObjectCategory::Building,
    );
    let depot = ObjectType::from_ini_section(
        "GADEPT",
        ini.section("GADEPT").unwrap(),
        ObjectCategory::Building,
    );
    assert!(barracks.has_rally_line());
    assert!(factory.has_rally_line());
    assert!(depot.has_rally_line());
}
```

**Step 5: Verify**

Run: `cargo test object_type:: -- --nocapture`

Expected: the new cloning and rally helper tests pass.

**Step 6: Commit**

Commit message: `rules: parse cloning for rally line eligibility`

### Task 2: Add Per-Producer Rally Target State

**Why:** The binary rally renderer reads producer-owned target state, so the sim needs entity-level rally state.

**Files:**
- Modify: `src/sim/game_entity.rs`
- Modify: `src/sim/world/world_hash.rs`
- Modify: `src/sim/snapshot.rs` if snapshot round-trip expectations need a field assertion

**Pattern:** Existing optional entity state fields such as `capture_target`, `c4_plant`, and `deploy_state`.

**Step 1: Add the field to `GameEntity`**
```rust
/// Per-producer rally target cell. Used by selected factory rally visuals.
/// Owner-level `HouseState.rally_point` remains the production fallback.
#[serde(default)]
pub rally_target: Option<(u16, u16)>,
```

Place it near `type_ref/category` or near production-related building fields, not among render-only fields.

**Step 2: Initialize in `GameEntity::new`**
```rust
rally_target: None,
```

**Step 3: Hash the field in `Simulation::hash_entities`**
```rust
if let Some((rx, ry)) = entity.rally_target {
    1u8.hash(hasher);
    rx.hash(hasher);
    ry.hash(hasher);
} else {
    0u8.hash(hasher);
}
```

Put this near other high-level entity intent fields such as movement/attack/capture target.

**Step 4: Add tests**
```rust
#[test]
fn new_entity_has_no_rally_target() {
    let e = test_entity();
    assert_eq!(e.rally_target, None);
}

#[test]
fn entity_rally_target_changes_state_hash() {
    let mut sim_a = test_sim();
    let mut sim_b = sim_a.clone();
    sim_b.entities.get_mut(1).unwrap().rally_target = Some((30, 31));
    assert_ne!(sim_a.state_hash(), sim_b.state_hash());
}
```

Use existing local helpers in `game_entity.rs` and `world_hash.rs`; do not introduce broad test scaffolding.

**Step 5: Verify**

Run: `cargo test new_entity_has_no_rally_target entity_rally_target_changes_state_hash -- --nocapture`

Expected: both tests pass.

**Step 6: Commit**

Commit message: `sim: store producer rally target on entities`

### Task 3: Extend The SetRally Command Contract

**Why:** The command must say which producers receive the rally target; relying on app-local selection inside sim would make deterministic behavior ambiguous.

**Files:**
- Modify: `src/sim/command.rs`
- Modify: `src/app_context_order.rs`
- Modify: `src/sim/replay.rs`

**Pattern:** Existing command payloads that carry concrete entity IDs, such as `Move`, `Attack`, `SellBuilding`, and `ToggleRepair`.

**Step 1: Change the enum variant**
```rust
SetRally {
    owner: InternedId,
    rx: u16,
    ry: u16,
    producer_ids: Vec<u64>,
},
```

**Step 2: Update the app call site**

In `src/app_context_order.rs`, build `producer_ids` from selected structures owned by `struct_owner_id`. Filter to structures only; eligibility is rechecked in sim with rules.

```rust
let mut producer_ids: Vec<u64> = selected_ids
    .iter()
    .copied()
    .filter(|stable_id| {
        sim.entities.get(*stable_id).is_some_and(|entity| {
            entity.category == EntityCategory::Structure && entity.owner == struct_owner_id
        })
    })
    .collect();
producer_ids.sort_unstable();
producer_ids.dedup();
```

Then pass `producer_ids` into `Command::SetRally`.

**Step 3: Update replay fixture**
```rust
Command::SetRally {
    owner: crate::sim::intern::test_intern("Americans"),
    rx: 10,
    ry: 11,
    producer_ids: vec![1, 2],
}
```

**Step 4: Verify compile points**

Run: `cargo test test_replay_json_roundtrip -- --nocapture`

Expected: replay command serialization round-trips with `producer_ids`.

**Step 5: Commit**

Commit message: `sim: carry producer ids in rally commands`

### Task 4: Apply Per-Producer Rally Targets

**Why:** The command contract exists; sim now needs to mutate the correct producer entities and preserve the owner fallback rally point.

**Files:**
- Modify: `src/sim/world/world_commands.rs`

**Pattern:** Existing command dispatch validation in `apply_command`, especially owner checks and rules lookup before mutation.

**Step 1: Update the `SetRally` match arm**
```rust
Command::SetRally {
    owner,
    rx,
    ry,
    producer_ids,
} => {
    production::set_rally_point_for_owner(self, owner, *rx, *ry);
    self.set_rally_target_for_producers(command_owner, producer_ids, *rx, *ry, rules)
}
```

**Step 2: Add a helper on `Simulation` in `world_commands.rs`**
```rust
fn set_rally_target_for_producers(
    &mut self,
    command_owner: &str,
    producer_ids: &[u64],
    rx: u16,
    ry: u16,
    rules: Option<&RuleSet>,
) -> bool {
    let Some(rules) = rules else {
        return true;
    };
    let mut ids = producer_ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    for stable_id in ids {
        let eligible = self.entities.get(stable_id).is_some_and(|entity| {
            entity.category == crate::map::entities::EntityCategory::Structure
                && self.interner.resolve(entity.owner) == command_owner
                && rules
                    .object(self.interner.resolve(entity.type_ref))
                    .is_some_and(|obj| obj.has_rally_line())
        });
        if eligible {
            if let Some(entity) = self.entities.get_mut(stable_id) {
                entity.rally_target = Some((rx, ry));
            }
        }
    }
    true
}
```

**Step 3: Add unit tests**

Create a test in the existing `world_commands.rs` test module or the closest command test module:

```rust
#[test]
fn set_rally_updates_only_owned_eligible_producers() {
    let mut sim = command_test_sim_with_structures();
    let owner = sim.interner.intern("Americans");
    let command = Command::SetRally {
        owner,
        rx: 40,
        ry: 41,
        producer_ids: vec![3, 2, 2, 4, 5],
    };
    assert!(sim.apply_command("Americans", &command, Some(&test_rules()), None, &BTreeMap::new()));
    assert_eq!(sim.entities.get(2).unwrap().rally_target, Some((40, 41)));
    assert_eq!(sim.entities.get(3).unwrap().rally_target, Some((40, 41)));
    assert_eq!(sim.entities.get(4).unwrap().rally_target, None);
    assert_eq!(sim.entities.get(5).unwrap().rally_target, None);
    assert_eq!(sim.houses.get(&owner).unwrap().rally_point, Some((40, 41)));
}
```

Use local test constructors already present in the file; keep test rules minimal with one `Factory=UnitType`, one `Factory=InfantryType`, one non-factory building, and one enemy producer.

**Step 4: Verify**

Run: `cargo test set_rally_updates_only_owned_eligible_producers -- --nocapture`

Expected: only owned eligible producers get `rally_target`; owner fallback still updates.

**Step 5: Commit**

Commit message: `sim: apply rally target to selected producers`

### Task 5: Reshape TargetLineState Into Timer And Option State

**Why:** Selected action lines should read live sim state; app state should only control the short feedback timer and user option gate.

**Files:**
- Modify: `src/app_target_lines.rs`

**Pattern:** Existing `TargetLineState` on `AppState` and `record_command_lines` call from `app_context_order.rs`.

**Step 1: Remove stored endpoint records**

Replace `TargetLineState` with:

```rust
#[derive(Debug, Clone)]
pub(crate) struct TargetLineState {
    start_tick: Option<u64>,
    unit_action_lines_enabled: bool,
}

impl Default for TargetLineState {
    fn default() -> Self {
        Self {
            start_tick: None,
            unit_action_lines_enabled: true,
        }
    }
}
```

**Step 2: Keep `record_command_lines` as a timer trigger**
```rust
pub(crate) fn record_command_lines(
    state: &mut TargetLineState,
    commands: &[CommandEnvelope],
    current_tick: u64,
) {
    let any_action_line_command = commands.iter().any(|envelope| {
        matches!(
            envelope.payload,
            Command::Move { .. }
                | Command::AttackMove { .. }
                | Command::Attack { .. }
                | Command::ForceAttack { .. }
                | Command::ForceAttackCell { .. }
        )
    });
    if any_action_line_command {
        state.start_tick = Some(current_tick);
    }
}
```

**Step 3: Add helper methods**
```rust
impl TargetLineState {
    pub(crate) fn is_selected_action_active(&self, tick: u64) -> bool {
        self.unit_action_lines_enabled
            && self
                .start_tick
                .is_some_and(|start| tick.saturating_sub(start) < DURATION_TICKS)
    }

    pub(crate) fn set_unit_action_lines_enabled(&mut self, enabled: bool) {
        self.unit_action_lines_enabled = enabled;
    }
}
```

**Step 4: Add tests**
```rust
#[test]
fn selected_action_timer_expires_at_25_ticks() {
    let mut state = TargetLineState::default();
    state.start_tick = Some(100);
    assert!(state.is_selected_action_active(124));
    assert!(!state.is_selected_action_active(125));
}

#[test]
fn unit_action_lines_option_disables_selected_timer() {
    let mut state = TargetLineState::default();
    state.start_tick = Some(100);
    state.set_unit_action_lines_enabled(false);
    assert!(!state.is_selected_action_active(101));
}
```

**Step 5: Verify**

Run: `cargo test selected_action_timer unit_action_lines_option -- --nocapture`

Expected: timer behavior matches the 25-frame binary gate.

**Step 6: Commit**

Commit message: `app: make target line state timer-only`

### Task 6: Add Shared Raster Helpers For Line Families

**Why:** Families need shared pixel emission, but gates and state resolution must remain separate.

**Files:**
- Modify: `src/app_target_lines.rs`

**Pattern:** Current `emit_colored_line` and `project_cell_destination` helpers.

**Step 1: Define a point type and constants**
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
struct ScreenPoint {
    x: f32,
    y: f32,
}

const LINE_DEPTH: f32 = 0.0005;
const ENDPOINT_BOX_RADIUS: i32 = 1;
```

**Step 2: Add endpoint-box emission**
```rust
fn emit_endpoint_box(instances: &mut Vec<SpriteInstance>, point: ScreenPoint, tint: [f32; 3]) {
    for dy in -ENDPOINT_BOX_RADIUS..=ENDPOINT_BOX_RADIUS {
        for dx in -ENDPOINT_BOX_RADIUS..=ENDPOINT_BOX_RADIUS {
            push_line_pixel(instances, point.x + dx as f32, point.y + dy as f32, tint);
        }
    }
}
```

**Step 3: Rename current line helper to solid body helper**
```rust
fn emit_solid_line(
    instances: &mut Vec<SpriteInstance>,
    start: ScreenPoint,
    end: ScreenPoint,
    tint: [f32; 3],
) {
    // Move the existing DDA stepping body here.
}
```

**Step 4: Add one selected action helper**
```rust
fn emit_selected_action_line(
    instances: &mut Vec<SpriteInstance>,
    start: ScreenPoint,
    end: ScreenPoint,
    tint: [f32; 3],
) {
    emit_endpoint_box(instances, start, tint);
    emit_endpoint_box(instances, end, tint);
    emit_solid_line(instances, start, end, tint);
}
```

**Step 5: Add tests with direct helper output**
```rust
#[test]
fn selected_action_line_emits_endpoint_boxes() {
    let mut instances = Vec::new();
    emit_selected_action_line(
        &mut instances,
        ScreenPoint { x: 10.0, y: 10.0 },
        ScreenPoint { x: 20.0, y: 10.0 },
        MOVE_COLOR,
    );
    assert!(instances.len() >= 18);
}
```

**Step 6: Verify**

Run: `cargo test selected_action_line_emits_endpoint_boxes -- --nocapture`

Expected: endpoint-box helper emits at least two 3x3 endpoint boxes plus body pixels.

**Step 7: Commit**

Commit message: `app: add raster helpers for action lines`

### Task 7: Build Selected Action Lines From Live Simulation State

**Why:** The current app approximation records clicked endpoints; the original reads live target/navigation state while the timer is active.

**Files:**
- Modify: `src/app_target_lines.rs`

**Pattern:** Current builder reads `Simulation`, `height_map`, and selected entities.

**Step 1: Add a selected line endpoint model**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedLineKind {
    Move,
    Attack,
}

struct SelectedActionLine {
    start: ScreenPoint,
    end: ScreenPoint,
    kind: SelectedLineKind,
}
```

**Step 2: Resolve attack before movement**
```rust
fn selected_action_line_for_entity(
    entity: &GameEntity,
    sim: &Simulation,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> Option<SelectedActionLine> {
    if !entity.selected || entity.category == EntityCategory::Structure {
        return None;
    }
    let start = ScreenPoint {
        x: entity.position.screen_x,
        y: entity.position.screen_y,
    };
    if let Some(attack) = &entity.attack_target {
        let end = resolve_attack_target_point(attack, sim, height_map)?;
        return Some(SelectedActionLine {
            start,
            end,
            kind: SelectedLineKind::Attack,
        });
    }
    let movement = entity.movement_target.as_ref()?;
    let (rx, ry) = movement
        .final_goal
        .or_else(|| movement.path.last().copied())?;
    Some(SelectedActionLine {
        start,
        end: project_cell_destination(rx, ry, height_map, None, Some(sim)).into(),
        kind: SelectedLineKind::Move,
    })
}
```

Adjust field names if `MovementTarget.final_goal` is represented differently in current code.

**Step 3: Implement target point resolution**
```rust
fn resolve_attack_target_point(
    attack: &AttackTarget,
    sim: &Simulation,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> Option<ScreenPoint> {
    match attack.target {
        TargetRef::Entity(target_id) => sim.entities.get(target_id).map(|target| ScreenPoint {
            x: target.position.screen_x,
            y: target.position.screen_y,
        }),
        TargetRef::Cell { rx, ry } => {
            let (x, y) = project_cell_destination(rx, ry, height_map, None, Some(sim));
            Some(ScreenPoint { x, y })
        }
    }
}
```

Use the actual `AttackTarget` target enum names from `src/sim/combat/mod.rs`.

**Step 4: Replace `build_target_line_instances` body**
```rust
pub(crate) fn build_target_line_instances(
    line_state: &TargetLineState,
    sim: Option<&Simulation>,
    height_map: &BTreeMap<(u16, u16), u8>,
) -> Vec<SpriteInstance> {
    let Some(sim) = sim else {
        return Vec::new();
    };
    if !line_state.is_selected_action_active(sim.tick) {
        return Vec::new();
    }
    let mut instances = Vec::new();
    for entity in sim.entities.values() {
        let Some(line) = selected_action_line_for_entity(entity, sim, height_map) else {
            continue;
        };
        let tint = match line.kind {
            SelectedLineKind::Attack => ATTACK_COLOR,
            SelectedLineKind::Move => MOVE_COLOR,
        };
        emit_selected_action_line(&mut instances, line.start, line.end, tint);
    }
    instances
}
```

**Step 5: Add tests**
```rust
#[test]
fn selected_action_attack_target_wins_over_movement() {
    let sim = sim_with_selected_unit_that_has_attack_and_move();
    let lines = build_target_line_instances(
        &active_line_state_for_tick(sim.tick),
        Some(&sim),
        &BTreeMap::new(),
    );
    assert!(!lines.is_empty());
    assert!(line_pixels_touch_attack_target(&lines));
}
```

Use small local helpers to avoid broad fixture churn.

**Step 6: Verify**

Run: `cargo test selected_action_attack_target_wins_over_movement -- --nocapture`

Expected: selected action builder emits the attack line when both attack and movement state exist.

**Step 7: Commit**

Commit message: `app: resolve selected action lines from live sim state`

### Task 8: Build Factory Rally Line Instances

**Why:** Selected factories need a separate line family that reads `GameEntity.rally_target` and uses owner color.

**Files:**
- Modify: `src/app_target_lines.rs`

**Pattern:** Existing app-layer render builders that read `Simulation`, `rules`, `house_color_map`, and `height_map` without mutating sim.

**Step 1: Add a public builder**
```rust
pub(crate) fn build_factory_rally_line_instances(
    sim: Option<&Simulation>,
    rules: Option<&RuleSet>,
    height_map: &BTreeMap<(u16, u16), u8>,
    house_color_map: &BTreeMap<String, HouseColorIndex>,
    local_owner: Option<&str>,
) -> Vec<SpriteInstance> {
    let (Some(sim), Some(rules), Some(local_owner)) = (sim, rules, local_owner) else {
        return Vec::new();
    };
    let mut instances = Vec::new();
    for entity in sim.entities.values() {
        if !entity.selected || entity.category != EntityCategory::Structure {
            continue;
        }
        if sim.interner.resolve(entity.owner) != local_owner {
            continue;
        }
        let Some((rx, ry)) = entity.rally_target else {
            continue;
        };
        let Some(obj) = rules.object(sim.interner.resolve(entity.type_ref)) else {
            continue;
        };
        if !obj.has_rally_line() {
            continue;
        }
        let start = ScreenPoint {
            x: entity.position.screen_x,
            y: entity.position.screen_y,
        };
        let (x, y) = project_cell_destination(rx, ry, height_map, None, Some(sim));
        let tint = rally_tint_for_owner(sim.interner.resolve(entity.owner), house_color_map);
        emit_rally_line(&mut instances, start, ScreenPoint { x, y }, tint, sim.tick);
    }
    instances
}
```

**Step 2: Add owner tint conversion**
```rust
fn rally_tint_for_owner(
    owner: &str,
    house_color_map: &BTreeMap<String, HouseColorIndex>,
) -> [f32; 3] {
    let index = house_color_map.get(owner).copied().unwrap_or_default();
    let color = crate::rules::house_colors::house_color_ramp(index)[0];
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
    ]
}
```

Flag this for visual review because exact original surface RGB may use a different shade.

**Step 3: Add rally line emission with phase hook**
```rust
fn emit_rally_line(
    instances: &mut Vec<SpriteInstance>,
    start: ScreenPoint,
    end: ScreenPoint,
    tint: [f32; 3],
    tick: u64,
) {
    let _phase = (0x7fff_ffffu64.saturating_sub(tick)) % 15;
    emit_solid_line(instances, start, end, tint);
}
```

Keep the phase parameter in the helper even if the first sprite emitter is solid; `/review-plan` should decide whether to add the exact `DAT_00842930` pattern immediately.

**Step 4: Add tests**
```rust
#[test]
fn factory_rally_builder_emits_only_selected_local_eligible_structures() {
    let sim = sim_with_selected_factory_and_non_factory();
    let rules = rules_with_factory_and_non_factory();
    let lines = build_factory_rally_line_instances(
        Some(&sim),
        Some(&rules),
        &BTreeMap::new(),
        &test_house_colors(),
        Some("Americans"),
    );
    assert!(!lines.is_empty());
}
```

**Step 5: Verify**

Run: `cargo test factory_rally_builder_emits_only_selected_local_eligible_structures -- --nocapture`

Expected: selected local eligible producer emits a rally line; selected non-factory does not.

**Step 6: Commit**

Commit message: `app: build factory rally line instances`

### Task 9: Wire Rally Buffers Into UI Instance Build And Render Upload

**Why:** Rally lines need explicit buffers so draw order can be tuned separately from selected action lines.

**Files:**
- Modify: `src/app_render/build_instances.rs`
- Modify: `src/app_render/mod.rs`
- Modify: `src/app_render/draw_passes.rs`

**Pattern:** Existing `UiInstances.target_line` field, `pool.upload`, and `draw_pooled_no_depth` calls.

**Step 1: Extend `UiInstances`**
```rust
pub(super) struct UiInstances {
    // existing fields...
    pub target_line: Vec<SpriteInstance>,
    pub factory_rally_first: Vec<SpriteInstance>,
    pub factory_rally_second: Vec<SpriteInstance>,
}
```

If `UiInstances` is defined in a different part of `build_instances.rs`, add the fields there and initialize them in the constructor.

**Step 2: Build rally instances**
```rust
let factory_rally = crate::app_target_lines::build_factory_rally_line_instances(
    state.simulation.as_ref(),
    state.rules.as_ref(),
    &state.height_map,
    &state.house_color_map,
    crate::app_commands::preferred_local_owner(state).as_deref(),
);
```

Then set:

```rust
factory_rally_first: factory_rally.clone(),
factory_rally_second: factory_rally,
```

This clone is acceptable in UI instance build because it happens once per frame for a small overlay vector. If profiling shows this vector grows large, replace it with two builder calls or a shared draw pass.

**Step 3: Upload buffers**
```rust
pool.upload(&state.gpu, "factory_rally_first", &ui.factory_rally_first);
pool.upload(&state.gpu, "factory_rally_second", &ui.factory_rally_second);
```

Place them next to `"target_lines"` in `src/app_render/mod.rs`.

**Step 4: Draw buffers explicitly**
```rust
draw_pooled_no_depth(
    &mut pass,
    &state.batch_renderer,
    pool,
    bracket_tex,
    "factory_rally_first",
);
draw_pooled_no_depth(
    &mut pass,
    &state.batch_renderer,
    pool,
    bracket_tex,
    "target_lines",
);
draw_pooled_no_depth(
    &mut pass,
    &state.batch_renderer,
    pool,
    bracket_tex,
    "factory_rally_second",
);
```

Start in Step 10 UI overlay near existing `"target_lines"`; visual verification can move one pass earlier if the original two-pass tactical draw requires it.

**Step 5: Verify compile**

Run: `cargo test -q app_target_lines -- --nocapture`

Expected: target-line tests still pass and UI instance struct compiles.

**Step 6: Commit**

Commit message: `render: add factory rally line buffers`

### Task 10: Add Focused Regression Tests For Command And Overlay Integration

**Why:** The highest-risk behavior crosses app command construction, sim mutation, and render builders.

**Files:**
- Modify: `src/app_context_order.rs` tests if present, or add tests near existing context-order tests
- Modify: `src/app_target_lines.rs`
- Modify: `src/sim/world/world_commands.rs`

**Pattern:** Existing module-local `#[cfg(test)] mod tests` blocks.

**Step 1: Test app command construction**
```rust
#[test]
fn right_click_structure_selection_sends_rally_producer_ids() {
    let queued = issue_test_right_click_with_selected_factory_and_tank();
    let rally = queued
        .iter()
        .find_map(|envelope| match &envelope.payload {
            Command::SetRally { producer_ids, .. } => Some(producer_ids),
            _ => None,
        })
        .expect("rally command queued");
    assert_eq!(rally, &vec![selected_factory_id()]);
}
```

Use existing context-order helpers if available; otherwise keep this as a small app-level fixture.

**Step 2: Test `UnitActionLines` gate does not affect rally builder**
```rust
#[test]
fn disabling_unit_action_lines_does_not_disable_rally_lines() {
    let mut state = TargetLineState::default();
    state.set_unit_action_lines_enabled(false);
    let rally = build_factory_rally_line_instances(
        Some(&sim_with_selected_factory_rally()),
        Some(&rules_with_vehicle_factory()),
        &BTreeMap::new(),
        &test_house_colors(),
        Some("Americans"),
    );
    assert!(!rally.is_empty());
    assert!(build_target_line_instances(&state, Some(&sim_with_selected_unit_move()), &BTreeMap::new()).is_empty());
}
```

**Step 3: Test missing data skips safely**
```rust
#[test]
fn line_builders_skip_when_sim_or_rules_missing() {
    assert!(build_target_line_instances(&TargetLineState::default(), None, &BTreeMap::new()).is_empty());
    assert!(build_factory_rally_line_instances(None, None, &BTreeMap::new(), &BTreeMap::new(), Some("Americans")).is_empty());
}
```

**Step 4: Verify**

Run: `cargo test right_click_structure_selection_sends_rally_producer_ids disabling_unit_action_lines_does_not_disable_rally_lines line_builders_skip_when_sim_or_rules_missing -- --nocapture`

Expected: integration behavior matches the family gates.

**Step 5: Commit**

Commit message: `test: cover command and overlay line integration`

### Task 11: Run Full Checks And Visual Verification

**Why:** This feature is mostly player-visible, so compile success is not enough.

**Files:**
- No source edits unless verification reveals a defect in the changed files.

**Pattern:** Existing repo cargo/test workflow and in-game visual comparison.

**Step 1: Format**

Run: `cargo fmt`

Expected: no formatting diff outside touched files unless existing dirty worktree already contains unformatted code.

**Step 2: Run focused tests**

Run:
```powershell
cargo test object_type:: rally_target set_rally selected_action factory_rally -- --nocapture
```

Expected: all focused tests pass.

**Step 3: Run broader compile/test pass**

Run:
```powershell
cargo test -q
```

Expected: pass, except unrelated dirty-worktree failures must be reported with exact failing test names and file paths.

**Step 4: Visual check selected action lines**

Run the game, select a mobile unit, right-click move, then attack a target.

Expected:

- move click shows a short-lived green selected action line
- attack click uses attack color
- line disappears after roughly the 25-frame window
- endpoint boxes are visible at both ends
- selected buildings do not show selected action lines

**Step 5: Visual check factory rally lines**

Run the game, select a barracks or war factory, set a rally point, and keep the producer selected.

Expected:

- selected eligible producer shows an owner-colored rally line to the rally target
- selected non-producer building does not show a rally line
- mixed selection of factory plus mobile units sets rally and still moves mobile units
- `UnitActionLines` disabled hides selected action lines only; rally line still appears

**Step 6: Commit**

Commit message: `verify command relationship line visuals`

## Sources & References

- **Design doc:** `docs/plans/2026-05-21-command-relationship-line-visuals-design.md`
- **Selected action line docs:** `docs/research/SELECTED_UNIT_ACTION_TARGET_LINE_BASELINE_GHIDRA_REPORT.md`, `docs/research/ACTIONLINES_DRAWLINE_007049C0_PIXEL_STYLE_GHIDRA_REPORT.md`, `docs/research/UNITACTIONLINES_OPTION_RENDERPASS_GATE_GHIDRA_REPORT.md`, `docs/research/TECHNOCLASS_DRAWACTIONLINES_004DC060_GHIDRA_REPORT.md`
- **Factory rally docs:** `docs/research/FACTORY_RALLY_POINT_LINE_CALLER_COLOR_GATE_GHIDRA_REPORT.md`, `docs/research/PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md`
- **Planning docs:** `docs/research/PLANNING_QUEUED_WAYPOINT_LINES_AND_FLAGS_GHIDRA_REPORT.md`
- **Psychic Sensor docs:** `docs/research/PSYCHIC_SENSOR_ENEMY_ACTION_LINES_RECHECK_GHIDRA_REPORT.md`, `docs/research/DRAWRADARACTIONLINES_004DC340_ENEMY_LINES_GHIDRA_REPORT.md`
- **Mind-control docs:** `docs/research/MIND_CONTROL_LINK_LINES_DRAWLINKS_GHIDRA_REPORT.md`
- **Corrected stale-doc context:** `docs/research/TARGET_LINES_GHIDRA_REPORT.md`, `docs/research/TECHNOCLASS_TARGET_FIELDS_GHIDRA_REPORT.md`, `docs/research/building-selection-brackets/SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md`
- **Ghidra addresses:** `0x004DC060 TechnoClass::DrawActionLines`, `0x007049C0 ActionLines::DrawLine`, `0x006DA9D0 factory rally renderer`, `0x006DAD60 planning overlay renderer`, `0x004DC340 DrawRadarActionLines`, `0x00472160 CaptureManagerClass::DrawLinks`, `0x00472640 CaptureManagerClass::ShouldDrawLinks`, `0x00455DA0 rally eligibility check`
- **INI keys:** `ini/rulesmd.ini:424 MaxWaypointPathLength=15`, `ini/rulesmd.ini:670 WaypointAnimationSpeed=10`, `ini/rulesmd.ini:853 MindControlAttackLineFrames=20`, `ini/rulesmd.ini:11695 Factory=InfantryType`, `ini/rulesmd.ini:11777 Factory=UnitType`, `ini/rulesmd.ini:11877 UnitRepair=yes`, `ini/rulesmd.ini:13353 PsychicDetectionRadius=15`, `ini/rulesmd.ini:13541 Cloning=yes`, `ini/artmd.ini:637 [MIND]`, `ini/artmd.ini:642-646 AlternateFLH0..4`
- **Current repo code:** `src/app_target_lines.rs`, `src/app_context_order.rs`, `src/app_render/build_instances.rs`, `src/app_render/mod.rs`, `src/app_render/draw_passes.rs`, `src/sim/command.rs`, `src/sim/game_entity.rs`, `src/sim/world/world_commands.rs`, `src/sim/world/world_hash.rs`, `src/sim/production/production_queue.rs`, `src/rules/object_type.rs`, `src/rules/house_colors.rs`
- **Recent commits checked:** `ecb99aa`, `af26066`, `973149b`, `0976eb2`, `f6925cd`, `39be632`, `5a7ffd6`, `cda018f`, `d04462c`, `fb39b7a`; no post-design commit invalidated the design premise, but the worktree has unrelated dirty files.
