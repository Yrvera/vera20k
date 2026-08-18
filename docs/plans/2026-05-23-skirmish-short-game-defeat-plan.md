# Skirmish Short Game Defeat Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make Rust's deterministic sim defeat condition match standard YR Skirmish Short Game behavior for last-building loss, including the stock MCV/BaseUnit survivor exception.

**Architecture:** This stays in `rules/` plus `sim/`. `[General] BaseUnit=` is rules data and belongs in `GeneralRules`; defeat evaluation remains deterministic gameplay state in `Simulation::check_defeat`. No render, UI, sidebar, audio, or net dependency is introduced.

**Design Doc:** `docs/plans/2026-05-23-skirmish-short-game-defeat-design.md`

---

## Grounding Summary

- `traces/SKIRMISH_SHORT_GAME_LAST_BUILDING_DEFEAT_TRACE.md` verifies the original player-visible gap: with Short Game enabled, YR defeats a house that has `OwnedBuildings=0`, one ordinary non-building unit, and zero counted BaseUnit/ConYard-style instances.
- `skirmish-ui/SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md` verifies offline Skirmish Start packs checkbox `0x54E` into `DAT_00A8B262`, and `HouseClass__Update @ 0x004F86F0` consumes that byte as the Short Game defeat selector.
- Live Ghidra re-check of `HouseClass__Update @ 0x004F86F0` confirms the active Short Game branch: non-campaign game mode, not already defeated, frame `> 0`, non-passive house; if `DAT_00A8B262 != 0`, defeat triggers only when `OwnedBuildings < 1` and the three `[General] BaseUnit` instance counts are also zero.
- Live Ghidra re-check of `HouseClass__MPlayer_Defeated @ 0x004FC0B0` confirms the defeated flag is set immediately at house offset `+0x1F5`; broader local defeat aftermath remains outside this plan.
- Stock YR `ini/rulesmd.ini:[General] BaseUnit=AMCV,SMCV,PCV`; stock base RA2 `ini/rules.ini:[General] BaseUnit=AMCV,SMCV`.
- Stock YR `ini/rulesmd.ini:[MultiplayerDialogSettings] ShortGame=yes`; base `ini/rules.ini` also has `ShortGame=yes`.
- Repo pattern: `RuleSet` owns `[General]` data through `GeneralRules`; `Simulation::check_defeat` already owns defeat marking and survivor victory resolution.
- Current Rust mismatch: `check_defeat` uses `owned_building_count + owned_unit_count == 0`, so ordinary units incorrectly keep a player alive under Short Game.
- Review correction: a simple `owned_building_count == 0` predicate is not enough. It would incorrectly defeat a player who still owns a packed MCV/BaseUnit after the last ConYard is gone.

## Key Technical Decisions

- Parse `[General] BaseUnit` into `GeneralRules.base_unit_types: Vec<String>` as uppercase type IDs - **Confidence:** high
  - **Source:** `ini/rulesmd.ini:390`, live Ghidra `HouseClass__Update @ 0x004F86F0`, repo pattern `GeneralRules::from_ini`.

- Short Game defeat predicate is `owned_building_count == 0 && !house_has_live_base_unit(owner, rules)` - **Confidence:** high
  - **Source:** live Ghidra branch checks `OwnedBuildings < 1` and three counted BaseUnit-style instance counts `< 1`; current Rust can represent this by scanning live entities owned by the house whose type is in `rules.general.base_unit_types`.

- Long-game predicate remains current Rust behavior expressed without addition: `owned_building_count == 0 && owned_unit_count == 0` - **Confidence:** high
  - **Source:** existing Rust behavior and reviewed design scope.

- `check_defeat` should accept `rules: Option<&RuleSet>` and `advance_tick` should pass its existing `rules` argument through - **Confidence:** high
  - **Source:** no new sim state is needed; `advance_tick` already receives rules each tick.

- Do not set `has_lost`, scatter units, reveal the map, play EVA, or disable input/sidebar in this patch - **Confidence:** high
  - **Source:** approved design scope and `MULTIPLAYER_DEFEAT_VICTORY_GHIDRA_REPORT.md`; those are broader `MPlayer_Defeated` aftermath behaviors.

