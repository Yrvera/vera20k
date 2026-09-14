//! World placement corridors of Walk75AEC0: boundary75C117 and completion75BD7D.
//! The paid approach itself remains the existing Walk numeric adapter. This
//! owner supplies the synchronous Mark/PerCell boundary before the pass tail.
use super::{ground_pose, locomotor::MovementLayer};
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::ruleset::RuleSet;
use crate::sim::{
    components::DriveCoord, occupancy::CellListInsertion, pathfinding::PathGrid, world::Simulation,
};

impl Simulation {
    // Foot4D3780/Object5F5850 publish the mark byte before Enter/Exit.
    // The raw gate is Foot+6B6; Walk's retained-head producer never toggles it.
    fn walk_mark_remove(
        &mut self,
        id: u64,
        rules: Option<&RuleSet>,
        fallback: Option<&PathGrid>,
        registry: Option<&OverlayTypeRegistry>,
    ) {
        let removed = self.substrate.entities.get_mut(id).and_then(|e| {
            if e.lifecycle.in_limbo || !e.lifecycle.cell_marked {
                return None;
            }
            e.lifecycle.cell_marked = false;
            Some((
                (e.position.rx, e.position.ry),
                if e.on_bridge {
                    MovementLayer::Bridge
                } else {
                    MovementLayer::Ground
                },
                e.foot_occupation_enabled,
                e.owner(),
                ground_pose::position_world_coord(&e.position),
            ))
        });
        if let Some((cell, layer, enabled, owner, coord)) = removed {
            self.substrate
                .occupancy
                .remove_on_layer(cell.0, cell.1, id, layer);
            if enabled {
                super::walk_head::raw_at(
                    &mut self.substrate.raw_cell_occupation,
                    owner,
                    coord,
                    false,
                    self.resolved_terrain.as_ref(),
                    self.path_grid.as_deref().or(fallback),
                );
            }
            self.recalculate_track_cell(cell, rules, registry);
        }
    }

    fn walk_mark_put(
        &mut self,
        id: u64,
        rules: Option<&RuleSet>,
        fallback: Option<&PathGrid>,
        registry: Option<&OverlayTypeRegistry>,
    ) {
        let entered = self.substrate.entities.get_mut(id).and_then(|e| {
            if e.lifecycle.in_limbo || e.lifecycle.cell_marked {
                return None;
            }
            e.lifecycle.cell_marked = true;
            e.occupancy_enter_order = self.substrate.next_occupancy_enter_order.next();
            let cell = (e.position.rx, e.position.ry);
            self.substrate.occupancy.add(
                cell.0,
                cell.1,
                id,
                if e.on_bridge {
                    MovementLayer::Bridge
                } else {
                    MovementLayer::Ground
                },
                e.sub_cell,
                CellListInsertion::from_category(e.category),
            );
            Some((
                cell,
                e.foot_occupation_enabled,
                e.owner(),
                ground_pose::position_world_coord(&e.position),
            ))
        });
        if let Some((cell, enabled, owner, coord)) = entered {
            if enabled {
                super::walk_head::raw_at(
                    &mut self.substrate.raw_cell_occupation,
                    owner,
                    coord,
                    true,
                    self.resolved_terrain.as_ref(),
                    self.path_grid.as_deref().or(fallback),
                );
            }
            self.recalculate_track_cell(cell, rules, registry);
        }
    }

    /// Walk75C117..75C1AE relinks current XYZ while retaining the paid head
    /// and Foot path entry. This corridor does not invoke PerCell.
    pub(crate) fn run_walk_boundary(
        &mut self,
        id: u64,
        coord: DriveCoord,
        rules: Option<&RuleSet>,
        fallback: Option<&PathGrid>,
        registry: Option<&OverlayTypeRegistry>,
    ) {
        let Some(old_cell) = self
            .substrate
            .entities
            .get(id)
            .map(|e| (e.position.rx, e.position.ry))
        else {
            return;
        };
        self.walk_mark_remove(id, rules, fallback, registry);
        let Some(e) = self.substrate.entities.get_mut(id) else {
            return;
        };
        put_walk_coords(&mut e.position, coord);
        let cell = (e.position.rx, e.position.ry);
        let active_layer = e
            .movement_target
            .as_ref()
            .map(|t| t.layer_at(t.next_index))
            .unwrap_or_else(|| {
                if e.on_bridge {
                    MovementLayer::Bridge
                } else {
                    MovementLayer::Ground
                }
            });
        // Recalc may replace the canonical PathGrid. Read its current cell
        // flags after REMOVE, then SetHeight samples current resolved terrain.
        let update = super::movement_bridge::resolve_cell_transition_bridge_state(
            &mut e.position,
            self.path_grid.as_deref().or(fallback),
            old_cell,
            cell,
            e.on_bridge,
        );
        super::movement_bridge::apply_pending_bridge_render_state(
            &mut e.locomotor,
            &mut e.bridge_occupancy,
            &mut e.on_bridge,
            active_layer,
            update,
            id,
        );
        ground_pose::commit_ground_height(
            &mut e.position,
            e.on_bridge,
            self.resolved_terrain.as_ref(),
            self.path_grid.as_deref().or(fallback),
        );
        // OccupancyGrid is a list projection, not an independent subcell
        // reservation chooser. The current coordinate supplies its slot.
        e.sub_cell = Some(crate::sim::cell_kernel::infantry_preferred_spot(
            crate::sim::cell_kernel::CellQueryPoint {
                x: coord.x,
                y: coord.y,
            },
        ));
        if let Some(t) = e.movement_target.as_mut() {
            t.path_blocked = false;
            t.blocked_delay = 0;
        }
        self.walk_mark_put(id, rules, fallback, registry);
    }

