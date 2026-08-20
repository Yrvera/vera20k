//! Active Techno sensor-count deposit lifecycle.
//!
//! FootClass add/remove/move/owner writers are at 0x004D7318, 0x004DB376,
//! 0x004D8611/0x004D8621, and 0x004DBEFB/0x004DBF81. Building sensor arrays
//! use 0x00455820/0x004556D0 and deliberately add with `SensorsSight=` but
//! remove with the signed-byte `CloakRadiusInCells=` value.

use crate::rules::object_type::{ObjectCategory, ObjectType};
use crate::rules::ruleset::RuleSet;
use crate::map::entities::EntityCategory;
use crate::sim::intern::InternedId;
use crate::sim::world::Simulation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SensorDeposit {
    pub owner: InternedId,
    pub center: (u16, u16),
    pub add_radius: u16,
    pub remove_radius: i16,
    pub building_array: bool,
}

impl SensorDeposit {
    fn unit(owner: InternedId, center: (u16, u16), radius: u16) -> Self {
        Self {
            owner,
            center,
            add_radius: radius,
            remove_radius: radius as i16,
            building_array: false,
        }
    }

    fn building(owner: InternedId, center: (u16, u16), add: u16, remove: i8) -> Self {
        Self {
            owner,
            center,
            add_radius: add,
            remove_radius: i16::from(remove),
            building_array: true,
        }
    }
}

fn unit_sensor_radius(entity_category: EntityCategory, object: &ObjectType) -> Option<u16> {
    if object.sensors_sight == 0
        || !matches!(
            entity_category,
            EntityCategory::Unit | EntityCategory::Infantry | EntityCategory::Aircraft
        )
    {
        return None;
    }
    Some(u16::from(object.sensors_sight))
}

impl Simulation {
    fn remove_cached_sensor_deposit(&mut self, stable_id: u64) -> Option<SensorDeposit> {
        let deposit = self
            .substrate
            .entities
            .get_mut(stable_id)
            .and_then(|entity| entity.sensor_deposit.take())?;
        self.fog.sensors_remove_at(
            deposit.owner,
            deposit.center,
            deposit.remove_radius.max(0) as u16,
        );
        Some(deposit)
    }