## Open Questions

### Resolved During Planning

- Does the scoped Skirmish Short Game checkbox feed defeat detection? Yes. `0x54E` writes `DAT_00A8B262`, and `HouseClass__Update @ 0x004F86F0` reads it in the multiplayer defeat branch.
- Is the BaseUnit survivor exception normal stock gameplay? Yes. Stock YR has `BaseUnit=AMCV,SMCV,PCV`, and the native branch counts those instances before calling defeat.
- Does current Rust have enough data to represent the exception? Yes. `GameEntity` stores `owner`, `type_ref`, and `dying`; `RuleSet` can parse BaseUnit type IDs.

### Deferred To Follow-Up

- Full local-player defeat aftermath: map reveal, input/sidebar/radar changes, EVA/message, `has_lost`, borrowed-time timing, and `Flag_To_Lose`.
- Passive-house exclusion. Current Rust `HouseState` does not carry the native `HouseType.MultiplayPassive` gate.
- Exact native `CountOwnedInstances` category internals beyond the stock BaseUnit identity list. This plan matches stock visible output by checking live entities whose type is in `[General] BaseUnit`.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/ruleset.rs` | Parse and store `[General] BaseUnit` on `GeneralRules`. |
| Modify | `src/sim/world/mod.rs` | Pass rules into defeat checking and switch defeat predicate based on Short Game plus BaseUnit survivors. |
| Modify | `src/sim/world/world_tests.rs` | Add focused tests for Short Game ordinary-unit defeat, BaseUnit survival, long-game behavior, and victory resolution. |

## Interface Changes

- `GeneralRules` gains `pub base_unit_types: Vec<String>`.
- Private `Simulation::check_defeat` changes from `fn check_defeat(&mut self)` to `fn check_defeat(&mut self, rules: Option<&RuleSet>)`.
- No public sim API, trait, config schema, or deterministic state hash change is planned.

## Sim Checklist

- [x] All logic uses integer counters and identity comparisons; no `f32` or `f64` in sim logic.
- [x] No new deterministic sim state; `BaseUnit` remains rules data, and existing house flags/counts are already hashed.
- [x] No dependencies on render, UI, sidebar, audio, or net.
- [x] Tick ordering unchanged: `advance_tick` still calls defeat detection in Phase 8.5 only when `self.tick > 0`.
- [x] `BTreeMap` owner iteration remains deterministic and unchanged.
- [x] Entity scan uses deterministic state only and excludes `dying` entities so already-counted deaths do not keep a house alive.

## Risk Areas

- MCV/BaseUnit-only players must stay alive under Short Game. This is the reviewed plan correction and should get an explicit regression test.
- Ordinary non-BaseUnit survivors must not keep a house alive under Short Game.
- Long-game behavior must remain unchanged: any owned unit keeps the house alive when Short Game is off.
- `rules: None` cannot know modded BaseUnit identities. The helper should treat missing rules as no BaseUnit survivors; normal app ticks pass `Some(rules)`.
- Existing tests that call `advance_tick` with `rules: None` may observe Short Game default behavior if they create houses with zero buildings. The focused tests should call `check_defeat(Some(&rules))` directly where BaseUnit identity matters.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Parse `[General] BaseUnit=AMCV,SMCV,PCV` | Native Short Game uses BaseUnit-style survivor counts | Unit test in `ruleset.rs` |
| Task 2 | Short Game defeats ordinary unit-only houses | Players should lose after last building if only tanks/infantry remain | `short_game_defeats_house_with_no_buildings_even_if_ordinary_units_remain` |
| Task 3 | Short Game keeps BaseUnit-only houses alive | Packed MCVs are the native "home" fallback when no buildings exist | `short_game_keeps_house_alive_when_base_unit_remains` |
| Task 4 | Long game still waits for all owned objects | The Short Game checkbox must visibly change defeat behavior | `long_game_keeps_house_alive_when_units_remain` |
| Task 5 | Victory resolution uses the corrected defeated state | Last non-allied survivor should win after the opponent is truly defeated | `short_game_victory_resolution_uses_new_defeat_state` |

---

## Tasks

### Task 1: Parse `[General] BaseUnit`

**Why:** Native Short Game does not defeat a house while it still owns one of the configured BaseUnit types. Rust must know that list before fixing the sim predicate.

**Files:**
- Modify: `src/rules/ruleset.rs`

**Pattern:** Follow existing `GeneralRules` fields parsed from `[General]` using `general.get_list(...)`, normalizing type IDs to uppercase strings.

**Step 1: Add the field to `GeneralRules` near the other gameplay-affecting `[General]` fields**

```rust
/// Unit types that count as a player's home when no buildings remain.
/// Parsed from `[General] BaseUnit=`. Stock YR: AMCV, SMCV, PCV.
pub base_unit_types: Vec<String>,
```

**Step 2: Add the default in `impl Default for GeneralRules`**

```rust
base_unit_types: vec!["AMCV".to_string(), "SMCV".to_string(), "PCV".to_string()],
```

**Step 3: Parse the field in `GeneralRules::from_ini`**

Inside the `Self { ... }` built from `let Some(general) = ini.section("General") else { return Self::default(); };`, add:

```rust
base_unit_types: general
    .get_list("BaseUnit")
    .map(|items| {
        items
            .into_iter()
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty())
            .collect()
    })
    .unwrap_or_else(|| Self::default().base_unit_types),
