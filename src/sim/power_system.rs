//! Per-player power state tracking and low-power effects.
//!
//! RA2's power system sums each building's `Power=` value per player:
//! positive values generate power (scaled by building health), negative
//! values consume power (always at full rated value). When drain exceeds
//! output the player enters "low power", disabling `Powered=yes` buildings
//! and slowing production.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/ (ObjectType, GeneralRules) and sim/ (EntityStore).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::InternedId;

/// Per-player power state, updated each simulation tick.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PowerState {
    /// Sum of health-scaled positive `Power=` values for all owned buildings.
    pub total_output: i32,
    /// Sum of absolute negative `Power=` values (always full rated, regardless of health).
    pub total_drain: i32,
    /// True when `total_output < total_drain` (binary threshold).
    pub is_low_power: bool,
    /// Remaining power-blackout frames. While > 0, `total_output` is forced to 0.
    /// Set by spy infiltration of power plants AND by ForceShield superweapon launch.
    #[serde(rename = "spy_blackout_remaining")]
    pub power_blackout_remaining: u32,
    /// Whether the player was in low-power state on the previous tick.
    /// Used to detect transitions for EVA voice events.
    pub was_low_power: bool,
    /// Sum of absolute `|Power=|` values from TypeClass for ALL owned buildings,
    /// regardless of health, construction state, or online status. Used by the
    /// sidebar power bar fill curve (asymptotic: `400 / (total + 400)`).
    pub theoretical_total_power: i32,
}

/// Events emitted when a player's power state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerEvent {
    /// Player entered low-power state (drain now exceeds output).
    EnteredLowPower { owner: InternedId },
    /// Player's power was restored after a deficit.
    PowerRestored { owner: InternedId },
}

/// Recalculate power totals for a single owner from their buildings.
///
/// Power output scales with building health using integer arithmetic:
/// `output = Power * current_hp / max_hp` (rounds down, matching RA2).
/// Drain is always the full rated `|Power|` regardless of health.
///
/// `UnitAbsorb`/`InfantryAbsorb` buildings (e.g., Yuri Bio-Reactor) add
/// `ExtraPower × OccupantCount` to their pre-scaled output when at least
/// one passenger is garrisoned and `ExtraPower > 0`. The bonus is HP-scaled
/// alongside the base power.
///
/// If spy blackout is active, output is forced to 0 after summation.
fn recalculate_power_for_owner(
    state: &mut PowerState,
    entities: &EntityStore,
    rules: &RuleSet,
    owner_id: InternedId,
    interner: &crate::sim::intern::StringInterner,
) {
    let mut produced: i32 = 0;
    let mut drained: i32 = 0;
    // Theoretical total: sum of |Power=| from TypeClass for ALL buildings,
    // including those under construction. Used by the power bar fill curve.
    // Does NOT include the ExtraPower garrison bonus.
    let mut theoretical: i32 = 0;

    for entity in entities.values() {
        if entity.category != EntityCategory::Structure || entity.owner != owner_id {
            continue;
        }
        let Some(obj) = rules.object(interner.resolve(entity.type_ref)) else {
            continue;
        };

        // Theoretical total includes ALL buildings regardless of state.
        theoretical += obj.power.unsigned_abs() as i32;

        // Skip buildings still under construction for operational power calc.
        if entity.building_up.is_some() {
            continue;
        }

        // Producer branch: base = max(Power, 0), plus ExtraPower × occupants
        // for InfantryAbsorb/UnitAbsorb buildings (gate is strict on all
        // three conditions, matching gamemd's GetPowerOutput).
        let mut output_contribution: i32 = obj.power.max(0);
        if (obj.infantry_absorb || obj.unit_absorb) && obj.extra_power > 0 {
            let occupants = entity.passenger_role.cargo().map_or(0, |c| c.count()) as i32;
            if occupants > 0 {
                output_contribution = output_contribution
                    .saturating_add(obj.extra_power.saturating_mul(occupants));
            }
        }
        if output_contribution > 0 {
            // Health-scaled output: integer division rounds toward zero,
            // equivalent to gamemd's ftol(base × health_ratio) for positive
            // operands. Bonus is folded into base before scaling.
            let hp = entity.health.current as i32;
            let max_hp = entity.health.max.max(1) as i32;
            produced = produced.saturating_add(output_contribution * hp / max_hp);
        }

        // Drain branch: always full rated value regardless of health.
        if obj.power < 0 {
            drained = drained.saturating_add(obj.power.saturating_abs());
        }
    }

    // Spy blackout forces output to zero.
    if state.power_blackout_remaining > 0 {
        produced = 0;
    }

    state.total_output = produced;
    state.total_drain = drained;
    state.is_low_power = produced < drained;
    state.theoretical_total_power = theoretical;
}

