//! Passenger/transport system — boarding, unloading, and cargo tracking.
//!
//! Handles infantry entering transports (Passengers>0), building garrisons
//! (CanBeOccupied=yes), IFV weapon swapping (Gunner=yes), and passenger
//! death on transport destruction.
//!
//! ## Original engine reference
//! The original engine uses a linked-list at offsets +0x1D0/+0x1CC for passenger
//! storage; we use Vec<u64> for simplicity.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/ (ObjectType), sim/game_entity, sim/entity_store.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::OrderIntent;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::StringInterner;
use crate::sim::movement;
use crate::sim::occupancy::CellListInsertion;
use crate::sim::world::{SimSoundEvent, Simulation};
use crate::util::fixed_math::ra2_speed_to_leptons_per_second;
use crate::util::lepton;

/// Passenger cargo state, attached as `Option<PassengerCargo>` on transport entities.
///
/// Tracks which entities are currently inside this transport/garrison.
/// Passengers are stored as a Vec of stable_ids for deterministic ordering.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PassengerCargo {
    /// Stable IDs of entities currently inside, in boarding order (FIFO unload).
    pub passengers: Vec<u64>,
    /// Maximum passenger count (from Passengers= or MaxNumberOccupants= in rules.ini).
    pub capacity: u32,
    /// Maximum Size= of individual passenger allowed (from SizeLimit= in rules.ini).
    /// 0 means no size restriction.
    pub size_limit: u32,
    /// Total Size units currently occupied (sum of passenger Size= values).
    pub total_size: u32,
    /// Round-robin index for garrison fire — which occupant fires next.
    /// Matches gamemd BuildingClass+0x69C (CurrentFireIdx). Init 0, advanced
    /// by garrison combat after each shot: `(idx + 1) % occupant_count`.
    pub garrison_fire_index: u8,
}

impl PassengerCargo {
    pub fn new(capacity: u32, size_limit: u32) -> Self {
        Self {
            passengers: Vec::new(),
            capacity,
            size_limit,
            total_size: 0,
            garrison_fire_index: 0,
        }
    }

    /// Number of passengers currently inside.
    pub fn count(&self) -> u32 {
        self.passengers.len() as u32
    }

    /// Whether the transport has room for a passenger of the given size.
    pub fn can_accept(&self, passenger_size: u32) -> bool {
        self.count() < self.capacity && (self.size_limit == 0 || passenger_size <= self.size_limit)
    }

    /// Add a passenger. Returns false if full or too large.
    pub fn board(&mut self, stable_id: u64, passenger_size: u32) -> bool {
        if !self.can_accept(passenger_size) {
            return false;
        }
        self.passengers.push(stable_id);
        self.total_size += passenger_size;
        true
    }

    /// Add a passenger without normal transport capacity/size gates.
    ///
    /// Standard paradrop superweapon loading uses CargoClass::AddPassenger on
    /// limbo-created infantry; it is driven by `*ParaDropNum`, not by PDPLANE's
    /// `Passengers=` or `SizeLimit=`.
    pub fn board_forced(&mut self, stable_id: u64, passenger_size: u32) {
        self.passengers.push(stable_id);
        self.total_size += passenger_size;
    }

    /// Remove a specific passenger. Returns true if found and removed.
    pub fn disembark(&mut self, stable_id: u64, passenger_size: u32) -> bool {
        if let Some(pos) = self.passengers.iter().position(|&id| id == stable_id) {
            self.passengers.remove(pos);
            self.total_size = self.total_size.saturating_sub(passenger_size);
            true
        } else {
            false
        }
    }

    /// Remove and return the first passenger (FIFO unload order).
    pub fn unload_first(&mut self) -> Option<u64> {
        if self.passengers.is_empty() {
            None
        } else {
            let id = self.passengers.remove(0);
            // total_size is corrected by the caller who knows the passenger's size
            Some(id)
        }
    }

    /// Is the cargo hold empty?
    pub fn is_empty(&self) -> bool {
        self.passengers.is_empty()
    }
}

/// Boarding intent phase — tracks a passenger's approach to a transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BoardingPhase {
    /// Moving toward the transport cell.
    Approach,
    /// Adjacent to transport, entering this tick.
    Entering,
}

/// Passenger/transport role for an entity. Replaces three separate Option fields
/// with a single enum that makes invalid states unrepresentable.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PassengerRole {
    /// Entity has no passenger/transport role. Most entities are this.
    None,
    /// Entity is a transport or garrisonable building that can hold passengers.
    Transport { cargo: PassengerCargo },
    /// Entity is approaching a transport to board it.
    Boarding {
        target_transport_id: u64,
        phase: BoardingPhase,
    },
    /// Entity is inside a transport (hidden from map, not targetable).
    Inside { transport_id: u64 },
}

impl PassengerRole {
    /// Returns the cargo hold if this entity is a transport.
    pub fn cargo(&self) -> Option<&PassengerCargo> {
        match self {
            Self::Transport { cargo } => Some(cargo),
            _ => Option::None,
        }
    }

    /// Returns a mutable reference to the cargo hold if this entity is a transport.
    pub fn cargo_mut(&mut self) -> Option<&mut PassengerCargo> {
        match self {
            Self::Transport { cargo } => Some(cargo),
            _ => Option::None,
        }
    }

    /// Returns the transport ID if this entity is inside one.
    pub fn inside_transport_id(&self) -> Option<u64> {
        match self {
            Self::Inside { transport_id } => Some(*transport_id),
            _ => Option::None,
        }
    }

    /// True if entity is inside a transport (hidden from map).
    pub fn is_inside_transport(&self) -> bool {
        matches!(self, Self::Inside { .. })
    }

    /// True if entity is a transport/garrison with a cargo hold.
    pub fn is_transport(&self) -> bool {
        matches!(self, Self::Transport { .. })
    }
}