```

**Step 4: Add parser tests in the existing `#[cfg(test)] mod tests` in `ruleset.rs`**

```rust
#[test]
fn base_unit_types_parse_from_general() {
    let ini = IniFile::from_str("[General]\nBaseUnit=AMCV,SMCV,PCV\n");
    let general = GeneralRules::from_ini(&ini);
    assert_eq!(general.base_unit_types, vec!["AMCV", "SMCV", "PCV"]);
}

#[test]
fn base_unit_types_default_to_stock_yr() {
    let ini = IniFile::from_str("[General]\n");
    let general = GeneralRules::from_ini(&ini);
    assert_eq!(general.base_unit_types, vec!["AMCV", "SMCV", "PCV"]);
}
```

**Step 5: Verify**

Run:

```powershell
cargo test base_unit_types --lib
```

Expected: both tests pass.

### Task 2: Thread Rules Into Defeat Detection

**Why:** `check_defeat` needs rules data to know which unit types count as BaseUnits.

**Files:**
- Modify: `src/sim/world/mod.rs`

**Pattern:** `advance_tick` already receives `rules: Option<&RuleSet>` and passes rules into other tick systems.

**Step 1: Change the private signature**

```rust
fn check_defeat(&mut self, rules: Option<&RuleSet>) {
```

**Step 2: Update the Phase 8.5 caller**

Replace:

```rust
self.check_defeat();
```

with:

```rust
self.check_defeat(rules);
```

**Step 3: Add a private helper near `check_defeat`**

```rust
fn house_has_live_base_unit(&self, owner: InternedId, rules: Option<&RuleSet>) -> bool {
    let Some(rules) = rules else {
        return false;
    };

    self.entities.values().any(|entity| {
        entity.owner == owner
            && !entity.dying
            && rules
                .general
                .base_unit_types
                .iter()
                .any(|type_id| self.interner.resolve(entity.type_ref).eq_ignore_ascii_case(type_id))
    })
}
```

**Step 4: Verify access**

Confirm `RuleSet` and `InternedId` are already imported in `src/sim/world/mod.rs`. They are currently used by `advance_tick` and `check_defeat`'s owner collection, so no new imports should be necessary.

### Task 3: Implement The Corrected Predicate

**Why:** This closes the ordinary-unit Short Game mismatch without defeating MCV/BaseUnit-only players.

**Files:**
- Modify: `src/sim/world/mod.rs`

**Pattern:** Keep the existing owner snapshot, `is_defeated` skip, alive-house collection, and win/alliance resolution unchanged.

**Step 1: Replace the current total-count predicate inside the `for &owner in &owners` loop**

Use this exact shape after the `house.is_defeated` skip:

```rust
let should_defeat = if self.game_options.short_game {
    house.owned_building_count == 0 && !self.house_has_live_base_unit(owner, rules)
} else {
    house.owned_building_count == 0 && house.owned_unit_count == 0
};

if should_defeat {
    if let Some(h) = self.houses.get_mut(&owner) {
        h.is_defeated = true;
    }
}
```

