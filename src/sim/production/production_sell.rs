//! Building sell/repair logic: refund calculation, crew ejection, repair tick.
//!
//! Extracted from production_placement.rs for file-size limits.

use crate::map::entities::EntityCategory;
use crate::rules::object_type::ObjectCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::DestroyedGarrisonBuilding;
use crate::sim::components::{Health, Position};
use crate::sim::intern::InternedId;
use crate::sim::movement;
use crate::sim::occupancy::CellListInsertion;
use crate::sim::passenger::PassengerRole;
use crate::sim::world::Simulation;
use crate::util::fixed_math::ra2_speed_to_leptons_per_second;
use crate::util::lepton;

use super::production_queue::{credits_entry_for_owner, credits_for_owner};
use super::production_tech::foundation_dimensions;

/// RA2 sell refund: 50% of cost (integer percentage).
const SELL_REFUND_PERCENT: u32 = 50;

/// Health as integer percentage (0–100).
fn health_percent(current: u16, max: u16) -> u32 {
    if max == 0 {
        return 100;
    }
    ((current as u32) * 100 / max as u32).min(100)
}

fn sell_refund_for_building(
    obj: &crate::rules::object_type::ObjectType,
    health: Option<Health>,
) -> i32 {
    let hp_pct: u32 = health
        .map(|hp| health_percent(hp.current, hp.max))
        .unwrap_or(100);
    // refund = cost * sell% * health% / 10000
    (obj.cost.max(0) as u64 * SELL_REFUND_PERCENT as u64 * hp_pct as u64 / 10000) as i32
}

/// Survivor divisor for the given owner's side, from `[General]` INI keys.
/// Uses HouseState.side_index (0=Allied, 1=Soviet, 2=Yuri) instead of
/// the old string-matching classify_owner_side hack.
fn survivor_divisor_for_owner(sim: &Simulation, rules: &RuleSet, owner: &str) -> i32 {
    let side = sim
        .interner
        .get(owner)
        .and_then(|id| sim.houses.get(&id))
        .map(|h| h.side_index)
        .unwrap_or(0);
    match side {
        1 => rules.general.soviet_survivor_divisor,
        2 => rules.general.third_survivor_divisor,
        _ => rules.general.allied_survivor_divisor,
    }
}

/// Compute survivor count using the RA2 formula: sell_refund / SurvivorDivisor.
///
/// The original engine divides the health-scaled sell refund by a per-side
/// divisor from `[General]`. Buildings at 0 HP produce no survivors. The
/// `Crewed=yes` flag must be set.
fn sell_survivor_limit(
    sim: &Simulation,
    obj: &crate::rules::object_type::ObjectType,
    health: Option<Health>,
    rules: &RuleSet,
    owner: &str,
) -> usize {
    if !obj.crewed {
        return 0;
    }
    let refund = sell_refund_for_building(obj, health);
    if refund <= 0 {
        return 0;
    }
    let divisor = survivor_divisor_for_owner(sim, rules, owner).max(1);
    (refund / divisor).max(0) as usize
}

fn sell_survivor_type(sim: &Simulation, rules: &RuleSet, owner: &str) -> Option<String> {
    let side = sim
        .interner
        .get(owner)
        .and_then(|id| sim.houses.get(&id))
        .map(|h| h.side_index)
        .unwrap_or(0);
    let mut preferred: Vec<&str> = match side {
        2 => vec!["INIT", "E2", "E1"],
        1 => vec!["E2", "E1", "INIT"],
        _ => vec!["E1", "E2", "INIT"],
    };
    preferred.extend(rules.infantry_ids.iter().map(String::as_str));

    preferred.into_iter().find_map(|id| {
        let obj = rules.object(id)?;
        if obj.category != ObjectCategory::Infantry {
            return None;
        }
        if !obj.owner.is_empty() && !obj.owner.iter().any(|h| h.eq_ignore_ascii_case(owner)) {
            return None;
        }
        Some(id.to_string())
    })
}

