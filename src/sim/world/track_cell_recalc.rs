//! Enter5683C0/Exit5687F0 relookup and Recalc(-1) for ordinary track Mark.
//! The resident catalog currently admits ordinary nonanimated/non-Tunnel
//! cells in bridge theaters. Unsupported constructors remain an explicit
//! runtime Recalc gap; the caller retains the preceding list/raw writes.

use super::{Simulation, navigation::NavigationCaches};
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::ruleset::RuleSet;

impl Simulation {
    pub(crate) fn recalculate_track_cell(
        &mut self,
        coord: (u16, u16),
        rules: Option<&RuleSet>,
        registry: Option<&OverlayTypeRegistry>,
    ) {
        let (Some(rules), Some(registry)) = (rules, registry) else {
            return;
        };
        let Some(index) = self
            .resolved_terrain
            .as_ref()
            .and_then(|terrain| terrain.native_fixed_cell_index(coord.0 as i16, coord.1 as i16))
        else {
            return;
        };
        let Some(overlay) = self
            .overlay_grid
            .as_ref()
            .and_then(|grid| grid.finalized_map_cell(coord.0, coord.1))
        else {
            return;
        };
        let structure_blocked =
            self.substrate
                .occupancy
                .get(coord.0, coord.1)
                .is_some_and(|list| {
                    list.iter_layer(crate::sim::movement::locomotor::MovementLayer::Ground)
                        .any(|entry| {
                            let Some(entity) = self.substrate.entities.get(entry.entity_id) else {
                                return false;
                            };
                            if entity.category != crate::map::entities::EntityCategory::Structure
                                || !entity.lifecycle.cell_marked
                            {
                                return false;
                            }
                            let object = rules.object(self.interner.resolve(entity.type_ref()));
                            let foundation = crate::sim::production::building_base_foundation_cells(
                                entity.position.rx,
                                entity.position.ry,
                                object.map_or("1x1", |object| object.foundation.as_str()),
                            );
                            crate::sim::production::building_movement_blocking_cells(
                                &foundation,
                                object.is_some_and(|object| object.bib),
                            )
                            .contains(&coord)
                        })
                });
        let terrain = self.resolved_terrain.as_mut().unwrap();
        let outcome = match terrain.recalc_resident_bridge_cell(
            index,
            overlay,
            -1,
            registry,
            self.playfield_bounds,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                log::warn!("ordinary movement Recalc at {coord:?} is not admitted: {error}");
                return;
            }
        };
        if let Some(runtime) = self
            .bridge_state
            .as_mut()
            .and_then(|state| state.cell_mut(coord.0, coord.1))
        {
            runtime.deck_level = terrain.cells()[index].bridge_deck_level;
        }
        self.overlay_grid
            .as_mut()
            .unwrap()
            .write_finalized_map_cell(coord.0, coord.1, outcome.finalized);
        if let Err(error) = (NavigationCaches {
            terrain_costs: &mut self.terrain_costs,
            zones: &mut self.zone_grid,
            path: &mut self.path_grid,
            playfield_bounds: self.playfield_bounds,
        })
        .publish_recalculated_cell_with_presence(
            terrain,
            self.bridge_state.as_ref(),
            coord,
            structure_blocked,
        ) {
            log::warn!("ordinary movement Recalc publication at {coord:?} failed: {error}");
        }
    }
}