/// Main per-tick power system entry point.
///
/// For each player with structures: recalculates power totals, decrements
/// spy blackout timer, and returns transition events for EVA voice lines.
///
/// gamemd has no HP-degradation effect during low power — `Powered=yes`
/// buildings simply become inoperational (`is_building_powered` returns false)
/// without taking damage. The DamageDelay timer fields at HouseClass+0x578C/
/// +0x5794 are written in the constructor and never read.
pub fn tick_power_states(
    power_states: &mut BTreeMap<InternedId, PowerState>,
    entities: &mut EntityStore,
    rules: &RuleSet,
    _tick_ms: u32,
    interner: &crate::sim::intern::StringInterner,
) -> Vec<PowerEvent> {
    // Collect unique owners who have structures.
    let mut owners: Vec<InternedId> = Vec::new();
    for entity in entities.values() {
        if entity.category == EntityCategory::Structure && !owners.contains(&entity.owner) {
            owners.push(entity.owner);
        }
    }
    owners.sort();

    let mut events: Vec<PowerEvent> = Vec::new();

    for &owner_id in &owners {
        let state = power_states.entry(owner_id).or_default();

        // Save previous state for transition detection.
        state.was_low_power = state.is_low_power;

        // Decrement spy blackout timer (1 tick = 1 frame at game speed).
        if state.power_blackout_remaining > 0 {
            state.power_blackout_remaining = state.power_blackout_remaining.saturating_sub(1);
        }

        // Recalculate power totals with health scaling.
        recalculate_power_for_owner(state, entities, rules, owner_id, interner);

        // Detect transitions.
        if state.is_low_power && !state.was_low_power {
            events.push(PowerEvent::EnteredLowPower { owner: owner_id });
        } else if !state.is_low_power && state.was_low_power {
            events.push(PowerEvent::PowerRestored { owner: owner_id });
        }
    }

    events
}

/// Check whether a specific building is functionally active (not disabled by low power).
///
/// Returns `false` if the owner is in low power AND the building has `Powered=yes`
/// AND consumes power (`Power= <= 0`). Power plants (positive `Power=`) are never
/// deactivated by low power.
pub fn is_building_powered(
    power_states: &BTreeMap<InternedId, PowerState>,
    rules: &RuleSet,
    entity: &GameEntity,
    interner: &crate::sim::intern::StringInterner,
) -> bool {
    if entity.category != EntityCategory::Structure {
        return true;
    }
    let Some(obj) = rules.object(interner.resolve(entity.type_ref)) else {
        return true;
    };
    // Power plants (positive Power=) are never deactivated.
    if obj.power > 0 {
        return true;
    }
    // Non-Powered buildings are never deactivated.
    if !obj.powered {
        return true;
    }
    // Check if owner is in low power.
    let is_low = power_states
        .get(&entity.owner)
        .is_some_and(|state| state.is_low_power);
    !is_low
}

/// Trigger a spy-infiltration power blackout for the target owner.
///
/// Sets `power_blackout_remaining` to the configured duration from `[General]`.
/// While active, the owner's power output is forced to 0.
pub fn trigger_spy_blackout(
    power_states: &mut BTreeMap<InternedId, PowerState>,
    owner_id: InternedId,
    duration_frames: u32,
) {
    let state = power_states.entry(owner_id).or_default();
    state.power_blackout_remaining = duration_frames;
}