    pub(crate) fn run_completed_walk_step(
        &mut self,
        id: u64,
        head: DriveCoord,
        rules: Option<&RuleSet>,
        fallback: Option<&PathGrid>,
        registry: Option<&OverlayTypeRegistry>,
    ) -> Result<bool, crate::sim::world::FrameAdvanceError> {
        self.walk_mark_remove(id, rules, fallback, registry);
        let Some(e) = self.substrate.entities.get_mut(id) else {
            return Ok(false);
        };
        let head = e
            .locomotor
            .as_ref()
            .and_then(|l| l.step_head())
            .unwrap_or(head);
        let old_head = e.locomotor.as_ref().and_then(|l| l.step_head());
        super::path_markers::consume_path_replay(&mut e.navigation.path_replay, 1);
        put_walk_coords(&mut e.position, head);
        e.navigation.path_replay.reference_cell =
            Some((e.position.rx as i16, e.position.ry as i16));
        if let Some(target) = e.movement_target.as_mut() {
            super::movement_step::configure_motion_after_transition(
                &mut e.navigation.path_replay,
                target,
                &e.locomotor,
                &mut e.drive_track,
                &mut e.drive_locomotion,
                &mut e.ship_locomotion,
                &mut e.facing,
                &mut e.facing_target,
                e.category,
                0,
                &e.position,
            );
        }
        //Infantry+1CC=5F5FA0; marked is already false, so the SetHeight0
        //receiver samples current ground+OnBridge without nested Mark calls.
        ground_pose::commit_ground_height(
            &mut e.position,
            e.on_bridge,
            self.resolved_terrain.as_ref(),
            self.path_grid.as_deref().or(fallback),
        );
        let current = ground_pose::position_world_coord(&e.position);
        let owner = e.owner();
        if let Some(old) = old_head {
            super::walk_head::raw_at(
                &mut self.substrate.raw_cell_occupation,
                owner,
                old,
                false,
                self.resolved_terrain.as_ref(),
                self.path_grid.as_deref().or(fallback),
            );
        }
        if let Some(loco) = e.locomotor.as_mut() {
            loco.set_step_head(None);
            loco.subcell_dest = Some((e.position.sub_x, e.position.sub_y));
        }
        e.sub_cell = Some(super::bump_crush::priority_sub_cell(
            e.position.sub_x,
            e.position.sub_y,
        ));
        super::walk_head::raw_at(
            &mut self.substrate.raw_cell_occupation,
            owner,
            current,
            true,
            self.resolved_terrain.as_ref(),
            self.path_grid.as_deref().or(fallback),
        );
        let changed = if let Some(rules) = rules {
            self.infantry_per_cell_bridge_repair(id, rules, registry)?
        } else {
            false
        };
        let survives = self.substrate.entities.get(id).is_some_and(|e| {
            e.lifecycle.object_alive && !e.lifecycle.in_limbo && e.object_is_falling_down == 0
        });
        if !survives {
            return Ok(changed);
        }
        if let Some(rules) = rules {
            self.refresh_unit_sensor_at_per_cell(id, rules);
            crate::sim::world::techno_ai_cloak::uncloak_on_sensor_neighbour_after_cell_entry(
                self, id, rules,
            );
            self.promote_entity_playfield_membership_after_move(id);
        }
        if let Some(e) = self.substrate.entities.get_mut(id) {
            super::navcom::finish_walk_navigation(e);
        }
        self.walk_mark_put(id, rules, fallback, registry);
        Ok(changed)
    }
}

fn put_walk_coords(position: &mut crate::sim::components::Position, coord: DriveCoord) {
    position.rx = (coord.x / 256) as u16;
    position.ry = (coord.y / 256) as u16;
    position.sub_x = crate::util::fixed_math::SimFixed::from_num(coord.x % 256);
    position.sub_y = crate::util::fixed_math::SimFixed::from_num(coord.y % 256);
    position.exact_z_leptons = Some(coord.z);
}