**Step 2: Update comments**

Replace the current zero-owned-objects comment with:

```rust
// Short Game defeats houses with no buildings unless a BaseUnit remains.
// Long games wait for all owned objects.
```

**Step 3: Verify unchanged flow**

Confirm these blocks are not moved or rewritten:

- The `house.is_defeated` skip remains before the predicate.
- The alive-house collection still filters `!h.is_defeated`.
- The single-survivor `has_won` logic remains unchanged.
- The allied-survivor win logic remains unchanged.

### Task 4: Add Defeat Predicate Tests

**Why:** The tests must cover the reviewed correction: ordinary units do not keep a Short Game player alive, but BaseUnits do.

**Files:**
- Modify: `src/sim/world/world_tests.rs`

**Pattern:** Add ordinary unit tests in the existing world child test module. Direct `sim.check_defeat(Some(&rules))` is allowed because `world_tests.rs` is a child module of `sim::world`.

**Step 1: Add a focused rules helper near `combat_test_rules`**

```rust
fn short_game_defeat_test_rules() -> RuleSet {
    let ini = IniFile::from_str(
        "[General]\nBaseUnit=AMCV,SMCV,PCV\n\n\
         [InfantryTypes]\n0=E1\n\n\
         [VehicleTypes]\n0=MTNK\n1=AMCV\n2=SMCV\n3=PCV\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=GACNST\n\n\
         [E1]\nStrength=125\nArmor=flak\nSpeed=4\n\n\
         [MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\n\n\
         [AMCV]\nStrength=450\nArmor=heavy\nSpeed=5\nDeploysInto=GACNST\n\n\
         [SMCV]\nStrength=450\nArmor=heavy\nSpeed=5\nDeploysInto=GACNST\n\n\
         [PCV]\nStrength=450\nArmor=heavy\nSpeed=5\nDeploysInto=GACNST\n\n\
         [GACNST]\nStrength=1000\nArmor=wood\nFoundation=4x3\nUndeploysInto=AMCV\n",
    );
    RuleSet::from_ini(&ini).expect("short game defeat test rules should parse")
}
```

**Step 2: Add a house-count helper near `empty_heights`**

```rust
fn insert_house_with_counts(
    sim: &mut Simulation,
    name: &str,
    buildings: u32,
    units: u32,
) -> InternedId {
    let owner = sim.interner.intern(name);
    let mut house = crate::sim::house_state::HouseState::new(owner, 0, None, true, 0, 10);
    house.owned_building_count = buildings;
    house.owned_unit_count = units;
    sim.houses.insert(owner, house);
    owner
}
```

**Step 3: Add an entity helper near the house helper**

```rust
fn insert_test_entity_for_owner(
    sim: &mut Simulation,
    stable_id: u64,
    owner: InternedId,
    type_id: &str,
    category: EntityCategory,
) {
    let mut entity = GameEntity::test_default(stable_id, type_id, sim.interner.resolve(owner), 10, 10);
    entity.owner = owner;
    entity.type_ref = sim.interner.intern(type_id);
    entity.category = category;
    sim.entities.insert(entity);
}
```

**Step 4: Add the ordinary-unit Short Game defeat test**

```rust
#[test]
fn short_game_defeats_house_with_no_buildings_even_if_ordinary_units_remain() {
    let rules = short_game_defeat_test_rules();
    let mut sim = Simulation::new();
    sim.game_options.short_game = true;
    let owner = insert_house_with_counts(&mut sim, "Americans", 0, 1);
    insert_test_entity_for_owner(&mut sim, 1, owner, "MTNK", EntityCategory::Vehicle);

    sim.check_defeat(Some(&rules));

    assert!(sim.houses[&owner].is_defeated);
}
```

**Step 5: Add the BaseUnit survival test**