/// Check if the given owner has at least one active (powered) radar building.
pub fn has_active_radar(
    entities: &EntityStore,
    power_states: &BTreeMap<InternedId, PowerState>,
    rules: &RuleSet,
    owner_id: InternedId,
    interner: &crate::sim::intern::StringInterner,
) -> bool {
    entities.values().any(|e| {
        e.category == EntityCategory::Structure
            && e.owner == owner_id
            && e.building_up.is_none()
            && rules
                .object(interner.resolve(e.type_ref))
                .is_some_and(|obj| obj.radar || obj.spy_sat)
            && is_building_powered(power_states, rules, e, interner)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::intern;

    fn test_interner() -> intern::StringInterner {
        intern::test_interner()
    }

    /// Build a minimal RuleSet with the given INI text.
    fn rules_from_ini(text: &str) -> RuleSet {
        let ini = IniFile::from_str(text);
        RuleSet::from_ini(&ini).expect("test rules")
    }

    fn make_building(id: u64, type_ref: &str, owner: &str, hp: u16, max_hp: u16) -> GameEntity {
        let mut e = GameEntity::test_default(id, type_ref, owner, 10, 10);
        e.category = EntityCategory::Structure;
        e.health = Health {
            current: hp,
            max: max_hp,
        };
        e
    }

    fn test_rules() -> RuleSet {
        rules_from_ini(
            "\
[BuildingTypes]
0=GAPOWR
1=NAPOWR
2=TESLA
3=GAPILE

[GAPOWR]
Power=200
Strength=600
Powered=no

[NAPOWR]
Power=150
Strength=400
Powered=no

[TESLA]
Power=-75
Strength=400
Powered=yes

[GAPILE]
Power=-10
Strength=500
Powered=yes

[General]
DamageDelay=1.0
SpyPowerBlackout=1000
MinLowPowerProductionSpeed=0.5
MaxLowPowerProductionSpeed=0.8
LowPowerPenaltyModifier=1.25
BuildSpeed=0.02
",
        )
    }

    #[test]
    fn test_health_scaled_output() {
        let rules = test_rules();
        let mut store = EntityStore::new();
        // Power plant at 50% HP should produce 50% output.
        store.insert(make_building(1, "GAPOWR", "Allies", 300, 600));
        // Barracks at any health always drains full amount.
        store.insert(make_building(2, "GAPILE", "Allies", 50, 500));

        let mut state = PowerState::default();
        let interner = test_interner();
        let allies = intern::test_intern("Allies");
        recalculate_power_for_owner(&mut state, &store, &rules, allies, &interner);

        assert_eq!(state.total_output, 100, "200 * 300/600 = 100");
        assert_eq!(state.total_drain, 10, "|-10| = 10");
        assert!(!state.is_low_power, "100 >= 10");
    }

    #[test]
    fn test_full_health_full_output() {
        let rules = test_rules();
        let mut store = EntityStore::new();
        store.insert(make_building(1, "GAPOWR", "Allies", 600, 600));

        let mut state = PowerState::default();
        let interner = test_interner();
        let allies = intern::test_intern("Allies");
        recalculate_power_for_owner(&mut state, &store, &rules, allies, &interner);

        assert_eq!(state.total_output, 200);
        assert_eq!(state.total_drain, 0);
        assert!(!state.is_low_power);
    }

    #[test]
    fn test_low_power_detection() {
        let rules = test_rules();
        let mut store = EntityStore::new();
        // Small power plant at low health.
        store.insert(make_building(1, "NAPOWR", "Soviet", 40, 400)); // 150 * 40/400 = 15
        // Tesla Coil drains 75.
        store.insert(make_building(2, "TESLA", "Soviet", 400, 400));

        let mut state = PowerState::default();
        let interner = test_interner();
        let soviet = intern::test_intern("Soviet");
        recalculate_power_for_owner(&mut state, &store, &rules, soviet, &interner);

        assert_eq!(state.total_output, 15, "150 * 40/400 = 15");
        assert_eq!(state.total_drain, 75);
        assert!(state.is_low_power, "15 < 75");
    }

    #[test]
    fn test_drain_always_full_regardless_of_health() {
        let rules = test_rules();
        let mut store = EntityStore::new();
        // Tesla Coil at 1 HP still drains full 75.
        store.insert(make_building(1, "TESLA", "Soviet", 1, 400));

        let mut state = PowerState::default();
        let interner = test_interner();
        let soviet = intern::test_intern("Soviet");
        recalculate_power_for_owner(&mut state, &store, &rules, soviet, &interner);

        assert_eq!(state.total_drain, 75, "drain is always full rated value");
    }

    #[test]
    fn test_spy_blackout_forces_zero_output() {
        let rules = test_rules();
        let mut store = EntityStore::new();
        store.insert(make_building(1, "GAPOWR", "Allies", 600, 600));
        store.insert(make_building(2, "GAPILE", "Allies", 500, 500));

        let mut state = PowerState::default();
        state.power_blackout_remaining = 100;
        let interner = test_interner();
        let allies = intern::test_intern("Allies");
        recalculate_power_for_owner(&mut state, &store, &rules, allies, &interner);

        assert_eq!(state.total_output, 0, "blackout forces output to 0");
        assert_eq!(state.total_drain, 10);
        assert!(state.is_low_power, "0 < 10 during blackout");
    }

    #[test]
    fn test_spy_blackout_timer_decrements() {
        let rules = test_rules();
        let mut store = EntityStore::new();
        store.insert(make_building(1, "GAPOWR", "Allies", 600, 600));

        let interner = test_interner();
        let allies = intern::test_intern("Allies");
        let mut states: BTreeMap<InternedId, PowerState> = BTreeMap::new();
        trigger_spy_blackout(&mut states, allies, 5);

        // Tick 5 times — each tick decrements by 1.
        for _ in 0..5 {
            tick_power_states(&mut states, &mut store, &rules, 16, &interner);
        }

        let state = states.get(&allies).expect("state should exist");
        assert_eq!(state.power_blackout_remaining, 0, "timer should reach 0");
        assert!(!state.is_low_power, "power restored after blackout");
    }

    #[test]
    fn test_power_transition_events() {
        let rules = test_rules();
        let mut store = EntityStore::new();
        // Start with just a tesla coil (drain=75, output=0) → immediate low power.
        store.insert(make_building(1, "TESLA", "Soviet", 400, 400));

        // Pre-intern all strings that will be used (including NAPOWR for the second
        // building added later) so the interner clone has everything.
        let soviet = intern::test_intern("Soviet");
        intern::test_intern("NAPOWR");
        let interner = test_interner();
        let mut states: BTreeMap<InternedId, PowerState> = BTreeMap::new();

        let events = tick_power_states(&mut states, &mut store, &rules, 16, &interner);
        assert!(
            events.contains(&PowerEvent::EnteredLowPower { owner: soviet }),
            "should detect entering low power"
        );

        // Add a power plant → should restore power.
        store.insert(make_building(2, "NAPOWR", "Soviet", 400, 400));
        let events = tick_power_states(&mut states, &mut store, &rules, 16, &interner);
        assert!(
            events.contains(&PowerEvent::PowerRestored { owner: soviet }),
            "should detect power restored"
        );
    }

    #[test]
    fn test_is_building_powered_for_generator() {
        let rules = test_rules();
        let allies = intern::test_intern("Allies");

        // Power plant (positive Power=) is never deactivated.
        let plant = make_building(1, "GAPOWR", "Allies", 600, 600);

        // Get interner AFTER all strings are interned (make_building interns type_ref).
        let interner = test_interner();
        let mut states: BTreeMap<InternedId, PowerState> = BTreeMap::new();
        states.insert(
            allies,
            PowerState {
                is_low_power: true,
                ..PowerState::default()
            },
        );

        assert!(
            is_building_powered(&states, &rules, &plant, &interner),
            "generators are never deactivated"
        );
    }

    #[test]
    fn test_is_building_powered_for_consumer_during_low_power() {
        let rules = test_rules();
        let soviet = intern::test_intern("Soviet");
        let tesla = make_building(1, "TESLA", "Soviet", 400, 400);

        // Get interner AFTER all strings are interned.
        let interner = test_interner();
        let mut states: BTreeMap<InternedId, PowerState> = BTreeMap::new();
        states.insert(
            soviet,
            PowerState {
                is_low_power: true,
                ..PowerState::default()
            },
        );

        assert!(
            !is_building_powered(&states, &rules, &tesla, &interner),
            "Powered=yes consumer deactivated during low power"
        );
    }

    #[test]
    fn test_is_building_powered_for_consumer_during_surplus() {
        let rules = test_rules();
        let soviet = intern::test_intern("Soviet");
        let tesla = make_building(1, "TESLA", "Soviet", 400, 400);

        // Get interner AFTER all strings are interned.
        let interner = test_interner();
        let mut states: BTreeMap<InternedId, PowerState> = BTreeMap::new();
        states.insert(
            soviet,
            PowerState {
                is_low_power: false,
                ..PowerState::default()
            },
        );

        assert!(
            is_building_powered(&states, &rules, &tesla, &interner),
            "consumer active during power surplus"
        );
    }

    #[test]
    fn test_low_power_does_not_damage_buildings() {
        // gamemd does not apply degradation damage during low power — the
        // DamageDelay timer fields at HouseClass+0x578C/+0x5794 are written
        // in the constructor and never read. This test pins the Rust port
        // to that behavior.
        let rules = test_rules();
        let mut store = EntityStore::new();
        // Tesla coil (Powered=yes, Power=-75) with no power plant → sustained low power.
        store.insert(make_building(1, "TESLA", "Soviet", 100, 400));

        let interner = test_interner();
        let mut states: BTreeMap<InternedId, PowerState> = BTreeMap::new();

        // Tick well past any prior degradation threshold.
        for _ in 0..3750 {
            tick_power_states(&mut states, &mut store, &rules, 16, &interner);
        }

        let entity = store.get(1).expect("entity should exist");
        assert_eq!(
            entity.health.current, 100,
            "low power must not damage buildings (gamemd parity)"
        );
    }

    #[test]
    fn test_building_under_construction_excluded() {
        let rules = test_rules();
        let mut store = EntityStore::new();
        let mut plant = make_building(1, "GAPOWR", "Allies", 600, 600);
        plant.building_up = Some(crate::sim::components::BuildingUp {
            elapsed_ticks: 0,
            total_ticks: 30,
        });
        store.insert(plant);

        let mut state = PowerState::default();
        let interner = test_interner();
        let allies = intern::test_intern("Allies");
        recalculate_power_for_owner(&mut state, &store, &rules, allies, &interner);

        assert_eq!(
            state.total_output, 0,
            "building under construction produces no power"
        );
    }

    #[test]
    fn test_has_active_radar_with_power() {
        // Need a radar building in the rules.
        let rules = rules_from_ini(
            "\
[BuildingTypes]
0=GAAIRC
1=GAPOWR

[GAAIRC]
Radar=yes
Power=-50
Strength=600
Powered=yes

[GAPOWR]
Power=200
Strength=600

[General]
BuildSpeed=0.02
",
        );
        let mut store = EntityStore::new();
        store.insert(make_building(1, "GAAIRC", "Allies", 600, 600));
        store.insert(make_building(2, "GAPOWR", "Allies", 600, 600));

        let interner = test_interner();
        let allies = intern::test_intern("Allies");
        let mut states: BTreeMap<InternedId, PowerState> = BTreeMap::new();
        tick_power_states(&mut states, &mut store, &rules, 16, &interner);

        assert!(
            has_active_radar(&store, &states, &rules, allies, &interner),
            "radar should be active with sufficient power"
        );

        // Remove the power plant → low power → radar disabled.
        store.remove(2);
        tick_power_states(&mut states, &mut store, &rules, 16, &interner);

        assert!(
            !has_active_radar(&store, &states, &rules, allies, &interner),
            "radar should be disabled during low power"
        );
    }

    // -------- ExtraPower (Yuri Bio-Reactor) bonus tests -----------------

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

    /// YAPOWR test entity with `n` garrisoned passengers and given hp/max.
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

    #[test]
    fn test_yapowr_empty_no_bonus() {
        let rules = yapowr_rules();
        let mut store = EntityStore::new();
        store.insert(make_yapowr(1, "Yuri", 750, 750, 0));

        let yuri = intern::test_intern("Yuri");
        let interner = test_interner();
        let mut state = PowerState::default();
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        assert_eq!(state.total_output, 150, "empty YAPOWR = base Power only");
        assert_eq!(state.total_drain, 0);
    }

    #[test]
    fn test_yapowr_garrisoned_full_hp() {
        let rules = yapowr_rules();
        let mut store = EntityStore::new();
        store.insert(make_yapowr(1, "Yuri", 750, 750, 5));

        let yuri = intern::test_intern("Yuri");
        let interner = test_interner();
        let mut state = PowerState::default();
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        assert_eq!(state.total_output, 650, "150 + 100*5 = 650 at full HP");
    }

    #[test]
    fn test_yapowr_garrisoned_half_hp_scales_bonus() {
        let rules = yapowr_rules();
        let mut store = EntityStore::new();
        store.insert(make_yapowr(1, "Yuri", 375, 750, 5));

        let yuri = intern::test_intern("Yuri");
        let interner = test_interner();
        let mut state = PowerState::default();
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        // (150 + 500) * 375 / 750 = 650 * 375 / 750 = 325
        assert_eq!(state.total_output, 325, "bonus scales with HP");
    }

    #[test]
    fn test_no_infantry_absorb_no_bonus() {
        // GAPOWR has no InfantryAbsorb/UnitAbsorb. A stray passenger
        // (which the garrison flow would never produce) must NOT grant
        // a bonus — the gate is on the TypeClass flags.
        let rules = yapowr_rules();
        let mut store = EntityStore::new();
        let mut e = make_building(1, "GAPOWR", "Allies", 600, 600);
        let mut cargo = crate::sim::passenger::PassengerCargo::new(5, 0);
        cargo.passengers.push(100);
        cargo.total_size += 1;
        e.passenger_role = crate::sim::passenger::PassengerRole::Transport { cargo };
        store.insert(e);

        let allies = intern::test_intern("Allies");
        let interner = test_interner();
        let mut state = PowerState::default();
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
        let mut e = make_building(1, "ZEROEX", "Yuri", 750, 750);
        let mut cargo = crate::sim::passenger::PassengerCargo::new(5, 0);
        for i in 0..5 {
            cargo.passengers.push(100 + i as u64);
            cargo.total_size += 1;
        }
        e.passenger_role = crate::sim::passenger::PassengerRole::Transport { cargo };
        store.insert(e);

        let yuri = intern::test_intern("Yuri");
        let interner = test_interner();
        let mut state = PowerState::default();
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
            cargo.passengers.push(100 + i as u64);
            cargo.total_size += 1;
        }
        e.passenger_role = crate::sim::passenger::PassengerRole::Transport { cargo };
        store.insert(e);

        let yuri = intern::test_intern("Yuri");
        let interner = test_interner();
        let mut state = PowerState::default();
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        assert_eq!(state.total_output, 150, "ExtraPower<0 fails strict > 0 gate");
    }

    #[test]
    fn test_unit_absorb_path_also_works() {
        // gamemd gate is (UnitAbsorb || InfantryAbsorb). UnitAbsorb alone
        // (no InfantryAbsorb) still grants the bonus.
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
            cargo.passengers.push(100 + i as u64);
            cargo.total_size += 1;
        }
        e.passenger_role = crate::sim::passenger::PassengerRole::Transport { cargo };
        store.insert(e);

        let yuri = intern::test_intern("Yuri");
        let interner = test_interner();
        let mut state = PowerState::default();
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

        let yuri = intern::test_intern("Yuri");
        let interner = test_interner();
        let mut state = PowerState::default();
        recalculate_power_for_owner(&mut state, &store, &rules, yuri, &interner);

        assert_eq!(
            state.total_output, 0,
            "building_up suppresses all output including bonus"
        );
    }
}