/// Check whether a passenger entity can enter a specific transport.
///
/// Validates: alive, not already transported, owner compatibility,
/// size fits, transport has room.  For garrison buildings
/// (`CanBeOccupied=yes`) additional checks apply: `Occupier=yes`
/// required on the infantry, and the building must not be at red health.
pub fn can_enter_transport(
    passenger: &GameEntity,
    transport: &GameEntity,
    passenger_obj: &ObjectType,
    transport_obj: &ObjectType,
    cargo: &PassengerCargo,
    condition_red_x1000: i64, // pre-scaled integer (0.25 → 250) for deterministic comparison
    interner: &StringInterner,
) -> bool {
    // Must be alive, not dying, not already inside something
    if !passenger.is_alive() || passenger.dying {
        return false;
    }
    if passenger.passenger_role.is_inside_transport() {
        return false;
    }
    // Transport must be alive
    if !transport.is_alive() || transport.dying {
        return false;
    }
    // Owner check — garrison buildings allow neutral/special civilian buildings.
    if transport_obj.can_be_occupied {
        let same_owner = passenger.owner == transport.owner;
        let transport_owner_str = interner.resolve(transport.owner);
        let neutral_building = transport_owner_str.eq_ignore_ascii_case("neutral")
            || transport_owner_str.eq_ignore_ascii_case("special");
        if !same_owner && !neutral_building {
            return false;
        }
    } else {
        // Vehicle transports: strict same-owner.
        if passenger.owner != transport.owner {
            return false;
        }
    }
    // Garrison-specific: only Occupier infantry, and building must not be at red health.
    if transport_obj.can_be_occupied {
        if transport.building_up.is_some() || transport.building_down.is_some() {
            return false;
        }
        if !passenger_obj.occupier {
            return false;
        }
        // Pure integer comparison: health_ratio <= condition_red
        // ↔ current * 1000 <= max * condition_red_x1000
        let current_x1000: i64 = transport.health.current as i64 * 1000;
        let threshold_x1000: i64 = transport.health.max.max(1) as i64 * condition_red_x1000;
        if current_x1000 <= threshold_x1000 {
            return false;
        }
    }
    // Size check
    cargo.can_accept(passenger_obj.size)
}

/// Maximum cell distance for a passenger to be considered "at" the transport.
/// Chebyshev distance in cells — 1 means same cell or adjacent.
const BOARD_DISTANCE: u32 = 1;

/// 8-directional neighbor offsets for finding unload exit cells.
const NEIGHBORS: [(i16, i16); 8] = [
    (0, -1),  // N
    (1, -1),  // NE
    (1, 0),   // E
    (1, 1),   // SE
    (0, 1),   // S
    (-1, 1),  // SW
    (-1, 0),  // W
    (-1, -1), // NW
];

/// Advance the passenger boarding/unloading system each tick.
///
/// Phase A: For entities with `boarding_state`, check if they arrived at
/// the transport's cell. If so, execute boarding. If the transport is
/// destroyed or full, cancel boarding.
///
/// Phase B: For transports with `OrderIntent::Unloading`, eject one
/// passenger per tick to an adjacent unoccupied cell. Clear the order
/// when all passengers are out.
/// Returns `true` if any entity's ownership changed this tick (garrison
/// transfer or revert), signalling that the sprite atlas needs a rebuild.
pub fn tick_passenger_system(sim: &mut Simulation, rules: &RuleSet) -> bool {
    let boarding_changed = tick_boarding(sim, rules);
    let unloading_changed = tick_unloading(sim, rules);
    boarding_changed || unloading_changed
}