```rust
#[test]
fn short_game_keeps_house_alive_when_base_unit_remains() {
    let rules = short_game_defeat_test_rules();
    let mut sim = Simulation::new();
    sim.game_options.short_game = true;
    let owner = insert_house_with_counts(&mut sim, "Americans", 0, 1);
    insert_test_entity_for_owner(&mut sim, 1, owner, "AMCV", EntityCategory::Vehicle);

    sim.check_defeat(Some(&rules));

    assert!(!sim.houses[&owner].is_defeated);
}
```

**Step 6: Add the dying BaseUnit does not survive test**

```rust
#[test]
fn short_game_defeats_when_only_base_unit_is_dying() {
    let rules = short_game_defeat_test_rules();
    let mut sim = Simulation::new();
    sim.game_options.short_game = true;
    let owner = insert_house_with_counts(&mut sim, "Americans", 0, 0);
    insert_test_entity_for_owner(&mut sim, 1, owner, "AMCV", EntityCategory::Vehicle);
    sim.entities.get_mut(1).expect("AMCV inserted").dying = true;

    sim.check_defeat(Some(&rules));

    assert!(sim.houses[&owner].is_defeated);
}
```

**Step 7: Add the long-game survival test**

```rust
#[test]
fn long_game_keeps_house_alive_when_units_remain() {
    let rules = short_game_defeat_test_rules();
    let mut sim = Simulation::new();
    sim.game_options.short_game = false;
    let owner = insert_house_with_counts(&mut sim, "Americans", 0, 1);

    sim.check_defeat(Some(&rules));

    assert!(!sim.houses[&owner].is_defeated);
}
```

**Step 8: Add the long-game zero-object defeat test**

```rust
#[test]
fn long_game_defeats_when_no_owned_objects_remain() {
    let rules = short_game_defeat_test_rules();
    let mut sim = Simulation::new();
    sim.game_options.short_game = false;
    let owner = insert_house_with_counts(&mut sim, "Americans", 0, 0);

    sim.check_defeat(Some(&rules));

    assert!(sim.houses[&owner].is_defeated);
}
```

**Step 9: Verify**

Run:

```powershell
cargo test short_game_ --lib
cargo test long_game_ --lib
```

Expected: the new tests pass. If the filters catch unrelated tests, run the explicit test names from this task.

### Task 5: Add Victory Resolution Regression

**Why:** The corrected predicate feeds the existing alive-house set. This test proves the survivor wins only when the opponent is actually defeated by Short Game.

**Files:**
- Modify: `src/sim/world/world_tests.rs`

**Pattern:** Reuse helpers from Task 4 and existing `check_defeat` win-resolution behavior.

**Step 1: Add the two-house Short Game victory test**

```rust
#[test]
fn short_game_victory_resolution_uses_new_defeat_state() {
    let rules = short_game_defeat_test_rules();
    let mut sim = Simulation::new();
    sim.game_options.short_game = true;
    let defeated = insert_house_with_counts(&mut sim, "Americans", 0, 1);
    let survivor = insert_house_with_counts(&mut sim, "Russians", 1, 0);
    insert_test_entity_for_owner(&mut sim, 1, defeated, "MTNK", EntityCategory::Vehicle);

    sim.check_defeat(Some(&rules));

    assert!(sim.houses[&defeated].is_defeated);
    assert!(sim.houses[&survivor].has_won);
}
```

**Step 2: Add the BaseUnit no-victory regression**

```rust
#[test]
fn short_game_base_unit_survivor_prevents_enemy_victory() {
    let rules = short_game_defeat_test_rules();
    let mut sim = Simulation::new();
    sim.game_options.short_game = true;
    let mcv_owner = insert_house_with_counts(&mut sim, "Americans", 0, 1);
    let enemy = insert_house_with_counts(&mut sim, "Russians", 1, 0);
    insert_test_entity_for_owner(&mut sim, 1, mcv_owner, "AMCV", EntityCategory::Vehicle);

    sim.check_defeat(Some(&rules));

    assert!(!sim.houses[&mcv_owner].is_defeated);
    assert!(!sim.houses[&enemy].has_won);
}
```

**Step 3: Verify**

Run:

```powershell
cargo test short_game_victory_resolution_uses_new_defeat_state --lib
cargo test short_game_base_unit_survivor_prevents_enemy_victory --lib
```

Expected: both tests pass.

### Task 6: Focused Regression And Diff Review