fn sell_survivor_positions(rx: u16, ry: u16, width: u16, height: u16) -> Vec<(u16, u16)> {
    let mut cells = Vec::new();
    let min_x = i32::from(rx) - 1;
    let max_x = i32::from(rx) + i32::from(width);
    let min_y = i32::from(ry) - 1;
    let max_y = i32::from(ry) + i32::from(height);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if x < 0 || y < 0 {
                continue;
            }
            let inside_x = x >= i32::from(rx) && x < i32::from(rx) + i32::from(width);
            let inside_y = y >= i32::from(ry) && y < i32::from(ry) + i32::from(height);
            if inside_x && inside_y {
                continue;
            }
            cells.push((x as u16, y as u16));
        }
    }

    cells.sort_by_key(|&(cx, cy)| {
        let dx = i32::from(cx) - (i32::from(rx) + i32::from(width) - 1);
        let dy = i32::from(cy) - (i32::from(ry) + i32::from(height) - 1);
        let dist_sq = dx * dx + dy * dy;
        (dist_sq, cy, cx)
    });
    cells
}

fn eject_sell_survivors(
    sim: &mut Simulation,
    rules: &RuleSet,
    owner: &str,
    building_type: &crate::rules::object_type::ObjectType,
    building_pos: Position,
    health: Option<Health>,
) -> usize {
    let Some(infantry_type) = sell_survivor_type(sim, rules, owner) else {
        return 0;
    };
    let survivor_limit = sell_survivor_limit(sim, building_type, health, rules, owner);
    if survivor_limit == 0 {
        return 0;
    }

    let (width, height) = foundation_dimensions(&building_type.foundation);
    let mut spawned = 0;
    for (spawn_rx, spawn_ry) in
        sell_survivor_positions(building_pos.rx, building_pos.ry, width, height)
            .into_iter()
            .take(survivor_limit)
    {
        if sim
            .spawn_object_at_height(
                &infantry_type,
                owner,
                spawn_rx,
                spawn_ry,
                64,
                building_pos.z,
                rules,
            )
            .is_some()
        {
            spawned += 1;
        }
    }
    spawned
}

/// Eject survivors from a crewed building destroyed in combat.
///
/// In the original RA2 engine, destroyed crewed buildings always eject at
/// least one infantry survivor regardless of the building's remaining HP (which
/// is 0). The survivor type is side-dependent (E1 for Allied, E2 for Soviet,
/// INIT for Yuri).
pub fn eject_destruction_survivors(
    sim: &mut Simulation,
    rules: &RuleSet,
    type_id: InternedId,
    owner: InternedId,
    rx: u16,
    ry: u16,
    z: u8,
) -> usize {
    let type_str = sim.interner.resolve(type_id);
    let owner_str = sim.interner.resolve(owner);
    let Some(obj) = rules.object(type_str) else {
        return 0;
    };
    if !obj.crewed {
        return 0;
    }
    let Some(infantry_type) = sell_survivor_type(sim, rules, owner_str) else {
        return 0;
    };
    // Clone owner string for spawn calls — rare path (building destruction only).
    let owner_owned = owner_str.to_string();
    let (width, height) = foundation_dimensions(&obj.foundation);
    // Always eject at least 1 survivor on destruction.
    let positions = sell_survivor_positions(rx, ry, width, height);
    let mut spawned = 0;
    for (spawn_rx, spawn_ry) in positions.into_iter().take(1) {
        if sim
            .spawn_object_at_height(
                &infantry_type,
                &owner_owned,
                spawn_rx,
                spawn_ry,
                64,
                z,
                rules,
            )
            .is_some()
        {
            spawned += 1;
        }
    }
    spawned
}