/// Snapshot-then-mutate: process entities that are trying to board a transport.
/// Returns `true` if any entity ownership changed (garrison transfer).
fn tick_boarding(sim: &mut Simulation, rules: &RuleSet) -> bool {
    let mut ownership_changed = false;
    // Snapshot entities with Boarding role — must collect fully before mutating.
    let keys: Vec<u64> = sim.entities.keys_sorted();
    let boarding_snapshot: Vec<(u64, u64)> = keys
        .iter()
        .filter_map(|&id| {
            let e = sim.entities.get(id)?;
            if let PassengerRole::Boarding {
                target_transport_id,
                ..
            } = &e.passenger_role
            {
                Some((id, *target_transport_id))
            } else {
                Option::None
            }
        })
        .collect();

    for (pax_id, transport_id) in boarding_snapshot {
        // Check transport still exists and is alive.
        let transport_alive = sim
            .entities
            .get(transport_id)
            .is_some_and(|t| t.is_alive() && !t.dying);
        if !transport_alive {
            // Transport gone — cancel boarding.
            if let Some(e) = sim.entities.get_mut(pax_id) {
                e.passenger_role = PassengerRole::None;
            }
            continue;
        }

        // Get positions to check distance.
        let (pax_rx, pax_ry) = match sim.entities.get(pax_id) {
            Some(e) => (e.position.rx, e.position.ry),
            None => continue,
        };
        let (trx, try_) = match sim.entities.get(transport_id) {
            Some(e) => (e.position.rx, e.position.ry),
            None => continue,
        };

        // Chebyshev distance between passenger and transport.
        let dx = (pax_rx as i32 - trx as i32).unsigned_abs();
        let dy = (pax_ry as i32 - try_ as i32).unsigned_abs();
        let dist = dx.max(dy);

        if dist <= BOARD_DISTANCE {
            // Passenger has arrived — attempt boarding.
            let pax_type_str = sim
                .entities
                .get(pax_id)
                .map(|e| sim.interner.resolve(e.type_ref).to_string())
                .unwrap_or_default();
            let transport_type_str = sim
                .entities
                .get(transport_id)
                .map(|e| sim.interner.resolve(e.type_ref).to_string())
                .unwrap_or_default();

            let pax_size = rules.object(&pax_type_str).map(|obj| obj.size).unwrap_or(1);

            let transport_obj = rules.object(&transport_type_str);
            let transport_gunner = transport_obj.map(|obj| obj.gunner).unwrap_or(false);
            let transport_open_topped = transport_obj.map(|obj| obj.open_topped).unwrap_or(false);

            let pax_obj = rules.object(&pax_type_str);
            let pax_ifv_mode = pax_obj.map(|obj| obj.ifv_mode).unwrap_or(0);
            let pax_open_transport_weapon =
                pax_obj.map(|obj| obj.open_transport_weapon).unwrap_or(-1);

            // Try to board.
            let boarded = sim
                .entities
                .get_mut(transport_id)
                .and_then(|t| t.passenger_role.cargo_mut())
                .is_some_and(|cargo| cargo.board(pax_id, pax_size));

            if boarded {
                // Garrison ownership transfer: when infantry boards a neutral/civilian
                // CanBeOccupied building, transfer building ownership to the infantry's
                // owner. Matches original engine's CheckAutoSellOrCivilian reconciliation
                // (we do it immediately rather than waiting one tick).
                let transport_can_be_occupied = rules
                    .object(&transport_type_str)
                    .map(|obj| obj.can_be_occupied)
                    .unwrap_or(false);
                if transport_can_be_occupied {
                    let pax_owner = sim.entities.get(pax_id).map(|e| e.owner);
                    if let Some(new_owner) = pax_owner {
                        if let Some(t) = sim.entities.get(transport_id) {
                            let t_owner_str = sim.interner.resolve(t.owner);
                            let is_neutral = t_owner_str.eq_ignore_ascii_case("neutral")
                                || t_owner_str.eq_ignore_ascii_case("special");
                            if is_neutral {
                                if let Some(t) = sim.entities.get_mut(transport_id) {
                                    // Save original owner before first transfer so we can
                                    // revert when the last occupant exits.
                                    if t.garrison_original_owner.is_none() {
                                        t.garrison_original_owner = Some(t.owner);
                                    }
                                    t.owner = new_owner;
                                    ownership_changed = true;
                                }
                            }
                        }
                    }
                }

                // Garrison sound/EVA: emit on first occupant entry only.
                // gamemd AddGarrisonOccupant fires EVA + BuildingGarrisonedSound
                // when count transitions 0→1; subsequent occupants are silent.
                let first_occupant = sim
                    .entities
                    .get(transport_id)
                    .and_then(|t| t.passenger_role.cargo())
                    .map_or(false, |c| c.count() == 1);
                if first_occupant
                    && rules
                        .object(&transport_type_str)
                        .map_or(false, |o| o.can_be_occupied)
                {
                    if let Some(t) = sim.entities.get(transport_id) {
                        let owner = t.owner;
                        let rx = t.position.rx;
                        let ry = t.position.ry;
                        sim.sound_events
                            .push(SimSoundEvent::StructureGarrisoned { owner });
                        sim.sound_events.push(SimSoundEvent::BuildingGarrisonedSfx {
                            owner,
                            rx,
                            ry,
                        });
                    }
                }

                // Hide the passenger entity.
                sim.clear_radio_contacts_for(pax_id);
                if let Some(pax) = sim.entities.get_mut(pax_id) {
                    pax.passenger_role = PassengerRole::Inside { transport_id };
                    pax.movement_target = None;
                    pax.attack_target = None;
                    pax.order_intent = None;
                }
                // Transport weapon override: Gunner=yes transports swap their
                // own weapon to weapon_list[IFVMode]; open-topped non-Gunner
                // transports fire the passenger's own Primary/Secondary based
                // on the passenger's OpenTransportWeapon slot.
                let new_override = if transport_gunner {
                    Some(crate::sim::combat::combat_weapon::WeaponOverride::IfvSlot(
                        pax_ifv_mode,
                    ))
                } else if transport_open_topped && pax_open_transport_weapon >= 0 {
                    Some(
                        crate::sim::combat::combat_weapon::WeaponOverride::OpenTransport(
                            pax_open_transport_weapon as u32,
                        ),
                    )
                } else {
                    None
                };
                if new_override.is_some() {
                    if let Some(t) = sim.entities.get_mut(transport_id) {
                        t.weapon_override = new_override;
                    }
                }
            } else {
                // Transport full — cancel boarding.
                if let Some(pax) = sim.entities.get_mut(pax_id) {
                    pax.passenger_role = PassengerRole::None;
                }
            }
        }
        // If still approaching (movement_target present), let movement continue.
        // If movement finished but not close enough, the unit just stops.
    }
    ownership_changed
}