**Why:** This changes rules parsing plus deterministic sim behavior. Verify the narrow behavior and confirm no unintended state/hash edit.

**Files:**
- Read-only review: `src/sim/world/world_hash.rs`

**Step 1: Run focused tests**

Run:

```powershell
cargo test base_unit_types --lib
cargo test short_game_ --lib
cargo test long_game_ --lib
```

Expected: all new tests pass. If broad filters catch unrelated failures, rerun the explicit test names from Tasks 1, 4, and 5.

**Step 2: Confirm hash impact**

Read `src/sim/world/world_hash.rs` and verify no edit is needed. This patch adds rules data and logic only; it does not add persistent sim state. Existing `GameOptions.short_game`, house counts, and defeat/victory flags are already hashed.

**Step 3: Run a broader check if time allows**

Run:

```powershell
cargo test world:: --lib
```

Expected: pass. If it is too broad or times out, record the timeout and focused-test results.

**Step 4: Inspect diff**

Run:

```powershell
git diff -- src/rules/ruleset.rs src/sim/world/mod.rs src/sim/world/world_tests.rs src/sim/world/world_hash.rs
```

Expected:

- `src/rules/ruleset.rs` only adds `GeneralRules.base_unit_types`, parsing, defaults, and tests.
- `src/sim/world/mod.rs` only threads rules into `check_defeat`, adds the BaseUnit helper, and changes the defeat predicate/comment.
- `src/sim/world/world_tests.rs` only adds focused helpers and tests.
- `src/sim/world/world_hash.rs` has no diff.

**Step 5: Commit policy**

Do not commit unless the user explicitly asks for a commit. If asked, use:

```text
sim: honor short game base-unit defeat rule
```

---

## Sources & References

- **Design doc:** `docs/plans/2026-05-23-skirmish-short-game-defeat-design.md`
- **Trace:** `docs/research/traces/SKIRMISH_SHORT_GAME_LAST_BUILDING_DEFEAT_TRACE.md`
- **Ghidra report:** `docs/research/skirmish-ui/SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`
- **Ghidra report:** `docs/research/MULTIPLAYER_DEFEAT_VICTORY_GHIDRA_REPORT.md`
- **Live Ghidra re-check:** `HouseClass__Update @ 0x004F86F0`
- **Live Ghidra re-check:** `HouseClass__MPlayer_Defeated @ 0x004FC0B0`
- **INI key:** `ini/rulesmd.ini:[General] BaseUnit=AMCV,SMCV,PCV`
- **INI key:** `ini/rulesmd.ini:[MultiplayerDialogSettings] ShortGame=yes`
- **INI key:** `ini/rules.ini:[General] BaseUnit=AMCV,SMCV`
- **INI key:** `ini/rules.ini:[MultiplayerDialogSettings] ShortGame=yes`
- **Related code:** `src/rules/ruleset.rs`
- **Related code:** `src/sim/world/mod.rs`
- **Related code:** `src/sim/game_options.rs`
- **Related code:** `src/sim/house_state.rs`
- **Related code:** `src/sim/world/world_hash.rs`

## Post-Plan Self-Review

- Spec coverage: The ordinary-unit Short Game gap and reviewed MCV/BaseUnit survivor correction are both covered by tasks and tests.
- Placeholder scan: No unresolved placeholder steps are present.
- Architecture check: Rules data stays in `rules/`; deterministic defeat logic stays in `sim/`.
- Interface ordering: `GeneralRules.base_unit_types` is added before sim logic consumes it.
- Risk coverage: Ordinary unit defeat, BaseUnit survival, dying BaseUnit, long-game survival, and victory resolution all have tests.
- Self-containment: Each task includes exact files, code shape, commands, and expected results.
- Sim compliance: No floating point, no new hash state, no tick-order move, deterministic owner iteration unchanged.
- Grounding coverage: The plan cites docs, live Ghidra checks, repo patterns, and INI defaults.
- Confidence tagging: All key decisions have confidence and sources.
- Deferred questions: Passive-house exclusion and full defeat aftermath are explicitly out of this implementation.
- Commit policy: The plan no longer asks the executor to commit unless the user explicitly requests it.
