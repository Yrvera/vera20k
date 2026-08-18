# YAPOWR InfantryAbsorb ExtraPower Bonus — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Reproduce gamemd's GetPowerOutput InfantryAbsorb/UnitAbsorb path so a
garrisoned Yuri Bio-Reactor (YAPOWR) contributes `(Power + ExtraPower ×
OccupantCount) × HealthRatio` instead of `Power × HealthRatio`.

**Architecture:** Single-file change in `sim/power_system.rs` + one new field on
`ObjectType`. No new modules, no upgrade-slot scaffolding, no public interface
changes. `sim/` boundary preserved.

**Design Doc:** `docs/plans/2026-05-20-yapowr-infantry-absorb-extra-power-design.md`

---

## Grounding Summary

- **ra2-rust-game-docs/**: `POWER_SYSTEM_GHIDRA_REPORT.md` (GREEN, 2026-05-20)
  is the primary reference. §GetPowerOutput documents the
  `ExtraPower × OccupantCount` branch and the `UnitAbsorb/InfantryAbsorb` gate.
- **Ghidra verified this session**:
  - `decompile_function 0x0044E7B0` (BuildingClass::GetPowerOutput) — confirmed
    the gate condition `(Type[0x16ae] || Type[0x16af]) && Type+0xee8 > 0 &&
    field_0x114 > 0` and the ordering (bonus added to `base` BEFORE
    `GetHealthRatio() * base` truncated via `ftol`).
  - `decompile_function 0x004555D0` (BuildingClass::IsOperational) — confirmed
    UpgradeCount bypass exists in the binary but cannot fire in stock YR
    (no `PowersUpBuilding=` add-ons in `rulesmd.ini`).
- **Repo pattern mirrored**: existing `infantry_absorb`/`unit_absorb` field
  declarations (`src/rules/object_type.rs:598/602`) and their parse lines
  (`src/rules/object_type.rs:1017-1018`). The new `extra_power: i32` follows
  the same pattern as `power: i32` (declared near line 222, parsed via
  `section.get_i32("Power").unwrap_or(0)` at line 832).
- **INI keys**: `[YAPOWR] ExtraPower=100`, `InfantryAbsorb=yes`, `Passengers=5`,
  `Power=150` (rulesmd.ini line ~13140). `[YABRCK] InfantryAbsorb=yes` test
  fixture exists in object_type.rs:1516. `ExtraPower=-9000` exists on
  `[GAOREP]` Allied Ore Processor in rules.ini line 8878 (negative value;
  must be skipped by the strict `> 0` gate).
- **Unknown**: none for this scope. Upgrade-slot mechanics (G2/G3 as
  originally written) are TS-ghost-not-in-stock and deferred.

## Key Technical Decisions

- **Inline bonus in `recalculate_power_for_owner`, no helper function.** —
  **Confidence:** high. **Source:** design doc Approach A, CLAUDE.md "don't
  add abstractions beyond what the task requires."
- **Bonus is `i32 × i32` integer multiply, added before health scaling.** —
  **Confidence:** high. **Source:** Ghidra 0x44E7B0 decompilation this
  session.
- **Strict `> 0` gate on `extra_power`** — negative or zero ExtraPower
  suppresses the bonus, matching gamemd's `0 < *(int *)(puVar1 + 0xee8)`
  test. **Confidence:** high. **Source:** Ghidra 0x44E7B0.
- **Occupant count read via `entity.passenger_role.cargo().map_or(0, |c|
  c.count())`** — already wired in Rust and used by garrison-fire code.
  **Confidence:** high. **Source:** repo `src/sim/passenger.rs:59`.
- **`theoretical_total_power` (sidebar curve input) NOT extended with the
  bonus** — the sidebar bar tracks `|Power=|` from TypeClass; the green-bar
  fill curve already reads `total_output` and so picks up the bonus
  automatically. **Confidence:** medium. **Source:** current Rust behavior
  + design doc rationale. Flag for `/review-plan`: does
  `theoretical_total_power` need to include `extra_power × max_passengers`?
  gamemd's sidebar behavior is unverified this session; deferred per design.

## Open Questions

### Resolved During Planning

- "Does the InfantryAbsorb bonus scale with HP?" — **Yes**, confirmed via
  Ghidra 0x44E7B0: bonus is added to `iVar5` before the `GetHealthRatio() *
  base` multiply and `ftol` truncation.
- "Does Power < 0 + InfantryAbsorb=yes occur in stock YR?" — **No**. Grepped
  `rulesmd.ini` and `rules.ini`; only `[YAPOWR]` has `InfantryAbsorb=yes` and
  it has `Power=150`. Single-direction power model holds.

### Deferred to Implementation

- None. The math is mechanical and all gate conditions are verified.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/object_type.rs` | Add `extra_power: i32` field + parse `ExtraPower=` |
| Modify | `src/sim/power_system.rs` | Extend `recalculate_power_for_owner` with bonus branch + tests |

## Interface Changes

- `ObjectType` gains a new public field `extra_power: i32`. Default 0, so any
  existing call site that constructs `ObjectType` via `from_section` is
  unaffected. No method signatures change.
- `recalculate_power_for_owner` (private) is the only consumer.

## Sim Checklist

- [x] All math uses integer (`i32`) — no f32/f64 in the new code.
- [x] New state covered by deterministic hash: `extra_power` lives on
  `ObjectType` (immutable rule data, no hash inclusion needed). Bonus
  computation reads `entity.passenger_role.cargo().count()` which is already
  part of the per-entity state contributing to the world hash.
- [x] No new dependencies on render/ui/sidebar/audio/net. Reads
  `crate::sim::passenger` (already sim-internal) and `crate::rules::ruleset`
  (already used).
- [x] Tick ordering: change is inside the existing
  `recalculate_power_for_owner` call within `tick_power_states`, which fires
  in the existing power-system slot of `World::advance_tick`. No reordering.
- [x] BTreeMap iteration order preserved (same `entities.values()` loop).

## Risk Areas

- **Determinism state-hash drift**: any save/replay file recorded BEFORE this
  change with a garrisoned YAPOWR will produce a different `total_output`
  after the fix. This is correct behavior — but flag for the test suite:
  no existing test garrisons a YAPOWR with ExtraPower set, so the 2435-test
  baseline should remain green.
- **Edge case: `extra_power` overflow** — `extra_power * occupants` for a
  pathological mod-INI value (e.g., `ExtraPower=2000000000` + 5 passengers)
  would overflow i32. Use `saturating_mul` to be safe. Stock YR values are
  `100 × 5 = 500`, comfortably inside i32.
- **Edge case: cargo present on a non-garrisonable building** — shouldn't
  happen (garrison-fire flow gates entry via `CanBeOccupied`), but the gate
  `(obj.infantry_absorb || obj.unit_absorb)` already requires one of those
  flags before reading cargo count, so a stray cargo on a regular building
  cannot leak a bonus.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | `ExtraPower × OccupantCount` bonus applied to total_output | Yuri Bio-Reactor power scaling. Player garrisons 5 infantry; power bar must jump from `150` to `650` (or HP-scaled value). Visible every Yuri match. | Unit tests + manual: build YAPOWR, garrison G.I.s, observe power bar in sidebar. |
| Task 4 | Bonus suppressed when `ExtraPower <= 0` | `[GAOREP]` Allied Ore Processor has `ExtraPower=-9000` + no InfantryAbsorb, so this is dual-protection. Must not double-count or sign-flip. Visible when GAOREP is built. | Unit test `test_negative_extra_power_no_bonus`. |
| Task 4 | Bonus inherits HP scaling | A damaged YAPOWR (50% HP) with garrison must produce half power. Visible when reactor takes damage and player is at marginal power. | Unit test `test_yapowr_half_hp_garrisoned`. |
| Task 4 | Bonus suppressed during construction | YAPOWR mid-build with phantom-attached infantry (shouldn't happen, but) must still produce 0. Visible as no early power surge. | Unit test mirrors existing `test_building_under_construction_excluded`. |

---

## Tasks

### Task 1: Add `extra_power: i32` field to ObjectType

**Why:** Make the INI key available on the type-class data before any
consumer can read it. Field-first, parse-and-use after.

**Files:**
- Modify: `src/rules/object_type.rs:222` (field declaration)

**Pattern:** Mirrors the existing `power: i32` field at line 222.

**Step 1: Add the field declaration**

Insert immediately after the existing `pub power: i32` line.

```rust
    /// Power generation (positive) or consumption (negative). Buildings only.
    pub power: i32,
    /// Extra power bonus per occupant for `InfantryAbsorb`/`UnitAbsorb`
    /// buildings. Parsed from `ExtraPower=` (signed i32). Only contributes
    /// when the building has `InfantryAbsorb=yes` or `UnitAbsorb=yes` and
    /// at least one passenger is garrisoned. Stock YR: YAPOWR Bio-Reactor
    /// uses `ExtraPower=100` × up to 5 garrisoned infantry.
    pub extra_power: i32,
```

**Step 2: Verify compile-only**

Run: `cargo check --lib`
Expected: FAIL — every `ObjectType { ... }` construction without
`extra_power: ...` will not compile. This is the desired error; Task 2
fixes it.

**Step 3: Commit**

```
rules/object_type: add extra_power field (no parse yet)
```

### Task 2: Parse `ExtraPower=` from INI in `ObjectType::from_section`

**Why:** Wire the field to the INI key so YAPOWR's `ExtraPower=100` reaches
the simulation. Must compile-and-test after this step.

**Files:**
- Modify: `src/rules/object_type.rs:832` (parse block, near `power:` parse)

**Pattern:** Mirrors `power: section.get_i32("Power").unwrap_or(0)` at line
832.

**Step 1: Add the parse line**

Insert immediately after the existing `power:` parse line in
`ObjectType::from_section`. Order matches the field declaration order in the
struct.

```rust
            power: section.get_i32("Power").unwrap_or(0),
            extra_power: section.get_i32("ExtraPower").unwrap_or(0),
```

**Step 2: Add a parse-level unit test**

Append to the existing `#[cfg(test)] mod tests` block in
`src/rules/object_type.rs`. Locate the existing `test_parse_garrison_building`
or `infantry_absorb` test (around line 1467/1516) and add adjacent:

```rust
    #[test]
    fn test_parse_extra_power_positive() {
        let ini = IniFile::from_str(
            "[YAPOWR]\nPower=150\nExtraPower=100\nInfantryAbsorb=yes\n",
        );
        let obj = ObjectType::from_section(
            "YAPOWR",
            ini.section("YAPOWR").expect("section"),
            ObjectCategory::Building,
        );
        assert_eq!(obj.power, 150);
        assert_eq!(obj.extra_power, 100);
        assert!(obj.infantry_absorb);
    }

    #[test]
    fn test_parse_extra_power_negative() {
        // [GAOREP] Allied Ore Processor in rules.ini has ExtraPower=-9000.
        // Should parse the signed value; the consumer's > 0 gate will
        // suppress the bonus.
        let ini = IniFile::from_str("[GAOREP]\nExtraPower=-9000\n");
        let obj = ObjectType::from_section(
            "GAOREP",
            ini.section("GAOREP").expect("section"),
            ObjectCategory::Building,
        );
        assert_eq!(obj.extra_power, -9000);
    }

    #[test]
    fn test_parse_extra_power_default_zero() {
        let ini = IniFile::from_str("[GAPOWR]\nPower=200\n");
        let obj = ObjectType::from_section(
            "GAPOWR",
            ini.section("GAPOWR").expect("section"),
            ObjectCategory::Building,
        );
        assert_eq!(obj.extra_power, 0);
    }
```

**Step 3: Verify**

Run: `cargo test --lib -p ra2_rust_game -- rules::object_type::tests::test_parse_extra_power`
Expected: 3 tests PASS.

If any other ObjectType test fails because of a missing field initializer in
a test fixture (e.g., a `ObjectType { ... }` literal that doesn't use
`from_section`), grep for `ObjectType {` in `src/` and add `extra_power: 0,`
to those literals. Stock state at 2026-05-20 should not have any such
literals, since the existing fixtures route through `IniFile::from_str` →
`RuleSet::from_ini` → `ObjectType::from_section`.

**Step 4: Commit**

```
rules/object_type: parse ExtraPower= (signed i32, default 0)
```

### Task 3: Extend `recalculate_power_for_owner` with the bonus branch

**Why:** Apply the verified gamemd formula. This is the core parity change.

**Files:**
- Modify: `src/sim/power_system.rs:71-96` (the per-entity loop body)

**Pattern:** Inline branch matching gamemd's GetPowerOutput at 0x44E7B0.

**Step 1: Rewrite the per-entity loop body**

Replace lines 71-96 of `recalculate_power_for_owner` with:

```rust
    for entity in entities.values() {
        if entity.category != EntityCategory::Structure || entity.owner != owner_id {
            continue;
        }
        let Some(obj) = rules.object(interner.resolve(entity.type_ref)) else {
            continue;
        };

        // Theoretical total includes ALL buildings regardless of state.
        // Tracks |Power=| from TypeClass only — does NOT include the
        // ExtraPower garrison bonus. The green-bar fill curve reads
        // total_output (below) which DOES reflect the bonus.
        theoretical += obj.power.unsigned_abs() as i32;

        // Skip buildings still under construction for operational power calc.
        if entity.building_up.is_some() {
            continue;
        }

        // Producer branch: base output = max(Power, 0), plus the garrison
        // ExtraPower bonus for UnitAbsorb/InfantryAbsorb buildings.
        let mut output_contribution: i32 = obj.power.max(0);
        if (obj.infantry_absorb || obj.unit_absorb) && obj.extra_power > 0 {
            let occupants = entity
                .passenger_role
                .cargo()
                .map_or(0, |c| c.count()) as i32;
            if occupants > 0 {
                output_contribution = output_contribution
                    .saturating_add(obj.extra_power.saturating_mul(occupants));
            }
        }
        if output_contribution > 0 {
            // Health-scaled output: integer division rounds toward zero,
            // equivalent to gamemd's ftol(base * health_ratio) for positive
            // operands.
            let hp = entity.health.current as i32;
            let max_hp = entity.health.max.max(1) as i32;
            produced = produced.saturating_add(output_contribution * hp / max_hp);
        }

        // Drain is always the full rated value regardless of health.
        if obj.power < 0 {
            drained = drained.saturating_add(obj.power.saturating_abs());
        }
    }
```

**Step 2: Update the function-level doc comment**

Above the `fn recalculate_power_for_owner` declaration (line 58 area),
extend the doc:

```rust
/// Recalculate power totals for a single owner from their buildings.
///
/// Power output scales with building health using integer arithmetic:
/// `output = Power * current_hp / max_hp` (rounds down, matching RA2).
/// Drain is always the full rated `|Power|` regardless of health.
///
/// `UnitAbsorb`/`InfantryAbsorb` buildings (e.g., Yuri Bio-Reactor) add
/// `ExtraPower × OccupantCount` to their pre-scaled output when at least
/// one passenger is garrisoned and `ExtraPower > 0`. The bonus is scaled
/// by HP along with the base power.
///
/// If spy blackout is active, output is forced to 0 after summation.
```

**Step 3: Verify existing tests still pass**

Run: `cargo test --lib -- power_system::tests`
Expected: all existing tests PASS (test_health_scaled_output,
test_full_health_full_output, test_low_power_detection,
test_drain_always_full_regardless_of_health,
test_spy_blackout_forces_zero_output, test_spy_blackout_timer_decrements,
test_power_transition_events, test_is_building_powered_for_generator,
test_is_building_powered_for_consumer_during_low_power,
test_is_building_powered_for_consumer_during_surplus,
test_low_power_does_not_damage_buildings,
test_building_under_construction_excluded, test_has_active_radar_with_power).

No existing test uses InfantryAbsorb + ExtraPower simultaneously, so
behavior is preserved.

**Step 4: Commit**

```
sim/power_system: apply ExtraPower × occupant bonus to InfantryAbsorb/UnitAbsorb buildings
```

### Task 4: Unit tests for the bonus branch

**Why:** Pin every ledger item: gate conditions, ordering, HP scaling,
strict `> 0` test, construction exclusion, both UnitAbsorb and
InfantryAbsorb gates.

**Files:**
- Modify: `src/sim/power_system.rs` (append to existing `#[cfg(test)] mod tests`)

**Pattern:** Reuse `make_building` helper at line 243 + `test_rules` helper
at line 253. Add a YAPOWR-shaped fixture inline per test.

**Step 1: Add a YAPOWR-shaped rules fixture helper**

Add inside the `tests` mod, near the top of the existing helpers:

```rust
    /// Rules with a YAPOWR-shaped Bio-Reactor: power producer +
    /// InfantryAbsorb=yes + ExtraPower=100. Mirrors stock YR rulesmd.ini.
    fn yapowr_rules() -> RuleSet {
        rules_from_ini(
            "\
[BuildingTypes]
0=YAPOWR
1=GAPOWR

[YAPOWR]
Power=150
Strength=750
Powered=no
InfantryAbsorb=yes
UnitAbsorb=no
ExtraPower=100
Passengers=5

[GAPOWR]
Power=200
Strength=600
Powered=no

[General]
BuildSpeed=0.02
",
        )
    }

    /// Build a YAPOWR test entity with `n` garrisoned passengers and the
    /// given hp/max.
    fn make_yapowr(
        id: u64,
        owner: &str,
        hp: u16,
        max_hp: u16,
        passenger_count: u32,
    ) -> GameEntity {
        let mut e = make_building(id, "YAPOWR", owner, hp, max_hp);
        let mut cargo = crate::sim::passenger::PassengerCargo::new(5, 0);
        for i in 0..passenger_count {
            cargo.passengers.push(100 + i as u64);
            cargo.total_size += 1;
        }
        e.passenger_role = crate::sim::passenger::PassengerRole::Transport { cargo };
        e
    }
```

**Step 2: Add the 7 unit tests**

```rust
    #[test]
    fn test_yapowr_empty_no_bonus() {
        let rules = yapowr_rules();
        let mut store = EntityStore::new();
        store.insert(make_yapowr(1, "Yuri", 750, 750, 0));

        let mut state = PowerState::default();
        let interner = test_interner();
        let yuri = intern::test_intern("Yuri");
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        assert_eq!(state.total_output, 150, "empty YAPOWR = base Power only");
        assert_eq!(state.total_drain, 0);
    }

    #[test]
    fn test_yapowr_garrisoned_full_hp() {
        let rules = yapowr_rules();
        let mut store = EntityStore::new();
        store.insert(make_yapowr(1, "Yuri", 750, 750, 5));

        let mut state = PowerState::default();
        let interner = test_interner();
        let yuri = intern::test_intern("Yuri");
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        assert_eq!(state.total_output, 650, "150 + 100*5 = 650 at full HP");
    }

    #[test]
    fn test_yapowr_garrisoned_half_hp_scales_bonus() {
        let rules = yapowr_rules();
        let mut store = EntityStore::new();
        store.insert(make_yapowr(1, "Yuri", 375, 750, 5));

        let mut state = PowerState::default();
        let interner = test_interner();
        let yuri = intern::test_intern("Yuri");
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        // (150 + 500) * 375 / 750 = 650 * 375 / 750 = 325
        assert_eq!(state.total_output, 325, "bonus scales with HP");
    }

    #[test]
    fn test_no_infantry_absorb_no_bonus() {
        // GAPOWR has Power=200 but no InfantryAbsorb. Garrisoned cargo
        // should be ignored (no garrison flow allows this in practice,
        // but the gate must hold).
        let rules = yapowr_rules();
        let mut store = EntityStore::new();
        let mut e = make_building(1, "GAPOWR", "Allies", 600, 600);
        let mut cargo = crate::sim::passenger::PassengerCargo::new(5, 0);
        cargo.passengers.push(100);
        cargo.total_size += 1;
        e.passenger_role = crate::sim::passenger::PassengerRole::Transport { cargo };
        store.insert(e);

        let mut state = PowerState::default();
        let interner = test_interner();
        let allies = intern::test_intern("Allies");
        recalculate_power_for_owner(&mut state, &store, &rules, allies, &interner);

        assert_eq!(state.total_output, 200, "no InfantryAbsorb = no bonus");
    }

    #[test]
    fn test_extra_power_zero_no_bonus() {
        let rules = rules_from_ini(
            "\
[BuildingTypes]
0=ZEROEX

[ZEROEX]
Power=150
Strength=750
InfantryAbsorb=yes
ExtraPower=0
Passengers=5

[General]
BuildSpeed=0.02
",
        );
        let mut store = EntityStore::new();
        store.insert(make_yapowr(1, "Yuri", 750, 750, 5)); // type_ref still YAPOWR

        // Rebuild entity with the ZEROEX type
        let mut store = EntityStore::new();
        let mut e = make_building(1, "ZEROEX", "Yuri", 750, 750);
        let mut cargo = crate::sim::passenger::PassengerCargo::new(5, 0);
        for i in 0..5 {
            cargo.passengers.push(100 + i);
            cargo.total_size += 1;
        }
        e.passenger_role = crate::sim::passenger::PassengerRole::Transport { cargo };
        store.insert(e);

        let mut state = PowerState::default();
        let interner = test_interner();
        let yuri = intern::test_intern("Yuri");
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        assert_eq!(state.total_output, 150, "ExtraPower=0 fails strict > 0 gate");
    }

    #[test]
    fn test_extra_power_negative_no_bonus() {
        let rules = rules_from_ini(
            "\
[BuildingTypes]
0=NEGEX

[NEGEX]
Power=150
Strength=750
InfantryAbsorb=yes
ExtraPower=-50
Passengers=5

[General]
BuildSpeed=0.02
",
        );
        let mut store = EntityStore::new();
        let mut e = make_building(1, "NEGEX", "Yuri", 750, 750);
        let mut cargo = crate::sim::passenger::PassengerCargo::new(5, 0);
        for i in 0..3 {
            cargo.passengers.push(100 + i);
            cargo.total_size += 1;
        }
        e.passenger_role = crate::sim::passenger::PassengerRole::Transport { cargo };
        store.insert(e);

        let mut state = PowerState::default();
        let interner = test_interner();
        let yuri = intern::test_intern("Yuri");
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        assert_eq!(state.total_output, 150, "ExtraPower<0 fails strict > 0 gate");
    }

    #[test]
    fn test_unit_absorb_path_also_works() {
        // gamemd gate is (UnitAbsorb || InfantryAbsorb). Verify UnitAbsorb
        // alone (no InfantryAbsorb) still grants the bonus.
        let rules = rules_from_ini(
            "\
[BuildingTypes]
0=UABS

[UABS]
Power=100
Strength=500
InfantryAbsorb=no
UnitAbsorb=yes
ExtraPower=80
Passengers=3

[General]
BuildSpeed=0.02
",
        );
        let mut store = EntityStore::new();
        let mut e = make_building(1, "UABS", "Yuri", 500, 500);
        let mut cargo = crate::sim::passenger::PassengerCargo::new(3, 0);
        for i in 0..2 {
            cargo.passengers.push(100 + i);
            cargo.total_size += 1;
        }
        e.passenger_role = crate::sim::passenger::PassengerRole::Transport { cargo };
        store.insert(e);

        let mut state = PowerState::default();
        let interner = test_interner();
        let yuri = intern::test_intern("Yuri");
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        assert_eq!(state.total_output, 260, "100 + 80*2 = 260 via UnitAbsorb gate");
    }

    #[test]
    fn test_yapowr_under_construction_excluded() {
        let rules = yapowr_rules();
        let mut store = EntityStore::new();
        let mut e = make_yapowr(1, "Yuri", 750, 750, 5);
        e.building_up = Some(crate::sim::components::BuildingUp {
            elapsed_ticks: 0,
            total_ticks: 30,
        });
        store.insert(e);

        let mut state = PowerState::default();
        let interner = test_interner();
        let yuri = intern::test_intern("Yuri");
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        assert_eq!(state.total_output, 0, "building_up suppresses all output including bonus");
    }
```

**Step 3: Verify**

Run: `cargo test --lib -- power_system::tests`
Expected: all 13 existing tests + 7 new tests = 20 PASS.

**Step 4: Commit**

```
sim/power_system: cover ExtraPower bonus, HP scaling, gate edge cases
```

### Task 5: Full regression — `cargo test --lib`

**Why:** Confirm the 2435-test baseline is unchanged outside the
power-system additions.

**Files:** none modified.

**Step 1: Run the full library test suite**

Run: `cargo test --lib`
Expected: `test result: ok.` with `passed = 2435 + 7 + 3 = 2445` (plus or
minus exact baseline; the delta should be +10 new tests, all passing).

**Step 2: If anything fails**

- Diagnose root cause; do NOT skip with `#[ignore]` or `#[cfg(...)]`.
- If a parallel session has landed unrelated breakage (per CLAUDE.md
  "Parallel sessions"), only report the count and continue.

**Step 3: No commit needed** (no file changes).

### Task 6: Patch the disparity scan — demote G2/G3 from HIGH to TS-ghost

**Why:** The G2/G3 items in the scan are misclassified as HIGH-active. Next
gap-scan will re-raise them as live bugs unless we record the verdict from
this brainstorm with Ghidra citations. Per CLAUDE.md "feedback-swarm-auto-patch"
memory: clear WRONG findings should be patched in-place with the verifying
Ghidra call cited.

**Files:**
- Modify: `docs/gap-scans/2026-05-20-disparity-scan-power-system.md` (G2,
  G3 entries + recommendations footer)

**Step 1: Replace the G2 finding text**

Replace the existing G2 block (lines 38-41 area) with:

```markdown
**G2. UpgradeCount ≥ 2 bypass — TS-ghost, dormant in stock YR**
- **Doc:** POWER_SYSTEM_GHIDRA_REPORT.md §IsOperational: "UpgradeCount < 2 —
  upgrades bypass power check!"
- **Rust state:** MISSING — `is_building_powered()` at
  `src/sim/power_system.rs:231–256` checks only `obj.powered` and
  `obj.power > 0`.
- **Verdict (2026-05-20):** **Not active in stock YR.** Grepping
  `ini/rulesmd.ini` and `ini/rules.ini` finds zero buildings with
  `PowersUpBuilding=` — only the commented template at line 3659/3086. Only
  `[GAPOWR]` and `[YAPOWR]` declare `Upgrades=2` capacity, but no add-on
  buildings exist to fill the slots. UpgradeCount stays at 0 for every
  building in a normal skirmish, so the `>= 2` bypass is unreachable code.
  Verified via `decompile_function 0x004555D0` (this session) — the bypass
  exists in the binary but is dormant for stock YR.
- **Severity rationale:** No player-visible effect. Deferred until mod
  scope or a future stock-YR mechanism re-introduces upgrade-add-on
  buildings.
```

**Step 2: Replace the G3 finding text**

Replace the existing G3 block (lines 43-46 area) with:

```markdown
**G3. Upgrade-slot power chain (3-slot iteration in GetPowerOutput) —
TS-ghost, dormant in stock YR**
- **Doc:** POWER_SYSTEM_GHIDRA_REPORT.md §GetPowerOutput: "if building has
  upgrade slots (3 slots): for each occupied slot: base += slot->PowerOutput."
- **Rust state:** MISSING — no upgrade-slot pointer scaffolding on
  GameEntity, no `PowersUpBuilding=` parsing.
- **Verdict (2026-05-20):** **Not active in stock YR.** Same reason as G2:
  no add-on buildings to attach. The 3-slot loop at
  `decompile_function 0x0044E7B0` is dormant in stock skirmish.
- **What IS the live extra-power gap:** the
  `InfantryAbsorb/UnitAbsorb × ExtraPower × OccupantCount` branch of
  GetPowerOutput. Fires every Yuri match where YAPOWR
  (`InfantryAbsorb=yes`, `ExtraPower=100`, `Passengers=5`) is garrisoned.
  Closed by
  [docs/plans/2026-05-20-yapowr-infantry-absorb-extra-power-design.md](../plans/2026-05-20-yapowr-infantry-absorb-extra-power-design.md).
```

**Step 3: Append a new G3.1 finding for the actually-live gap**

Insert immediately after the patched G3 block:

```markdown
**G3.1. YAPOWR InfantryAbsorb ExtraPower × OccupantCount bonus missing
(NEW — replaces the original G3 framing)**
- **Doc:** POWER_SYSTEM_GHIDRA_REPORT.md §GetPowerOutput: "if (UnitAbsorb ||
  InfantryAbsorb) && ExtraPower > 0 && OccupantCount > 0:
  base += ExtraPower × OccupantCount."
- **Rust state:** MISSING at scan time (2026-05-20). Will be added by
  [docs/plans/2026-05-20-yapowr-infantry-absorb-extra-power-plan.md](../plans/2026-05-20-yapowr-infantry-absorb-extra-power-plan.md).
- **Severity rationale:** Fires every Yuri match where a Bio-Reactor is
  garrisoned. Player-visible power-bar jump from 150 to 650 (full
  garrison, full HP). High player-visibility × high frequency in any
  Yuri-faction match.
- **Verification:** `decompile_function 0x0044E7B0` (this session)
  confirmed the gate condition and ordering — bonus added BEFORE health
  scaling, gates strict on all three conditions.
```

**Step 4: Update the Recommendations footer**

Replace the existing line `G3 (upgrade slot power) and G2 (UpgradeCount
bypass) form a natural cluster ...` with:

```markdown
G1 (degradation damage) was the most urgent and has been closed (2026-05-20
quickwins). G2 (UpgradeCount bypass) and G3 (3-slot upgrade chain) are
TS-ghost-not-active-in-stock and deferred until upgrade-add-on buildings
exist. G3.1 (YAPOWR InfantryAbsorb ExtraPower bonus) is the actually-live
extra-power gap — closed by the YAPOWR design/plan landed 2026-05-20.
G4 (low-power shroud) is the next highest-visibility missing visual effect.
```

**Step 5: Commit**

```
gap-scans/power-system: demote G2/G3 to TS-ghost, add G3.1 YAPOWR ExtraPower
```

### Task 7: Verification against gamemd.exe behavior

**Why:** Confirm the implementation matches original engine behavior in a
real skirmish (not just synthetic unit tests).

**Verify:**

- **Setup**: Skirmish as Yuri faction, build a Yuri Bio-Reactor (YAPOWR).
- **Empty reactor**: power bar shows +150 from the reactor (matches `Power=150`).
- **Garrison 1 infantry**: power bar jumps to +250 (`150 + 100×1`).
- **Garrison 5 infantry (max)**: power bar shows +650 (`150 + 100×5`).
- **Damage the reactor to ~50% HP**: power output drops to ~325 (linear
  health scale).
- **Empty the reactor (kill garrison)**: power output returns to 150 ×
  current_hp_ratio.
- **Compare side-by-side** with gamemd.exe if possible: same garrison
  composition should give identical power-bar reading.

**Expected:** All five observations match. Any divergence indicates a
formula error and must be debugged before declaring complete.

**No commit** (verification only).

---

## Sources & References

- **Design doc:**
  [docs/plans/2026-05-20-yapowr-infantry-absorb-extra-power-design.md](2026-05-20-yapowr-infantry-absorb-extra-power-design.md)
- **Ghidra reports:**
  [ra2-rust-game-docs/POWER_SYSTEM_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/POWER_SYSTEM_GHIDRA_REPORT.md)
  (GREEN, 2026-05-20)
- **gamemd.exe addresses (verified this session via decompile_function):**
  - `0x0044E7B0` — BuildingClass::GetPowerOutput (bonus gate + ordering)
  - `0x004555D0` — BuildingClass::IsOperational (UpgradeCount bypass —
    confirmed dormant in stock YR)
- **INI keys:**
  - `[YAPOWR]` Power=150, ExtraPower=100, InfantryAbsorb=yes, Passengers=5,
    Strength=750 — rulesmd.ini line ~13140
  - `[GAOREP]` ExtraPower=-9000 — rules.ini line 8878 (negative-value
    fixture for the strict `> 0` gate test)
  - `[GAPOWR]` Power=200, Upgrades=2 — rulesmd.ini line 11669 (dormant
    upgrade capacity, no add-ons in stock)
- **Related code:**
  - `src/rules/object_type.rs:222` (power field), `:598/602`
    (infantry_absorb/unit_absorb), `:832` (Power parse), `:1017/1018`
    (absorb parses)
  - `src/sim/power_system.rs:58-107` (recalculate_power_for_owner)
  - `src/sim/passenger.rs:31` (PassengerCargo), `:59` (count())
  - `src/sim/game_entity.rs:224` (passenger_role field)
- **Disparity scan being patched:**
  [docs/gap-scans/2026-05-20-disparity-scan-power-system.md](../gap-scans/2026-05-20-disparity-scan-power-system.md)