/// 8-directional neighbor offsets for scatter moves (same as passenger.rs).
const NEIGHBORS: [(i16, i16); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

fn eject_garrison_passengers_at_edges(
    sim: &mut Simulation,
    rules: &RuleSet,
    rx: u16,
    ry: u16,
    z: u8,
    width: u16,
    height: u16,
    passenger_ids: &[u64],
    owner_override: Option<InternedId>,
) -> usize {
    if passenger_ids.is_empty() || width == 0 || height == 0 {
        return 0;
    }

    let exit_cells = sell_survivor_positions(rx, ry, width, height);

    // Collect currently occupied cells to avoid stacking.
    let occupied_cells: Vec<(u16, u16)> = sim
        .entities
        .values()
        .filter(|e| !e.passenger_role.is_inside_transport() && !e.dying && e.is_alive())
        .map(|e| (e.position.rx, e.position.ry))
        .collect();

    let mut ejected: usize = 0;
    let mut used_cells: Vec<(u16, u16)> = Vec::new();

    // Iterate in reverse (LIFO); gamemd walks occupants high index to low.
    for &pax_id in passenger_ids.iter().rev() {
        let exit = exit_cells.iter().find(|&&(cx, cy)| {
            !occupied_cells.iter().any(|&(ox, oy)| ox == cx && oy == cy)
                && !used_cells.iter().any(|&(ux, uy)| ux == cx && uy == cy)
        });
        let Some(&(exit_rx, exit_ry)) = exit else {
            // All cells blocked; no parachute fallback exists in this helper yet.
            if let Some(pax) = sim.entities.get_mut(pax_id) {
                pax.health.current = 0;
                pax.dying = true;
                pax.passenger_role = PassengerRole::None;
            }
            continue;
        };
        used_cells.push((exit_rx, exit_ry));

        let Some(pax) = sim.entities.get_mut(pax_id) else {
            continue;
        };
        pax.passenger_role = PassengerRole::None;
        if let Some(owner) = owner_override {
            pax.owner = owner;
        }
        pax.position.rx = exit_rx;
        pax.position.ry = exit_ry;
        pax.position.z = z;
        let (sub_x, sub_y) = lepton::subcell_lepton_offset(pax.sub_cell);
        pax.position.sub_x = sub_x;
        pax.position.sub_y = sub_y;
        pax.position.refresh_screen_coords();
        let pax_sub_cell = pax.sub_cell;

        sim.occupancy.add(
            exit_rx,
            exit_ry,
            pax_id,
            crate::sim::movement::locomotor::MovementLayer::Ground,
            pax_sub_cell,
            CellListInsertion::PrependNonBuilding,
        );

        // Approximate gamemd's queued Scatter mission with the current direct
        // move helper until the mission queue has a verified Scatter path.
        let scatter_speed = sim
            .entities
            .get(pax_id)
            .and_then(|e| rules.object(sim.interner.resolve(e.type_ref)))
            .map(|obj| ra2_speed_to_leptons_per_second(obj.speed))
            .unwrap_or(ra2_speed_to_leptons_per_second(4));
        let start_dir = sim.rng.next_u32() as usize % 8;
        for i in 0..8 {
            let (dx, dy) = NEIGHBORS[(start_dir + i) % 8];
            let sx = exit_rx as i32 + dx as i32;
            let sy = exit_ry as i32 + dy as i32;
            if sx >= 0 && sy >= 0 {
                let dest = (sx as u16, sy as u16);
                let blocked = occupied_cells
                    .iter()
                    .any(|&(ox, oy)| ox == dest.0 && oy == dest.1)
                    || used_cells
                        .iter()
                        .any(|&(ux, uy)| ux == dest.0 && uy == dest.1);
                if !blocked {
                    movement::issue_direct_move(&mut sim.entities, pax_id, dest, scatter_speed);
                    break;
                }
            }
        }
        ejected += 1;
    }

    ejected
}

/// Eject garrison occupants from a building being sold, placing each infantry
/// at an adjacent free cell. Matches gamemd `SellBuilding @ 0x00457DE0` which
/// iterates occupants in LIFO order and Unlimbos them at foundation edges.
///
/// Returns the number of occupants successfully ejected.
fn eject_garrison_occupants(sim: &mut Simulation, rules: &RuleSet, building_id: u64) -> usize {
    // Snapshot building data before mutation.
    let (rx, ry, z, width, height, passenger_ids) = {
        let entity = match sim.entities.get(building_id) {
            Some(e) => e,
            None => return 0,
        };
        let cargo = match entity.passenger_role.cargo() {
            Some(c) if !c.is_empty() => c,
            _ => return 0,
        };
        let obj = match rules.object(sim.interner.resolve(entity.type_ref)) {
            Some(o) => o,
            None => return 0,
        };
        let (fw, fh) = foundation_dimensions(&obj.foundation);
        (
            entity.position.rx,
            entity.position.ry,
            entity.position.z,
            fw,
            fh,
            cargo.passengers.clone(),
        )
    };

    let ejected = eject_garrison_passengers_at_edges(
        sim,
        rules,
        rx,
        ry,
        z,
        width,
        height,
        &passenger_ids,
        None,
    );

    // Clear player-sell cargo only. Native SellBuilding is an ejection helper;
    // empty-garrison ownership reversion belongs to reconciliation/unload.
    if let Some(building) = sim.entities.get_mut(building_id) {
        if let Some(cargo) = building.passenger_role.cargo_mut() {
            cargo.passengers.clear();
            cargo.total_size = 0;
            cargo.garrison_fire_index = 0;
        }
    }

    ejected
}

/// Eject garrison occupants from a building destroyed in combat.
///
/// Verified gamemd evidence routes destroyed `CanBeOccupied` garrisons through
/// the same SellBuilding helper used by sell/abandon. With current helper
/// support this means LIFO occupants use the existing foundation-edge scan; no
/// shuffled interior-foundation placement is performed here. The building has
/// already been removed, so destruction does not restore `garrison_original_owner`.
///
/// Returns the count of occupants successfully ejected (excludes those killed
/// when no edge cell can be used).
pub fn eject_destruction_garrison(
    sim: &mut Simulation,
    rules: &RuleSet,
    event: &DestroyedGarrisonBuilding,
) -> usize {
    eject_garrison_passengers_at_edges(
        sim,
        rules,
        event.rx,
        event.ry,
        event.z,
        event.foundation_w,
        event.foundation_h,
        &event.passenger_ids,
        Some(event.owner),
    )
}

/// Sell a building entity: refund part of its current value, eject crew, and despawn it.
///
/// Captured civilian `CanBeOccupied` garrisons use the same player-sell
/// transaction once they are owned by the seller: occupants eject through
/// the SellBuilding-style helper, then the building is removed/refunded.
/// Revert-to-civilian belongs to empty-garrison reconciliation, not player sell.
pub fn sell_building(sim: &mut Simulation, rules: &RuleSet, stable_id: u64) -> bool {
    let (owner_name, type_id, position, health) = {
        let Some(entity) = sim.entities.get(stable_id) else {
            return false;
        };
        if entity.category != EntityCategory::Structure {
            return false;
        }
        (
            sim.interner.resolve(entity.owner).to_string(),
            sim.interner.resolve(entity.type_ref).to_string(),
            entity.position.clone(),
            Some(entity.health),
        )
    };
    let Some(obj) = rules.object(&type_id) else {
        return false;
    };

    let refund = sell_refund_for_building(obj, health);
    let ejected = eject_sell_survivors(sim, rules, &owner_name, obj, position, health);
    // Eject garrison occupants alive before removing the building (gamemd SellBuilding).
    let garrison_ejected = eject_garrison_occupants(sim, rules, stable_id);
    let interrupted_miners =
        crate::sim::miner::interrupt_refinery_docked_miners(sim, rules, stable_id);
    // Remove from EntityStore.
    sim.clear_radio_contacts_for(stable_id);
    sim.entities.remove(stable_id);
    // SpySat sold: fully reshroud the owner so only current LOS remains visible.
    let owner_id = sim.interner.intern(&owner_name);
    if obj.spy_sat {
        sim.fog.reset_explored_for_owner(owner_id);
    }
    // Refresh superweapon grants — sold building may have been providing a SW.
    if sim.game_options.super_weapons {
        crate::sim::superweapon::refresh_super_weapons_for_owner(sim, rules, owner_id);
    }
    if refund > 0 {
        *credits_entry_for_owner(sim, &owner_name) += refund;
    }
    log::info!(
        "Building {} sold by {}: refunded {} credits, ejected {} crew + {} garrison, undocked {} miners",
        type_id,
        owner_name,
        refund,
        ejected,
        garrison_ejected,
        interrupted_miners
    );
    true
}

/// Toggle repair mode on a building. If already repairing, stop. Otherwise start.
pub fn toggle_repair(sim: &mut Simulation, stable_id: u64) -> bool {
    let Some(entity) = sim.entities.get_mut(stable_id) else {
        return false;
    };
    if entity.category != EntityCategory::Structure {
        return false;
    }
    if entity.repairing {
        entity.repairing = false;
        log::info!("Repair stopped on entity {}", stable_id);
    } else {
        entity.repairing = true;
        log::info!("Repair started on entity {}", stable_id);
    }
    true
}

/// Repair cost: 25% of building cost spread across all HP.
const REPAIR_COST_PERCENT: u32 = 25;
/// HP healed per sim tick (at 15 Hz this is ~60 HP/sec).
const REPAIR_HP_PER_TICK: u16 = 4;

/// Tick all repairing buildings: heal HP and deduct credits.
pub fn tick_repairs(sim: &mut Simulation, rules: &RuleSet) {
    // Collect snapshot of repairing structures.
    let actions: Vec<(u64, String, String, u16, u16)> = sim
        .entities
        .values()
        .filter(|e| {
            e.repairing
                && e.category == EntityCategory::Structure
                && e.health.current < e.health.max
        })
        .map(|e| {
            (
                e.stable_id,
                sim.interner.resolve(e.owner).to_string(),
                sim.interner.resolve(e.type_ref).to_string(),
                e.health.current,
                e.health.max,
            )
        })
        .collect();
    let mut stop_repairing: Vec<u64> = Vec::new();
    for (stable_id, owner, type_id, current_hp, max_hp) in actions {
        let cost_per_hp: i32 = rules
            .object(&type_id)
            .map(|obj| {
                // total_repair_cost = cost * 25 / 100, then / max_hp (ceiling division)
                let total_repair_cost: u32 = obj.cost.max(0) as u32 * REPAIR_COST_PERCENT / 100;
                total_repair_cost.div_ceil(max_hp.max(1) as u32).max(1) as i32
            })
            .unwrap_or(1);
        let credits = credits_for_owner(sim, &owner);
        if credits < cost_per_hp {
            stop_repairing.push(stable_id);
            continue;
        }
        let heal = REPAIR_HP_PER_TICK.min(max_hp - current_hp);
        if heal == 0 {
            stop_repairing.push(stable_id);
            continue;
        }
        *credits_entry_for_owner(sim, &owner) -= cost_per_hp * heal as i32;
        if let Some(entity) = sim.entities.get_mut(stable_id) {
            entity.health.current = (entity.health.current + heal).min(entity.health.max);
            if entity.health.current >= entity.health.max {
                stop_repairing.push(stable_id);
            }
        }
    }
    for stable_id in stop_repairing {
        if let Some(entity) = sim.entities.get_mut(stable_id) {
            entity.repairing = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::game_entity::GameEntity;

    fn garrison_edge_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "[InfantryTypes]\n\
             0=E1\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             0=CAGAS01\n\
             [E1]\n\
             Name=GI\n\
             Cost=200\n\
             Strength=100\n\
             Armor=flak\n\
             Speed=4\n\
             Sight=5\n\
             TechLevel=1\n\
             Owner=Americans,Neutral\n\
             Occupier=yes\n\
             Size=1\n\
             [CAGAS01]\n\
             Name=GasStation\n\
             Cost=400\n\
             Strength=400\n\
             Armor=wood\n\
             Foundation=2x2\n\
             CanBeOccupied=yes\n\
             CanOccupyFire=yes\n\
             MaxNumberOccupants=5\n",
        );
        RuleSet::from_ini(&ini).expect("garrison edge rules should parse")
    }

    fn insert_hidden_passenger(
        sim: &mut Simulation,
        stable_id: u64,
        transport_id: u64,
        owner: &str,
    ) -> u64 {
        let mut pax = GameEntity::test_default(stable_id, "E1", owner, 0, 0);
        pax.owner = sim.interner.intern(owner);
        pax.type_ref = sim.interner.intern("E1");
        pax.passenger_role = PassengerRole::Inside { transport_id };
        sim.entities.insert(pax);
        stable_id
    }

    fn insert_captured_player_owned_garrison(
        sim: &mut Simulation,
        building_id: u64,
        passenger_id: u64,
    ) {
        let americans = sim.interner.intern("Americans");
        let neutral = sim.interner.intern("Neutral");

        let mut building = GameEntity::test_default(building_id, "CAGAS01", "Americans", 10, 10);
        building.category = EntityCategory::Structure;
        building.owner = americans;
        building.type_ref = sim.interner.intern("CAGAS01");
        building.garrison_original_owner = Some(neutral);
        building.passenger_role = PassengerRole::Transport {
            cargo: crate::sim::passenger::PassengerCargo::new(5, 1),
        };
        if let Some(cargo) = building.passenger_role.cargo_mut() {
            assert!(cargo.board(passenger_id, 1));
        }
        sim.entities.insert(building);

        insert_hidden_passenger(sim, passenger_id, building_id, "Americans");
    }

    #[test]
    fn captured_civilian_garrison_player_sell_removes_building_and_refunds() {
        let rules = garrison_edge_rules();
        let mut sim = Simulation::new();
        let building_id = 10;
        let passenger_id = 11;
        insert_captured_player_owned_garrison(&mut sim, building_id, passenger_id);

        let before = credits_for_owner(&sim, "Americans");

        assert!(sell_building(&mut sim, &rules, building_id));

        assert!(sim.entities.get(building_id).is_none());
        assert_eq!(credits_for_owner(&sim, "Americans") - before, 200);

        let passenger = sim
            .entities
            .get(passenger_id)
            .expect("passenger should survive sell eject");
        assert!(matches!(passenger.passenger_role, PassengerRole::None));
        assert!(!passenger.dying);
        assert!(
            passenger.position.rx < 10
                || passenger.position.rx > 11
                || passenger.position.ry < 10
                || passenger.position.ry > 11,
            "passenger should be ejected outside the 2x2 foundation"
        );

        assert!(
            !sim.sound_events.iter().any(|event| {
                matches!(
                    event,
                    crate::sim::world::SimSoundEvent::StructureAbandoned { .. }
                )
            }),
            "player sell must not emit StructureAbandoned"
        );
    }

    #[test]
    fn sellbuilding_helper_ejects_without_owner_revert() {
        let rules = garrison_edge_rules();
        let mut sim = Simulation::new();
        let building_id = 20;
        let passenger_id = 21;
        insert_captured_player_owned_garrison(&mut sim, building_id, passenger_id);

        let americans = sim.interner.intern("Americans");
        let neutral = sim.interner.intern("Neutral");

        assert_eq!(eject_garrison_occupants(&mut sim, &rules, building_id), 1);

        let building = sim
            .entities
            .get(building_id)
            .expect("helper should not remove building");
        assert_eq!(
            building.owner, americans,
            "SellBuilding-style helper must not ChangeOwner"
        );
        assert_eq!(
            building.garrison_original_owner,
            Some(neutral),
            "helper must not consume reconciliation state during player-sell ejection"
        );
        assert!(
            building
                .passenger_role
                .cargo()
                .is_some_and(|cargo| cargo.is_empty()),
            "helper should clear building cargo"
        );

        let passenger = sim
            .entities
            .get(passenger_id)
            .expect("passenger should remain");
        assert!(matches!(passenger.passenger_role, PassengerRole::None));
        assert!(!passenger.dying);
    }

    #[test]
    fn destroyed_garrison_uses_sell_edge_scan_and_lifo_order() {
        let rules = garrison_edge_rules();
        let mut sim = Simulation::new();
        let building_id = 10;
        let pax1 = insert_hidden_passenger(&mut sim, 11, building_id, "Neutral");
        let pax2 = insert_hidden_passenger(&mut sim, 12, building_id, "Neutral");
        let pax3 = insert_hidden_passenger(&mut sim, 13, building_id, "Neutral");
        let owner = sim.interner.intern("Americans");

        let event = DestroyedGarrisonBuilding {
            building_id,
            type_id: sim.interner.intern("CAGAS01"),
            owner,
            rx: 10,
            ry: 10,
            z: 0,
            foundation_w: 2,
            foundation_h: 2,
            passenger_ids: vec![pax1, pax2, pax3],
        };

        assert_eq!(eject_destruction_garrison(&mut sim, &rules, &event), 3);

        let expected = sell_survivor_positions(10, 10, 2, 2);
        let checks = [
            (pax3, expected[0]),
            (pax2, expected[1]),
            (pax1, expected[2]),
        ];

        for (pax_id, expected_cell) in checks {
            let pax = sim.entities.get(pax_id).expect("passenger should remain");
            assert_eq!(
                (pax.position.rx, pax.position.ry),
                expected_cell,
                "destroyed garrison should use the sell edge scan in LIFO order"
            );
            assert!(
                pax.position.rx < 10
                    || pax.position.rx > 11
                    || pax.position.ry < 10
                    || pax.position.ry > 11,
                "destroyed garrison should not place passengers inside the foundation"
            );
            assert_eq!(
                pax.owner, owner,
                "destruction keeps death-time building owner"
            );
            assert!(matches!(pax.passenger_role, PassengerRole::None));
            assert!(!pax.dying);
        }
    }
}