/// Process transports with `OrderIntent::Unloading` — eject one passenger per tick.
/// Returns `true` if any entity ownership changed (garrison revert to Neutral).
fn tick_unloading(sim: &mut Simulation, rules: &RuleSet) -> bool {
    let mut ownership_changed = false;
    // Snapshot transports that are unloading — must collect fully before mutating.
    let keys: Vec<u64> = sim.entities.keys_sorted();
    let unload_snapshot: Vec<u64> = keys
        .iter()
        .filter_map(|&id| {
            let e = sim.entities.get(id)?;
            if matches!(e.order_intent, Some(OrderIntent::Unloading)) {
                Some(id)
            } else {
                None
            }
        })
        .collect();

    for transport_id in unload_snapshot {
        let (trx, try_, tz) = match sim.entities.get(transport_id) {
            Some(e) => (e.position.rx, e.position.ry, e.position.z),
            None => continue,
        };

        // Collect occupied cell positions (skip transported/dying entities).
        let occupied_cells: Vec<(u16, u16)> = {
            let all_keys: Vec<u64> = sim.entities.keys_sorted();
            all_keys
                .iter()
                .filter_map(|&eid| {
                    let e = sim.entities.get(eid)?;
                    if !e.passenger_role.is_inside_transport() && !e.dying && e.is_alive() {
                        Some((e.position.rx, e.position.ry))
                    } else {
                        None
                    }
                })
                .collect()
        };

        // Find an adjacent free cell for the passenger to exit to.
        // Simple search: first unoccupied neighbor in 8 directions.
        let exit_cell = NEIGHBORS.iter().find_map(|&(dx, dy)| {
            let nx = trx as i16 + dx;
            let ny = try_ as i16 + dy;
            if nx < 0 || ny < 0 {
                return None;
            }
            let (nx, ny) = (nx as u16, ny as u16);
            let occupied = occupied_cells.iter().any(|&(ox, oy)| ox == nx && oy == ny);
            if occupied { None } else { Some((nx, ny)) }
        });

        let Some((exit_rx, exit_ry)) = exit_cell else {
            // No free cell — skip this tick, try again next tick.
            continue;
        };

        // Pop the first passenger from the cargo.
        let pax_id = sim
            .entities
            .get_mut(transport_id)
            .and_then(|t| t.passenger_role.cargo_mut())
            .and_then(|cargo| cargo.unload_first());

        let Some(pax_id) = pax_id else {
            // Cargo empty — clear unload order.
            if let Some(t) = sim.entities.get_mut(transport_id) {
                t.order_intent = None;
            }
            continue;
        };

        // Get passenger size for total_size bookkeeping.
        let pax_type_str = sim
            .entities
            .get(pax_id)
            .map(|e| sim.interner.resolve(e.type_ref).to_string())
            .unwrap_or_default();
        let pax_size = rules.object(&pax_type_str).map(|obj| obj.size).unwrap_or(1);

        // Adjust total_size on the cargo.
        if let Some(cargo) = sim
            .entities
            .get_mut(transport_id)
            .and_then(|t| t.passenger_role.cargo_mut())
        {
            cargo.total_size = cargo.total_size.saturating_sub(pax_size);
        }

        // Restore the passenger entity to the map.
        let pax_sub_cell;
        if let Some(pax) = sim.entities.get_mut(pax_id) {
            pax.passenger_role = PassengerRole::None;
            pax.position.rx = exit_rx;
            pax.position.ry = exit_ry;
            pax.position.z = tz;
            // Recalculate sub-cell offsets and screen position.
            let (sub_x, sub_y) = lepton::subcell_lepton_offset(pax.sub_cell);
            pax.position.sub_x = sub_x;
            pax.position.sub_y = sub_y;
            pax.position.refresh_screen_coords();
            pax_sub_cell = pax.sub_cell;
        } else {
            pax_sub_cell = None;
        }
        // Register unloaded passenger in occupancy grid.
        sim.occupancy.add(
            exit_rx,
            exit_ry,
            pax_id,
            crate::sim::movement::locomotor::MovementLayer::Ground,
            pax_sub_cell,
            CellListInsertion::PrependNonBuilding,
        );

        // Scatter: issue a short move to a random adjacent cell so ejected
        // infantry flee the building footprint (gamemd mission 0xF / Scatter).
        let scatter_speed = rules
            .object(&pax_type_str)
            .map(|obj| ra2_speed_to_leptons_per_second(obj.speed))
            .unwrap_or(ra2_speed_to_leptons_per_second(4));
        let start_dir = sim.rng.next_u32() as usize % 8;
        for i in 0..8 {
            let (dx, dy) = NEIGHBORS[(start_dir + i) % 8];
            let sx = exit_rx as i32 + dx as i32;
            let sy = exit_ry as i32 + dy as i32;
            if sx >= 0 && sy >= 0 {
                let dest = (sx as u16, sy as u16);
                let occupied = occupied_cells
                    .iter()
                    .any(|&(ox, oy)| ox == dest.0 && oy == dest.1);
                if !occupied {
                    movement::issue_direct_move(&mut sim.entities, pax_id, dest, scatter_speed);
                    break;
                }
            }
        }

        // When the transport is empty, clear any passenger-driven weapon override
        // (covers both Gunner=yes IFV swap and open-topped passenger weapon).
        let transport_type_str = sim
            .entities
            .get(transport_id)
            .map(|e| sim.interner.resolve(e.type_ref).to_string())
            .unwrap_or_default();
        let is_empty = sim
            .entities
            .get(transport_id)
            .and_then(|t| t.passenger_role.cargo())
            .is_some_and(|c| c.is_empty());
        if is_empty {
            if let Some(t) = sim.entities.get_mut(transport_id) {
                t.weapon_override = None;
            }
        }

        // If cargo is now empty, clear the unload order and revert garrison ownership.
        let cargo_empty = sim
            .entities
            .get(transport_id)
            .and_then(|t| t.passenger_role.cargo())
            .is_some_and(|c| c.is_empty());
        if cargo_empty {
            // Garrison ownership revert: when last occupant leaves a CanBeOccupied
            // building, revert ownership to the building's original (pre-garrison)
            // owner. Matches original engine's CheckAutoSellOrCivilian which
            // transfers back to the Civilian house identified by side index.
            let is_garrison_building = rules
                .object(&transport_type_str)
                .map(|obj| obj.can_be_occupied)
                .unwrap_or(false);
            // Pre-intern "Neutral" as fallback for garrison ownership revert.
            let neutral_id = sim.interner.intern("Neutral");
            // Capture pre-revert owner BEFORE the mut borrow — gamemd's
            // CheckAutoSellOrCivilian fires EVA_StructureAbandoned for the
            // player whose garrison just emptied, not the post-revert civilian.
            let abandoning_owner = if is_garrison_building {
                sim.entities.get(transport_id).map(|t| t.owner)
            } else {
                None
            };
            if let Some(t) = sim.entities.get_mut(transport_id) {
                t.order_intent = None;
                if is_garrison_building {
                    let revert_owner = t.garrison_original_owner.take().unwrap_or(neutral_id);
                    t.owner = revert_owner;
                    ownership_changed = true;
                }
            }
            if let Some(owner) = abandoning_owner {
                sim.sound_events
                    .push(SimSoundEvent::StructureAbandoned { owner });
            }
        }
    }
    ownership_changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;

    fn garrison_test_rules() -> RuleSet {
        let ini_str = "\
[InfantryTypes]
0=E1
[VehicleTypes]
[AircraftTypes]
[BuildingTypes]
0=CAGAS01

[E1]
Name=Conscript
Cost=100
Strength=125
Armor=none
Speed=4
Occupier=yes

[CAGAS01]
Name=GasStation
Cost=0
Strength=400
Armor=wood
Foundation=2x2
CanBeOccupied=yes
CanOccupyFire=yes
MaxNumberOccupants=5

[General]
[AudioVisual]
BuildingGarrisonedSound=BuildingGarrisoned
ConditionRed=25%
ConditionYellow=50%
";
        let ini = IniFile::from_str(ini_str);
        RuleSet::from_ini(&ini).expect("parse garrison test rules")
    }

    /// Spawn a CanBeOccupied building entity at (rx, ry) owned by `owner_str`.
    fn spawn_garrison_building(
        sim: &mut Simulation,
        rules: &RuleSet,
        type_ref: &str,
        owner_str: &str,
        rx: u16,
        ry: u16,
    ) -> u64 {
        let stable_id = sim.allocate_stable_id();
        let owner_id = sim.interner.intern(owner_str);
        let type_id = sim.interner.intern(type_ref);
        let mut ge = GameEntity::test_default(stable_id, type_ref, owner_str, rx, ry);
        ge.owner = owner_id;
        ge.type_ref = type_id;
        let obj = rules.object(type_ref).expect("type exists");
        ge.passenger_role = PassengerRole::Transport {
            cargo: PassengerCargo::new(obj.max_number_occupants, 1),
        };
        sim.entities.insert(ge);
        stable_id
    }

    /// Spawn an Occupier infantry entity at (rx, ry) in `Boarding::Entering` state
    /// targeting `transport_id`.
    fn spawn_boarding_occupier(
        sim: &mut Simulation,
        type_ref: &str,
        owner_str: &str,
        transport_id: u64,
        rx: u16,
        ry: u16,
    ) -> u64 {
        let stable_id = sim.allocate_stable_id();
        let owner_id = sim.interner.intern(owner_str);
        let type_id = sim.interner.intern(type_ref);
        let mut ge = GameEntity::test_default(stable_id, type_ref, owner_str, rx, ry);
        ge.owner = owner_id;
        ge.type_ref = type_id;
        ge.passenger_role = PassengerRole::Boarding {
            target_transport_id: transport_id,
            phase: BoardingPhase::Entering,
        };
        sim.entities.insert(ge);
        stable_id
    }

    fn can_enter_garrison_fixture(
        sim: &Simulation,
        rules: &RuleSet,
        passenger_id: u64,
        building_id: u64,
    ) -> bool {
        let passenger = sim.entities.get(passenger_id).expect("passenger exists");
        let transport = sim.entities.get(building_id).expect("building exists");
        let passenger_obj = rules
            .object(sim.interner.resolve(passenger.type_ref))
            .expect("passenger type exists");
        let transport_obj = rules
            .object(sim.interner.resolve(transport.type_ref))
            .expect("transport type exists");
        let cargo = transport.passenger_role.cargo().expect("cargo exists");
        can_enter_transport(
            passenger,
            transport,
            passenger_obj,
            transport_obj,
            cargo,
            rules.general.condition_red_x1000,
            &sim.interner,
        )
    }

    #[test]
    fn test_cargo_new() {
        let cargo = PassengerCargo::new(5, 2);
        assert_eq!(cargo.capacity, 5);
        assert_eq!(cargo.size_limit, 2);
        assert_eq!(cargo.count(), 0);
        assert!(cargo.is_empty());
        assert_eq!(cargo.total_size, 0);
    }

    #[test]
    fn test_board_and_count() {
        let mut cargo = PassengerCargo::new(3, 0);
        assert!(cargo.board(100, 1));
        assert!(cargo.board(101, 1));
        assert!(cargo.board(102, 1));
        assert_eq!(cargo.count(), 3);
        assert!(!cargo.is_empty());
        assert_eq!(cargo.total_size, 3);
        // Full — cannot board more
        assert!(!cargo.board(103, 1));
        assert_eq!(cargo.count(), 3);
    }

    #[test]
    fn test_size_limit_rejection() {
        let mut cargo = PassengerCargo::new(5, 2);
        // Size 1 fits
        assert!(cargo.can_accept(1));
        assert!(cargo.board(100, 1));
        // Size 2 fits
        assert!(cargo.can_accept(2));
        assert!(cargo.board(101, 2));
        // Size 3 rejected by SizeLimit=2
        assert!(!cargo.can_accept(3));
        assert!(!cargo.board(102, 3));
        assert_eq!(cargo.count(), 2);
        assert_eq!(cargo.total_size, 3);
    }

    #[test]
    fn test_size_limit_zero_means_no_restriction() {
        let mut cargo = PassengerCargo::new(5, 0);
        assert!(cargo.can_accept(100)); // Any size fits
        assert!(cargo.board(1, 50));
        assert_eq!(cargo.total_size, 50);
    }

    #[test]
    fn test_disembark() {
        let mut cargo = PassengerCargo::new(5, 0);
        cargo.board(100, 1);
        cargo.board(101, 2);
        cargo.board(102, 1);

        assert!(cargo.disembark(101, 2));
        assert_eq!(cargo.count(), 2);
        assert_eq!(cargo.total_size, 2);
        assert_eq!(cargo.passengers, vec![100, 102]);

        // Disembarking non-existent ID returns false
        assert!(!cargo.disembark(999, 1));
    }

    #[test]
    fn test_unload_first_fifo() {
        let mut cargo = PassengerCargo::new(5, 0);
        cargo.board(100, 1);
        cargo.board(101, 1);
        cargo.board(102, 1);

        assert_eq!(cargo.unload_first(), Some(100));
        assert_eq!(cargo.unload_first(), Some(101));
        assert_eq!(cargo.unload_first(), Some(102));
        assert_eq!(cargo.unload_first(), None);
        assert!(cargo.is_empty());
    }

    #[test]
    fn test_can_accept_when_full() {
        let mut cargo = PassengerCargo::new(1, 1);
        assert!(cargo.can_accept(1));
        cargo.board(100, 1);
        assert!(!cargo.can_accept(1));
    }

    #[test]
    fn test_can_enter_garrison_rejects_red_health_building() {
        let mut sim = Simulation::new();
        let rules = garrison_test_rules();
        let bldg = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Americans", 10, 10);
        let pax = spawn_boarding_occupier(&mut sim, "E1", "Americans", bldg, 10, 11);

        {
            let building = sim.entities.get_mut(bldg).expect("building exists");
            building.health.max = 400;
            building.health.current = 100;
        }

        assert!(
            !can_enter_garrison_fixture(&sim, &rules, pax, bldg),
            "CanDock rejects garrison entry at ConditionRed or below"
        );
    }

    #[test]
    fn test_can_enter_garrison_rejects_non_occupier() {
        let mut sim = Simulation::new();
        let rules = garrison_test_rules();
        let bldg = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Americans", 10, 10);
        let pax = spawn_boarding_occupier(&mut sim, "E1", "Americans", bldg, 10, 11);
        let passenger = sim.entities.get(pax).expect("passenger exists");
        let transport = sim.entities.get(bldg).expect("building exists");
        let mut passenger_obj = rules.object("E1").expect("E1 exists").clone();
        let transport_obj = rules.object("CAGAS01").expect("CAGAS01 exists");
        let cargo = transport.passenger_role.cargo().expect("cargo exists");

        passenger_obj.occupier = false;

        assert!(
            !can_enter_transport(
                passenger,
                transport,
                &passenger_obj,
                transport_obj,
                cargo,
                rules.general.condition_red_x1000,
                &sim.interner,
            ),
            "CanDock requires Occupier=yes infantry"
        );
    }

    #[test]
    fn test_can_enter_garrison_rejects_full_building_at_capacity() {
        let mut sim = Simulation::new();
        let rules = garrison_test_rules();
        let bldg = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Americans", 10, 10);
        let pax = spawn_boarding_occupier(&mut sim, "E1", "Americans", bldg, 10, 11);

        {
            let building = sim.entities.get_mut(bldg).expect("building exists");
            let cargo = building.passenger_role.cargo_mut().expect("cargo exists");
            for occupant_id in 1000..1005 {
                assert!(cargo.board(occupant_id, 1));
            }
            assert_eq!(cargo.count(), cargo.capacity);
        }

        assert!(
            !can_enter_garrison_fixture(&sim, &rules, pax, bldg),
            "CanDock rejects exactly full garrisons"
        );
    }

    #[test]
    fn test_can_enter_garrison_rejects_non_friendly_non_civilian_building() {
        let mut sim = Simulation::new();
        let rules = garrison_test_rules();
        let bldg = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Russians", 10, 10);
        let pax = spawn_boarding_occupier(&mut sim, "E1", "Americans", bldg, 10, 11);

        assert!(
            !can_enter_garrison_fixture(&sim, &rules, pax, bldg),
            "CanDock rejects non-friendly occupied-owner buildings"
        );
    }

    #[test]
    fn test_can_enter_garrison_allows_neutral_and_special_buildings() {
        for owner in ["Neutral", "Special"] {
            let mut sim = Simulation::new();
            let rules = garrison_test_rules();
            let bldg = spawn_garrison_building(&mut sim, &rules, "CAGAS01", owner, 10, 10);
            let pax = spawn_boarding_occupier(&mut sim, "E1", "Americans", bldg, 10, 11);

            assert!(
                can_enter_garrison_fixture(&sim, &rules, pax, bldg),
                "CanDock allows {owner} civilian garrison buildings"
            );
        }
    }

    #[test]
    fn test_can_enter_garrison_rejects_building_up_or_down() {
        let rules = garrison_test_rules();

        {
            let mut sim = Simulation::new();
            let bldg = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Americans", 10, 10);
            let pax = spawn_boarding_occupier(&mut sim, "E1", "Americans", bldg, 10, 11);
            let building = sim.entities.get_mut(bldg).expect("building exists");
            building.building_up = Some(crate::sim::components::BuildingUp {
                elapsed_ticks: 0,
                total_ticks: 30,
            });

            assert!(
                !can_enter_garrison_fixture(&sim, &rules, pax, bldg),
                "CanDock rejects buildings still playing build-up"
            );
        }

        {
            let mut sim = Simulation::new();
            let bldg = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Americans", 10, 10);
            let pax = spawn_boarding_occupier(&mut sim, "E1", "Americans", bldg, 10, 11);
            let owner = sim.entities.get(bldg).expect("building exists").owner;
            let spawn_type = sim.interner.intern("AMCV");
            let building = sim.entities.get_mut(bldg).expect("building exists");
            building.building_down = Some(crate::sim::components::BuildingDown {
                elapsed_ticks: 0,
                total_ticks: 30,
                spawn_type,
                spawn_owner: owner,
                spawn_rx: 10,
                spawn_ry: 10,
                spawn_z: 0,
                was_selected: false,
            });

            assert!(
                !can_enter_garrison_fixture(&sim, &rules, pax, bldg),
                "CanDock rejects buildings in reverse build-down"
            );
        }
    }

    #[test]
    fn test_first_occupant_emits_garrisoned_event() {
        let mut sim = Simulation::new();
        let rules = garrison_test_rules();
        let bldg = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Americans", 10, 10);
        let _pax = spawn_boarding_occupier(&mut sim, "E1", "Americans", bldg, 10, 11);

        tick_boarding(&mut sim, &rules);

        let mut found_eva = false;
        let mut found_sfx = false;
        for evt in &sim.sound_events {
            match evt {
                SimSoundEvent::StructureGarrisoned { owner } => {
                    assert_eq!(
                        sim.interner.resolve(*owner),
                        "Americans",
                        "EVA owner should be the garrisoning player"
                    );
                    found_eva = true;
                }
                SimSoundEvent::BuildingGarrisonedSfx { owner, rx, ry } => {
                    assert_eq!(sim.interner.resolve(*owner), "Americans");
                    assert_eq!((*rx, *ry), (10, 10));
                    found_sfx = true;
                }
                _ => {}
            }
        }
        assert!(found_eva, "expected StructureGarrisoned event");
        assert!(found_sfx, "expected BuildingGarrisonedSfx event");
    }

    #[test]
    fn test_boarding_inside_transition_clears_live_radio_contacts() {
        let mut sim = Simulation::new();
        let rules = garrison_test_rules();
        let bldg = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Americans", 10, 10);
        let pax = spawn_boarding_occupier(&mut sim, "E1", "Americans", bldg, 10, 11);

        sim.entities
            .get_mut(pax)
            .unwrap()
            .mark_live_contact_with(bldg);
        sim.entities
            .get_mut(bldg)
            .unwrap()
            .mark_live_contact_with(pax);

        tick_boarding(&mut sim, &rules);

        assert!(matches!(
            sim.entities.get(pax).unwrap().passenger_role,
            PassengerRole::Inside { transport_id } if transport_id == bldg
        ));
        assert_eq!(
            sim.entities.get(pax).unwrap().radio_contacts,
            Vec::<u64>::new()
        );
        assert!(
            !sim.entities.get(bldg).unwrap().has_live_contact_with(pax),
            "boarding hide should clear peer radio contacts to the passenger"
        );
    }

    #[test]
    fn test_second_occupant_emits_no_garrison_event() {
        let mut sim = Simulation::new();
        let rules = garrison_test_rules();
        let bldg = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Americans", 10, 10);

        // Pre-populate with one occupant (simulating a previous successful board).
        if let Some(t) = sim.entities.get_mut(bldg) {
            if let Some(cargo) = t.passenger_role.cargo_mut() {
                cargo.board(9999, 1);
            }
        }
        let _pax = spawn_boarding_occupier(&mut sim, "E1", "Americans", bldg, 10, 11);

        tick_boarding(&mut sim, &rules);

        for evt in &sim.sound_events {
            match evt {
                SimSoundEvent::StructureGarrisoned { .. }
                | SimSoundEvent::BuildingGarrisonedSfx { .. } => {
                    panic!(
                        "garrison event should NOT emit on non-first occupant: {:?}",
                        evt
                    );
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_last_occupant_emits_abandoned_event_with_pre_revert_owner() {
        let mut sim = Simulation::new();
        let rules = garrison_test_rules();
        // Spawn a CanBeOccupied building owned by Americans (post-garrison state),
        // with garrison_original_owner = Neutral (pre-garrison state).
        let bldg = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Americans", 10, 10);
        let neutral_id = sim.interner.intern("Neutral");
        // Set up the "1 occupant inside, original owner = Neutral" state.
        if let Some(t) = sim.entities.get_mut(bldg) {
            t.garrison_original_owner = Some(neutral_id);
            if let Some(cargo) = t.passenger_role.cargo_mut() {
                // Pretend a passenger entity 12345 was inside.
                cargo.board(12345, 1);
            }
            t.order_intent = Some(OrderIntent::Unloading);
        }
        // Spawn a placeholder passenger entity so unload_first finds it.
        let pax_owner = sim.interner.intern("Americans");
        let pax_type = sim.interner.intern("E1");
        let mut pax = GameEntity::test_default(12345, "E1", "Americans", 9, 10);
        pax.owner = pax_owner;
        pax.type_ref = pax_type;
        pax.passenger_role = PassengerRole::Inside { transport_id: bldg };
        sim.entities.insert(pax);

        // Tick unloading — should pop the one passenger and trigger empty branch.
        tick_unloading(&mut sim, &rules);

        // Assert StructureAbandoned was emitted with the PRE-revert owner (Americans).
        let mut found = false;
        for evt in &sim.sound_events {
            if let SimSoundEvent::StructureAbandoned { owner } = evt {
                assert_eq!(
                    sim.interner.resolve(*owner),
                    "Americans",
                    "StructureAbandoned should carry pre-revert owner, not post-revert civilian"
                );
                found = true;
            }
        }
        assert!(
            found,
            "expected StructureAbandoned event after last occupant left"
        );

        // Confirm the revert actually happened (post-revert owner = Neutral).
        let bldg_owner_str = sim
            .entities
            .get(bldg)
            .map(|t| sim.interner.resolve(t.owner).to_string())
            .expect("building exists");
        assert_eq!(
            bldg_owner_str, "Neutral",
            "owner should have reverted to Neutral"
        );
    }

    #[test]
    fn test_non_garrison_transport_emits_no_garrison_events() {
        // Passengers=5 IFV-style transport (not CanBeOccupied) — no garrison events.
        let ini_str = "\
[InfantryTypes]
0=E1
[VehicleTypes]
0=IFV
[AircraftTypes]
[BuildingTypes]

[E1]
Name=Conscript
Cost=100
Strength=125
Armor=none
Speed=4
Occupier=yes

[IFV]
Name=IFV
Cost=600
Strength=200
Armor=light
Speed=8
Passengers=5

[General]
[AudioVisual]
ConditionRed=25%
ConditionYellow=50%
";
        let ini = IniFile::from_str(ini_str);
        let rules = RuleSet::from_ini(&ini).expect("parse");
        let mut sim = Simulation::new();
        let bldg_id = sim.allocate_stable_id();
        let owner_id = sim.interner.intern("Americans");
        let type_id = sim.interner.intern("IFV");
        let mut bldg = GameEntity::test_default(bldg_id, "IFV", "Americans", 10, 10);
        bldg.owner = owner_id;
        bldg.type_ref = type_id;
        bldg.passenger_role = PassengerRole::Transport {
            cargo: PassengerCargo::new(5, 0),
        };
        sim.entities.insert(bldg);
        let _pax = spawn_boarding_occupier(&mut sim, "E1", "Americans", bldg_id, 10, 11);

        tick_boarding(&mut sim, &rules);

        for evt in &sim.sound_events {
            match evt {
                SimSoundEvent::StructureGarrisoned { .. }
                | SimSoundEvent::BuildingGarrisonedSfx { .. } => {
                    panic!(
                        "non-garrison transport should not emit garrison events: {:?}",
                        evt
                    );
                }
                _ => {}
            }
        }
    }

    /// Helper: insert an Occupier infantry directly into a garrison building's
    /// cargo (skipping the boarding flow). Used by destruction-eject tests.
    fn place_inside_garrison(
        sim: &mut Simulation,
        rules: &RuleSet,
        building_id: u64,
        type_ref: &str,
        owner_str: &str,
    ) -> u64 {
        let stable_id = sim.allocate_stable_id();
        let owner_id = sim.interner.intern(owner_str);
        let type_id = sim.interner.intern(type_ref);
        let mut ge = GameEntity::test_default(stable_id, type_ref, owner_str, 0, 0);
        ge.owner = owner_id;
        ge.type_ref = type_id;
        ge.passenger_role = PassengerRole::Inside {
            transport_id: building_id,
        };
        sim.entities.insert(ge);
        // Add to building's cargo.
        if let Some(bldg) = sim.entities.get_mut(building_id) {
            if let Some(cargo) = bldg.passenger_role.cargo_mut() {
                let obj = rules.object(type_ref).expect("type exists");
                cargo.board(stable_id, obj.size.max(1));
            }
        }
        // Building inherits garrisoning player's ownership (sim does this on
        // first board). For destruction tests we set it explicitly here, and
        // also set category=Structure since GameEntity::test_default leaves it
        // as Unit — the death-loop branch keys on Structure.
        if let Some(bldg) = sim.entities.get_mut(building_id) {
            if bldg.garrison_original_owner.is_none() {
                bldg.garrison_original_owner = Some(bldg.owner);
            }
            bldg.owner = owner_id;
            bldg.category = crate::map::entities::EntityCategory::Structure;
        }
        stable_id
    }

    /// Construct a `DestroyedGarrisonBuilding` event from a still-alive
    /// building's state, then despawn the building (mirroring the combat
    /// death-loop side effects) and call the eject helper. This tests the
    /// helper end-to-end without needing a full combat tick + damage events.
    fn eject_via_event(sim: &mut Simulation, rules: &RuleSet, building_id: u64) -> Vec<u64> {
        let event = {
            let bldg = sim.entities.get(building_id).expect("building present");
            let cargo = bldg.passenger_role.cargo().expect("cargo present");
            let obj = rules
                .object(sim.interner.resolve(bldg.type_ref))
                .expect("type exists");
            let (foundation_w, foundation_h) =
                crate::sim::production::foundation_dimensions(&obj.foundation);
            crate::sim::combat::DestroyedGarrisonBuilding {
                building_id,
                type_id: bldg.type_ref,
                owner: bldg.owner,
                rx: bldg.position.rx,
                ry: bldg.position.ry,
                z: bldg.position.z,
                foundation_w,
                foundation_h,
                passenger_ids: cargo.passengers.clone(),
            }
        };
        let survivor_ids = event.passenger_ids.clone();
        sim.entities.remove(building_id);
        crate::sim::production::eject_destruction_garrison(sim, rules, &event);
        survivor_ids
    }

    #[test]
    fn test_garrison_eject_on_destruction_happy_path() {
        let rules = garrison_test_rules();
        let mut sim = Simulation::new();
        let building_id = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Allied", 10, 10);
        let pax1 = place_inside_garrison(&mut sim, &rules, building_id, "E1", "Allied");
        let pax2 = place_inside_garrison(&mut sim, &rules, building_id, "E1", "Allied");
        let pax3 = place_inside_garrison(&mut sim, &rules, building_id, "E1", "Allied");

        let survivor_ids = eject_via_event(&mut sim, &rules, building_id);
        assert_eq!(survivor_ids.len(), 3, "all 3 occupants captured");

        // Building gone.
        assert!(
            sim.entities.get(building_id).is_none(),
            "building despawned"
        );

        let expected_positions = [(12, 10), (11, 12), (12, 11)];
        for (pid, expected_position) in [
            (pax1, expected_positions[0]),
            (pax2, expected_positions[1]),
            (pax3, expected_positions[2]),
        ] {
            let pax = sim.entities.get(pid).expect("survivor present");
            assert!(pax.is_alive(), "occupant {pid} should be alive");
            assert!(!pax.dying, "occupant {pid} should not be dying");
            assert!(matches!(pax.passenger_role, PassengerRole::None));
            assert_eq!(
                sim.interner.resolve(pax.owner),
                "Allied",
                "occupant {pid} should retain garrisoning owner"
            );
            assert_eq!(
                (pax.position.rx, pax.position.ry),
                expected_position,
                "occupant {pid} should use sell-style foundation-edge ejection"
            );
        }

        // Each occupant should be at a unique edge cell (LIFO + used_cells dedup).
        let mut positions: Vec<(u16, u16)> = [pax1, pax2, pax3]
            .iter()
            .map(|&p| {
                let pe = sim.entities.get(p).unwrap();
                (pe.position.rx, pe.position.ry)
            })
            .collect();
        positions.sort();
        positions.dedup();
        assert_eq!(positions.len(), 3, "all occupants on distinct cells");
    }

    #[test]
    fn test_garrison_eject_blocked_edge_cells_kills_occupants() {
        let rules = garrison_test_rules();
        let mut sim = Simulation::new();
        let building_id = spawn_garrison_building(&mut sim, &rules, "CAGAS01", "Allied", 10, 10);
        let pax = place_inside_garrison(&mut sim, &rules, building_id, "E1", "Allied");

        // Block all sell-style edge ejection cells around the 2x2 building.
        let owner_id = sim.interner.intern("Allied");
        for (bx, by) in [
            (12, 11),
            (11, 12),
            (12, 10),
            (10, 12),
            (12, 12),
            (11, 9),
            (9, 11),
            (10, 9),
            (12, 9),
            (9, 10),
            (9, 12),
            (9, 9),
        ] {
            let blocker_id = sim.allocate_stable_id();
            let mut blocker = GameEntity::test_default(blocker_id, "E1", "Allied", bx, by);
            blocker.owner = owner_id;
            blocker.type_ref = sim.interner.intern("E1");
            sim.entities.insert(blocker);
        }

        eject_via_event(&mut sim, &rules, building_id);

        let pax_entity = sim.entities.get(pax).expect("entity present");
        assert!(pax_entity.dying, "occupant should be marked dying");
        assert_eq!(pax_entity.health.current, 0);
        assert!(matches!(pax_entity.passenger_role, PassengerRole::None));
    }
}