    /// `FootClass::Unlimbo` sensor writer. The deposit is absent for a
    /// rules-less/headless fixture and for non-Foot objects.
    pub(crate) fn add_unit_sensor_after_unlimbo(&mut self, stable_id: u64, rules: &RuleSet) {
        let Some((owner, center, radius, in_limbo)) = self
            .substrate
            .entities
            .get(stable_id)
            .and_then(|entity| {
                let object = rules.object(self.interner.resolve(entity.type_ref))?;
                Some((
                    entity.owner,
                    (entity.position.rx, entity.position.ry),
                    unit_sensor_radius(entity.category, object)?,
                    entity.lifecycle.in_limbo,
                ))
            })
        else {
            return;
        };
        if in_limbo {
            return;
        }
        let _ = self.remove_cached_sensor_deposit(stable_id);
        self.fog.sensors_add_at(owner, center, radius);
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.sensor_deposit = Some(SensorDeposit::unit(owner, center, radius));
        }
    }

    /// `BuildingClass::AddSensorArrayAt @ 0x00455820`, used only after map
    /// initialization or construction completion and only while powered.
    pub(crate) fn add_building_sensor_array_if_powered(
        &mut self,
        stable_id: u64,
        rules: &RuleSet,
    ) {
        let Some((owner, center, sight, remove_radius, powered, in_limbo)) = self
            .substrate
            .entities
            .get(stable_id)
            .and_then(|entity| {
                let object = rules.object(self.interner.resolve(entity.type_ref))?;
                if entity.category != EntityCategory::Structure
                    || object.category != ObjectCategory::Building
                    || !object.sensor_array
                    || object.sensors_sight == 0
                {
                    return None;
                }
                Some((
                    entity.owner,
                    (entity.position.rx, entity.position.ry),
                    u16::from(object.sensors_sight),
                    object.cloak_radius_in_cells,
                    crate::sim::power_system::is_building_powered(
                        &self.power_states,
                        rules,
                        entity,
                        &self.interner,
                    ),
                    entity.lifecycle.in_limbo,
                ))
            })
        else {
            return;
        };
        if !powered || in_limbo {
            return;
        }
        let _ = self.remove_cached_sensor_deposit(stable_id);
        self.fog.sensors_add_at(owner, center, sight);
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.sensor_deposit = Some(SensorDeposit::building(
                owner,
                center,
                sight,
                remove_radius,
            ));
        }
    }

    /// Exact cached remove used by Foot/Building Limbo.
    pub(crate) fn remove_sensor_before_limbo(&mut self, stable_id: u64) {
        let _ = self.remove_cached_sensor_deposit(stable_id);
    }

    /// FootClass::PerCellProcess old-remove/new-add pair. TubeMovement owns an
    /// early-return turn and is intentionally not routed here until its native
    /// completion writer lands.
    pub(crate) fn move_unit_sensor_after_cell_change(
        &mut self,
        stable_id: u64,
        old_cell: Option<(u16, u16)>,
        new_cell: Option<(u16, u16)>,
        rules: &RuleSet,
    ) {
        if old_cell == new_cell {
            return;
        }
        let had_unit_deposit = self
            .substrate
            .entities
            .get(stable_id)
            .and_then(|entity| entity.sensor_deposit)
            .is_some_and(|deposit| !deposit.building_array);
        if had_unit_deposit {
            let _ = self.remove_cached_sensor_deposit(stable_id);
        }
        self.add_unit_sensor_after_unlimbo(stable_id, rules);
    }

    /// FootClass::ChangeOwner old-remove/new-add pair.
    pub(crate) fn transfer_sensor_before_owner_change(
        &mut self,
        stable_id: u64,
        new_owner: InternedId,
    ) {
        let Some(mut deposit) = self.remove_cached_sensor_deposit(stable_id) else {
            return;
        };
        deposit.owner = new_owner;
        self.fog
            .sensors_add_at(new_owner, deposit.center, deposit.add_radius);
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.sensor_deposit = Some(deposit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::entities::{EntityCategory, MapEntity};
    use crate::map::playfield::PlayfieldBounds;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::components::BuildingUp;

    fn rules() -> RuleSet {
        RuleSet::from_ini(&IniFile::from_str(
            "[General]\nCloakingStages=9\nCloakDelay=.02\n\
             [VehicleTypes]\n0=DEST\n1=SUB\n2=SQD\n\
             [BuildingTypes]\n0=NAPSIS\n1=NAPOWR\n\
             [DEST]\nStrength=600\nSpeed=6\nSensorsSight=8\n\
             [SUB]\nStrength=600\nSpeed=4\nCloakable=yes\nCloakingSpeed=1\nSensorsSight=7\n\
             [SQD]\nStrength=600\nSpeed=4\nCloakable=yes\nCloakingSpeed=5\nSensorsSight=8\n\
             [NAPSIS]\nStrength=750\nSensorArray=yes\nSensorsSight=15\nPower=-100\nPowered=yes\n\
             [NAPOWR]\nStrength=750\nPower=200\n",
        ))
        .expect("cloak/sensor rules")
    }

    fn sim_with_map_authority() -> Simulation {
        let mut sim = Simulation::with_seed(0x5E45_0A00);
        sim.fog.width = 64;
        sim.fog.height = 64;
        sim.playfield_bounds = Some(PlayfieldBounds::from_normalized_local_size(
            64, 2, 2, 56, 52,
        ));
        sim
    }

    #[test]
    fn unit_sensor_unlimbo_move_owner_limbo_and_overlap_use_cached_deposits() {
        let rules = rules();
        let mut sim = sim_with_map_authority();
        let first = sim
            .spawn_object_at_height("DEST", "Americans", 20, 20, 0, 0, &rules)
            .unwrap();
        let second = sim
            .spawn_object_at_height("DEST", "Americans", 20, 20, 0, 0, &rules)
            .unwrap();
        let americans = sim.substrate.entities.get(first).unwrap().owner;
        let soviet = sim.interner.intern("Soviet");
        assert!(sim.fog.has_sensor_for_house(americans, 13, 20));
        assert!(!sim.fog.has_sensor_for_house(americans, 28, 20));

        sim.techno_limbo(first);
        assert!(
            sim.fog.has_sensor_for_house(americans, 13, 20),
            "the overlapping second deposit remains positive"
        );

        let old = Some((20, 20));
        sim.substrate.entities.get_mut(second).unwrap().position.rx = 30;
        sim.move_unit_sensor_after_cell_change(second, old, Some((30, 20)), &rules);
        assert!(!sim.fog.has_sensor_for_house(americans, 13, 20));
        assert!(sim.fog.has_sensor_for_house(americans, 37, 20));

        sim.change_owner(second, soviet);
        assert!(!sim.fog.has_sensor_for_house(americans, 37, 20));
        assert!(sim.fog.has_sensor_for_house(soviet, 37, 20));

        // Drift every live fact after the deposit. Limbo must still remove the
        // cached Soviet/30,20/radius8 footprint, not current owner/position.
        let neutral = sim.interner.intern("Neutral");
        {
            let entity = sim.substrate.entities.get_mut(second).unwrap();
            entity.owner = neutral;
            entity.position.rx = 5;
            entity.position.ry = 5;
        }
        sim.techno_limbo(second);
        assert!(!sim.fog.has_sensor_for_house(soviet, 37, 20));
    }

    #[test]
    fn napsis_construction_adds_radius_fifteen_but_limbo_removes_default_twenty() {
        let rules = rules();
        let mut sim = sim_with_map_authority();
        sim.spawn_object_at_height("NAPOWR", "Soviet", 20, 20, 0, 0, &rules)
            .unwrap();
        let id = sim
            .spawn_object_at_height("NAPSIS", "Soviet", 30, 30, 0, 0, &rules)
            .unwrap();
        let owner = sim.substrate.entities.get(id).unwrap().owner;
        assert!(sim.substrate.entities.get(id).unwrap().sensor_deposit.is_none());
        sim.substrate.entities.get_mut(id).unwrap().building_up = Some(BuildingUp {
            elapsed_ticks: 0,
            total_ticks: 1,
        });
        sim.advance_tick(
            &[],
            Some(&rules),
            &std::collections::BTreeMap::new(),
            None,
            None,
            67,
        );
        assert!(sim.substrate.entities.get(id).unwrap().building_up.is_none());
        assert!(sim.fog.has_sensor_for_house(owner, 44, 30));
        assert!(!sim.fog.has_sensor_for_house(owner, 45, 30));
        sim.techno_limbo(id);
        let index = 30 * usize::from(sim.fog.width) + 45;
        assert_eq!(sim.fog.sensors_by_house[&owner][index], -1);
    }

    #[test]
    fn map_and_production_unlimbo_apply_exact_unit_cloak_state_two_rule() {
        let rules = rules();
        let mut sim = sim_with_map_authority();
        let bounds = sim.playfield_bounds.unwrap();
        let inside = (0u16..64)
            .flat_map(|ry| (0u16..64).map(move |rx| (rx, ry)))
            .find(|&(rx, ry)| bounds.contains_height_aware_packed(rx.into(), ry.into(), 0, 0))
            .expect("mode-one inside cell");
        let outside = (0u16..64)
            .flat_map(|ry| (0u16..64).map(move |rx| (rx, ry)))
            .find(|&(rx, ry)| !bounds.contains_height_aware_packed(rx.into(), ry.into(), 0, 0))
            .expect("mode-one outside cell");
        let height = std::collections::BTreeMap::new();
        sim.spawn_from_map(
            &[
                MapEntity {
                    owner: "Soviet".into(),
                    type_id: "SUB".into(),
                    health: 256,
                    cell_x: inside.0,
                    cell_y: inside.1,
                    facing: 0,
                    category: EntityCategory::Unit,
                    sub_cell: 0,
                    veterancy: 0,
                    high: false,
                    mission: None,
                },
                MapEntity {
                    owner: "Soviet".into(),
                    type_id: "SUB".into(),
                    health: 256,
                    cell_x: outside.0,
                    cell_y: outside.1,
                    facing: 0,
                    category: EntityCategory::Unit,
                    sub_cell: 0,
                    veterancy: 0,
                    high: false,
                    mission: None,
                },
            ],
            Some(&rules),
            &height,
        );
        assert_eq!(sim.substrate.entities.get(1).unwrap().cloak.as_ref().unwrap().state, 0);
        assert_eq!(sim.substrate.entities.get(2).unwrap().cloak.as_ref().unwrap().state, 2);

        let inside = sim
            .spawn_object_at_height("SUB", "Soviet", inside.0, inside.1, 0, 0, &rules)
            .unwrap();
        let outside = sim
            .spawn_object_at_height("SUB", "Soviet", outside.0, outside.1, 0, 0, &rules)
            .unwrap();
        assert_eq!(sim.substrate.entities.get(inside).unwrap().cloak.as_ref().unwrap().state, 0);
        assert_eq!(sim.substrate.entities.get(outside).unwrap().cloak.as_ref().unwrap().state, 2);
    }
}
