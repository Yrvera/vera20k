//! Building43FB20 operational edges publish gaps at the object's Logic turn.
//! House power assessment changes inputs; it does not call gap add/remove.
//! Evidence: PHASE3_GAP_OPERATIONAL_PUBLICATION_NATIVE_REPORT.md.
use super::Simulation;
use crate::map::entities::EntityCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::{intern::InternedId, power_system, vision};

impl Simulation {
    fn gap_viewers(&self) -> Vec<InternedId> {
        self.houses
            .keys()
            .chain(self.fog.by_owner.keys())
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    /// Selected stock GAGAP domain of4555D0. Constructor660 is true; no
    /// represented writer supplies504/67C/6CC/PoweredSpecial here. Optional
    /// producer domains remain explicit, not inferred from unrelated fields.
    pub(super) fn gap_operational_state(&self, id: u64, rules: &RuleSet) -> Option<(bool, i32)> {
        let entity = self.substrate.entities.get(id)?;
        if entity.category != EntityCategory::Structure
            || !entity.lifecycle.object_alive
            || entity.lifecycle.in_limbo
        {
            return None;
        }
        let object = rules.object(self.interner.resolve(entity.type_ref()))?;
        if !object.gap_generator {
            return None;
        }
        let operational = entity.health.current != 0
            //Actual Rust placement currently retains Construction in the
            //BuildingUp owner, without publishing that native Mission yet.
            //Keep its admission closed until that represented build completes.
            && entity.building_up.is_none()
            && !matches!(entity.mission.effective().raw(), 0x12 | 0x13)
            && power_system::is_building_powered(&self.power_states, rules, entity, &self.interner);
        Some((operational, i32::from(object.gap_radius_in_cells)))
    }

    pub(super) fn visit_building_gap(&mut self, id: u64, rules: &RuleSet) {
        let Some((operational, radius)) = self.gap_operational_state(id, rules) else {
            return;
        };
        if self
            .substrate
            .entities
            .get(id)
            .unwrap()
            .gap_generator
            .last_operational
            == operational
        {
            return;
        }
        let viewers = self.gap_viewers();
        for viewer in viewers {
            self.set_building_gap_for_viewer(id, viewer, operational, radius);
        }
        //43FBEF stores6C8 after the dispatcher returns. A SpySat bracket does
        //not change this sample, even when it changes one viewer's269.
        self.substrate
            .entities
            .get_mut(id)
            .unwrap()
            .gap_generator
            .last_operational = operational;
    }

    fn set_building_gap_for_viewer(
        &mut self,
        id: u64,
        viewer: InternedId,
        active: bool,
        radius: i32,
    ) {
        let Some(entity) = self.substrate.entities.get_mut(id) else {
            return;
        };
        let deposit = entity.gap_generator.viewers.entry(viewer).or_default();
        if deposit.active == active {
            return;
        }
        if deposit.radius == 0 {
            deposit.radius = radius;
        }
        deposit.active = active;
        let radius = deposit.radius;
        let source = vision::GapGeneratorSource {
            stable_id: id,
            owner: entity.owner(),
            rx: entity.position.rx,
            ry: entity.position.ry,
            radius,
        };
        self.fog.alliances = self.house_alliances.clone();
        // A normal scenario constructs its viewer Houses before Technos.
        self.fog
            .by_owner
            .entry(viewer)
            .or_insert_with(|| vision::OwnerVisibility::new(self.fog.width, self.fog.height));
        let spy_sat_active = self
            .houses
            .iter()
            .filter_map(|(&owner, house)| house.spy_sat_active.then_some(owner))
            .collect();
        vision::publish_gap_generator_event(
            &mut self.fog,
            viewer,
            source,
            active,
            &self.interner,
            &spy_sat_active,
        );
    }

    /// Techno6F6AC0 calls ordinary release, then6FB470, before Object Conceal.
    /// Removal has no operational check and leaves cached26C/Building6C8 alone.
    pub(super) fn remove_building_gap_before_limbo(&mut self, id: u64) {
        let Some(entity) = self.substrate.entities.get(id) else {
            return;
        };
        let viewers: Vec<_> = entity
            .gap_generator
            .viewers
            .iter()
            .filter_map(|(&viewer, deposit)| deposit.active.then_some((viewer, deposit.radius)))
            .collect();
        for (viewer, radius) in viewers {
            self.set_building_gap_for_viewer(id, viewer, false, radius);
        }
    }

    /// Building448260 removes before owner transfer and rechecks4555D0 after
    /// it, independently of6C8. Production ownership supplies the RuleSet.
    pub(super) fn reapply_building_gap_after_owner_change(&mut self, id: u64, rules: &RuleSet) {
        let Some((operational, radius)) = self.gap_operational_state(id, rules) else {
            return;
        };
        let viewers = self.gap_viewers();
        for viewer in viewers {
            self.set_building_gap_for_viewer(id, viewer, operational, radius);
        }
    }

    ///701875's ordinary reveal follows the owner swap and precedes Building
    ///448260's gap re-add. Use the existing source geometry/admission owner.
    pub(super) fn reveal_building_sight_after_owner_change(
        &mut self,
        id: u64,
        rules: Option<&RuleSet>,
    ) {
        let Some(entity) = self.substrate.entities.get(id) else {
            return;
        };
        if entity.category != EntityCategory::Structure {
            return;
        }
        self.fog.alliances = self.house_alliances.clone();
        let config = vision::VisionConfig {
            require_playfield_membership: self.playfield_bounds.is_some(),
            veteran_sight: rules.map_or(0.0, |r| r.general.veteran_sight),
            leptons_per_sight_increase: rules.map_or(0, |r| r.general.leptons_per_sight_increase),
            reveal_by_height: rules.is_none_or(|r| r.general.reveal_by_height),
            fog_of_war: self.session.game_options.fog_of_war,
        };
        let heights = config
            .reveal_by_height
            .then(|| {
                self.path_grid
                    .as_ref()
                    .map(|grid| grid.ground_height_grid())
            })
            .flatten();
        let ability = vision::entity_has_sight_ability(entity, &self.interner, rules);
        vision::reveal_entity_vision(
            &mut self.fog,
            entity,
            &config,
            heights.as_deref(),
            ability,
            &self.interner,
        );
    }

    /// The live4ADEE0/4ADCD0 bracket visits gap candidates even if they had no
    /// prior deposit. Only this viewer's269/26C changes;6C8 stays untouched.
    /// Returns hostile cell re-admissions and whether ANY (also friendly)
    /// successful add clears House240 after the bulk mapping store.
    pub(super) fn prepare_spy_sat_gap_reentry(
        &mut self,
        viewer: InternedId,
        rules: &RuleSet,
    ) -> (Vec<vision::GapGeneratorSource>, bool) {
        let candidates: Vec<_> = self
            .substrate
            .entities
            .iter_sorted()
            .filter_map(|(id, entity)| {
                let (operational, radius) = self.gap_operational_state(id, rules)?;
                let own = entity.owner() == viewer;
                let allied = crate::map::houses::is_allied_with(
                    &self.house_alliances,
                    self.interner.resolve(entity.owner()),
                    self.interner.resolve(viewer),
                );
                // Nonhuman allied Building branch is ordinary-sight ONLY. The
                //known-own multiplayer discovery domain is shared with the caller.
                (own || !allied).then_some((id, operational, radius))
            })
            .collect();
        let mut reentry = Vec::new();
        let mut admitted = false;
        for (id, operational, radius) in candidates {
            let entity = self.substrate.entities.get_mut(id).unwrap();
            let deposit = entity.gap_generator.viewers.entry(viewer).or_default();
            // Removal first; cached radius survives. Original add subsequently
            //rechecks current operational state, not the last Building sample.
            if deposit.radius == 0 && (deposit.active || operational) {
                deposit.radius = radius;
            }
            deposit.active = operational;
            admitted |= operational;
            let radius = deposit.radius;
            if operational
                && !crate::map::houses::are_houses_friendly(
                    &self.house_alliances,
                    self.interner.resolve(entity.owner()),
                    self.interner.resolve(viewer),
                )
            {
                reentry.push(vision::GapGeneratorSource {
                    stable_id: id,
                    owner: entity.owner(),
                    rx: entity.position.rx,
                    ry: entity.position.ry,
                    radius,
                });
            }
        }
        (reentry, admitted)
    }
}
