//! Ordered ObjectClass-style lifecycle authority.
//!
//! Owns the independent Reveal, Conceal/Limbo, UnInit, LogicVector membership,
//! and pending-delete transitions.  Upper-layer work is emitted as ordered data;
//! this module never depends on render, UI, sidebar, audio, or net.

use crate::map::entities::EntityCategory;
use crate::sim::combat::TargetKind;
use crate::sim::components::NavTargetRef;
use crate::sim::intern::InternedId;
use crate::sim::lifecycle_request::LifecycleRequest;
use crate::sim::occupancy::{
    BUILDING_OCCUPATION_BIT, CellListInsertion, OBJECT_OCCUPATION_BIT, VEHICLE_OCCUPATION_BIT,
    cell_list_layer_for_entity, entity_occupancy_cells, infantry_raw_occupation_mask,
};
use crate::sim::passenger::PassengerRole;
use crate::util::fixed_math::SimFixed;

use super::Simulation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlacementEvidence {
    RejectedEarly,
    MarkFailed,
    MarkSucceeded,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RevealPosition {
    /// Isometric map-cell coordinate, not a screen axis.
    pub rx: u16,
    /// Isometric map-cell coordinate, not a screen axis.
    pub ry: u16,
    /// Current Rust height level, not pixels or leptons.
    pub z: u8,
    /// Lepton offset inside the cell (256 leptons per cell).
    pub sub_x: SimFixed,
    /// Lepton offset inside the cell (256 leptons per cell).
    pub sub_y: SimFixed,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RevealRequest {
    pub position: RevealPosition,
    pub placement: PlacementEvidence,
    /// Caller-supplied result of the still-blocked native type/mode gate.
    pub logic_eligible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevealFailure {
    MissingObject,
    RejectedEarly,
    MarkFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevealOutcome {
    Revealed { logic_registered: bool },
    AlreadyRevealed,
    Failed(RevealFailure),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConcealOutcome {
    Concealed,
    AlreadyConcealed,
    MissingOrDead,
}

/// Release-visible lifecycle handoffs.  Consumers may be temporarily no-op,
/// but the stream preserves the verified native relative ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LifecycleOutput {
    RevealDisplay { stable_id: u64 },
    DisplayRemove { stable_id: u64 },
    DetachAttachedAnims { stable_id: u64 },
    StopVoc { stable_id: u64 },
    DirtyTacticalRect { stable_id: u64 },
    ClearDrawnState { stable_id: u64 },
    ClearRedraw { stable_id: u64 },
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LifecycleTestEvent {
    RevealLimboCleared,
    RevealCoordinatesCommitted,
    MarkPut,
    RawOccupationListLinked,
    RawOccupationMarked,
    CellMarked,
    RevealDisplayBoundary,
    LogicAppended,
    LogicMembershipSet,
    ConcealDeselected,
    RawOccupationListUnlinked,
    RawOccupationCleared,
    ConcealUnmarked,
    ConcealDisplayBoundary,
    ConcealAnimBoundary,
    ConcealVocBoundary,
    ConcealLogicRemoved,
    ConcealDirtyTacticalRectBoundary,
    ConcealClearDrawnStateBoundary,
    ConcealLimboSet,
    ConcealClearRedrawBoundary,
    BreakSlot {
        slot: usize,
        target: Option<u64>,
    },
    BreakSenderCleared {
        target: u64,
    },
    BreakReceiverClassEffect {
        target: u64,
    },
    BreakReceiverCleared {
        target: u64,
    },
    UninitClassPre {
        stable_id: u64,
    },
    UninitRemovalNotifyBoundary {
        stable_id: u64,
        object_alive: bool,
        cell_marked: bool,
    },
    UninitRemovalListenerVisited {
        expired_id: u64,
        listener_id: u64,
        target_alive: bool,
        target_in_limbo: bool,
    },
    UninitAliveCleared {
        stable_id: u64,
    },
    PendingDeleteQueued {
        stable_id: u64,
    },
    BinaryFrameCommitted,
    PendingDeleteDrainStarted,
    FinalizedCommon {
        stable_id: u64,
    },
}

impl Simulation {
    #[cfg(test)]
    pub(crate) fn trace_lifecycle_for_test(&mut self, event: LifecycleTestEvent) {
        self.lifecycle_test_events.push(event);
    }

    fn current_reveal_position(&self, stable_id: u64) -> Option<RevealPosition> {
        self.substrate
            .entities
            .get(stable_id)
            .map(|entity| RevealPosition {
                rx: entity.position.rx,
                ry: entity.position.ry,
                z: entity.position.z,
                sub_x: entity.position.sub_x,
                sub_y: entity.position.sub_y,
            })
    }

    fn raw_occupation_cell_facts(&self, rx: u16, ry: u16) -> (i16, bool) {
        let Some(terrain_cell) = self
            .resolved_terrain
            .as_ref()
            .and_then(|terrain| terrain.cell(rx, ry))
        else {
            return (0, false);
        };
        let ground_level = i16::from(terrain_cell.level as i8);
        let live_structural_bridge = terrain_cell.bridge_facts.has_structural_bridge()
            && self
                .bridge_state
                .as_ref()
                .is_some_and(|state| state.is_bridge_walkable(rx, ry));
        (ground_level, live_structural_bridge)
    }

    fn mark_common_raw_occupation(
        &mut self,
        category: EntityCategory,
        cells: &[(u16, u16)],
        position: RevealPosition,
    ) -> bool {
        match category {
            EntityCategory::Unit => {
                let (ground_level, live_structural_bridge) =
                    self.raw_occupation_cell_facts(position.rx, position.ry);
                if i16::from(position.z as i8) >= ground_level + 4 && live_structural_bridge {
                    self.substrate.raw_cell_occupation.mark_deck(
                        position.rx,
                        position.ry,
                        VEHICLE_OCCUPATION_BIT,
                    );
                } else {
                    self.substrate.raw_cell_occupation.mark_ground(
                        position.rx,
                        position.ry,
                        VEHICLE_OCCUPATION_BIT,
                    );
                }
                true
            }
            EntityCategory::Infantry => {
                let (ground_level, live_structural_bridge) =
                    self.raw_occupation_cell_facts(position.rx, position.ry);
                let z = i16::from(position.z as i8);
                if z > ground_level + 4 {
                    return false;
                }
                let mask = infantry_raw_occupation_mask(position.sub_x, position.sub_y);
                if z >= ground_level + 4 && live_structural_bridge {
                    self.substrate
                        .raw_cell_occupation
                        .mark_deck(position.rx, position.ry, mask);
                } else {
                    self.substrate
                        .raw_cell_occupation
                        .mark_ground(position.rx, position.ry, mask);
                }
                true
            }
            EntityCategory::Structure => {
                for &(rx, ry) in cells {
                    self.substrate
                        .raw_cell_occupation
                        .mark_ground(rx, ry, BUILDING_OCCUPATION_BIT);
                }
                !cells.is_empty()
            }
            EntityCategory::Aircraft => {
                let (ground_level, live_structural_bridge) =
                    self.raw_occupation_cell_facts(position.rx, position.ry);
                if i16::from(position.z as i8) >= ground_level + 4 && live_structural_bridge {
                    self.substrate.raw_cell_occupation.mark_deck(
                        position.rx,
                        position.ry,
                        OBJECT_OCCUPATION_BIT,
                    );
                } else {
                    self.substrate.raw_cell_occupation.mark_ground(
                        position.rx,
                        position.ry,
                        OBJECT_OCCUPATION_BIT,
                    );
                }
                true
            }
        }
    }

    fn clear_common_raw_occupation(
        &mut self,
        category: EntityCategory,
        cells: &[(u16, u16)],
        position: RevealPosition,
    ) -> bool {
        match category {
            EntityCategory::Unit => {
                let (ground_level, _) = self.raw_occupation_cell_facts(position.rx, position.ry);
                if i16::from(position.z as i8) >= ground_level + 4 {
                    self.substrate.raw_cell_occupation.clear_deck(
                        position.rx,
                        position.ry,
                        VEHICLE_OCCUPATION_BIT,
                    );
                } else {
                    self.substrate.raw_cell_occupation.clear_ground(
                        position.rx,
                        position.ry,
                        VEHICLE_OCCUPATION_BIT,
                    );
                }
                true
            }
            EntityCategory::Structure => {
                for &(rx, ry) in cells {
                    self.substrate.raw_cell_occupation.clear_ground(
                        rx,
                        ry,
                        BUILDING_OCCUPATION_BIT,
                    );
                }
                !cells.is_empty()
            }
            EntityCategory::Aircraft => {
                let (ground_level, live_structural_bridge) =
                    self.raw_occupation_cell_facts(position.rx, position.ry);
                if i16::from(position.z as i8) >= ground_level + 4 && live_structural_bridge {
                    self.substrate.raw_cell_occupation.clear_deck(
                        position.rx,
                        position.ry,
                        OBJECT_OCCUPATION_BIT,
                    );
                } else {
                    self.substrate.raw_cell_occupation.clear_ground(
                        position.rx,
                        position.ry,
                        OBJECT_OCCUPATION_BIT,
                    );
                }
                true
            }
            // Generic Infantry removal intentionally leaves the destructive raw
            // bit stale; movement/sub-cell transitions own explicit clears.
            EntityCategory::Infantry => false,
        }
    }

    /// Compatibility convenience for already-admitted current-position callers.
    /// It still executes the complete result-bearing Reveal transaction.
    pub(crate) fn reveal(&mut self, stable_id: u64) -> RevealOutcome {
        if self.substrate.anims.contains_key(stable_id) {
            let registered = self.reveal_anim(stable_id);
            return RevealOutcome::Revealed {
                logic_registered: registered,
            };
        }
        if self.substrate.particle_systems.contains_key(stable_id) {
            let registered = self.reveal_particle_system(stable_id);
            return RevealOutcome::Revealed {
                logic_registered: registered,
            };
        }
        let Some(position) = self.current_reveal_position(stable_id) else {
            return RevealOutcome::Failed(RevealFailure::MissingObject);
        };
        self.try_reveal_entity(
            stable_id,
            RevealRequest {
                position,
                placement: PlacementEvidence::MarkSucceeded,
                logic_eligible: true,
            },
        )
    }

    /// ObjectClass::Reveal: clear limbo for the attempt, commit coordinates,
    /// Mark(PUT), expose display, then append eligible LogicClass membership.
    pub(crate) fn try_reveal_entity(
        &mut self,
        stable_id: u64,
        request: RevealRequest,
    ) -> RevealOutcome {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return RevealOutcome::Failed(RevealFailure::MissingObject);
        };
        if !entity.lifecycle.in_limbo {
            return RevealOutcome::AlreadyRevealed;
        }
        if entity.lifecycle.cell_marked || request.placement == PlacementEvidence::RejectedEarly {
            return RevealOutcome::Failed(RevealFailure::RejectedEarly);
        }

        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.lifecycle.in_limbo = false;
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::RevealLimboCleared);

        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.position.rx = request.position.rx;
            entity.position.ry = request.position.ry;
            entity.position.z = request.position.z;
            entity.position.sub_x = request.position.sub_x;
            entity.position.sub_y = request.position.sub_y;
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::RevealCoordinatesCommitted);

        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::MarkPut);
        if request.placement == PlacementEvidence::MarkFailed {
            if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                entity.lifecycle.in_limbo = true;
            }
            return RevealOutcome::Failed(RevealFailure::MarkFailed);
        }

        self.mark_entity_put(stable_id);
        if !self
            .substrate
            .entities
            .get(stable_id)
            .is_some_and(|entity| entity.lifecycle.object_alive)
        {
            return RevealOutcome::Revealed {
                logic_registered: false,
            };
        }
        self.lifecycle_outputs
            .push(LifecycleOutput::RevealDisplay { stable_id });
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::RevealDisplayBoundary);

        let logic_registered = if request.logic_eligible {
            self.register_logic_object(stable_id)
        } else {
            false
        };
        RevealOutcome::Revealed { logic_registered }
    }

    fn mark_entity_put(&mut self, stable_id: u64) {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return;
        };
        if entity.lifecycle.cell_marked {
            return;
        }
        let cells = entity_occupancy_cells(entity);
        let layer = cell_list_layer_for_entity(entity);
        let sub_cell = if entity.category == EntityCategory::Infantry {
            entity.sub_cell
        } else {
            None
        };
        let insertion = CellListInsertion::from_category(entity.category);
        let category = entity.category;
        let current_cell = (entity.position.rx, entity.position.ry);
        let raw_position = RevealPosition {
            rx: entity.position.rx,
            ry: entity.position.ry,
            z: entity.position.z,
            sub_x: entity.position.sub_x,
            sub_y: entity.position.sub_y,
        };
        let inside_transport = entity.passenger_role.is_inside_transport();
        let order = self.substrate.next_occupancy_enter_order.next();

        if !inside_transport {
            if let Some(layer) = layer {
                for &(rx, ry) in &cells {
                    self.substrate
                        .occupancy
                        .add(rx, ry, stable_id, layer, sub_cell, insertion);
                }
                if matches!(
                    category,
                    EntityCategory::Unit
                        | EntityCategory::Infantry
                        | EntityCategory::Structure
                        | EntityCategory::Aircraft
                ) {
                    #[cfg(test)]
                    self.trace_lifecycle_for_test(LifecycleTestEvent::RawOccupationListLinked);
                    if self.mark_common_raw_occupation(category, &cells, raw_position) {
                        #[cfg(test)]
                        self.trace_lifecycle_for_test(LifecycleTestEvent::RawOccupationMarked);
                    }
                }
                if category == EntityCategory::Unit {
                    self.substrate.cell_occupation.mark_vehicle_on_layer(
                        current_cell.0,
                        current_cell.1,
                        stable_id,
                        layer,
                    );
                }
            }
        }
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.occupancy_enter_order = order;
            entity.lifecycle.cell_marked = true;
            if let Some(drive) = entity.drive_locomotion.as_mut() {
                drive.current_occupation_cleared = false;
            }
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::CellMarked);
    }

    fn unmark_entity_remove(&mut self, stable_id: u64) -> bool {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return false;
        };
        if !entity.lifecycle.cell_marked {
            return false;
        }
        let cells = entity_occupancy_cells(entity);
        let layer = cell_list_layer_for_entity(entity);
        let category = entity.category;
        let current_cell = (entity.position.rx, entity.position.ry);
        let raw_position = RevealPosition {
            rx: entity.position.rx,
            ry: entity.position.ry,
            z: entity.position.z,
            sub_x: entity.position.sub_x,
            sub_y: entity.position.sub_y,
        };
        let inside_transport = entity.passenger_role.is_inside_transport();
        if category == EntityCategory::Unit {
            let (entities, occupation) = (
                &mut self.substrate.entities,
                &mut self.substrate.cell_occupation,
            );
            if let Some(drive) = entities
                .get_mut(stable_id)
                .and_then(|entity| entity.drive_locomotion.as_mut())
            {
                crate::sim::occupancy::clear_drive_head_to_occupation_for_remove(
                    drive, occupation, stable_id,
                );
            }
        }
        if let Some(layer) = layer {
            for &(rx, ry) in &cells {
                self.substrate
                    .occupancy
                    .remove_on_layer(rx, ry, stable_id, layer);
            }
            if !inside_transport
                && matches!(
                    category,
                    EntityCategory::Unit
                        | EntityCategory::Infantry
                        | EntityCategory::Structure
                        | EntityCategory::Aircraft
                )
            {
                #[cfg(test)]
                self.trace_lifecycle_for_test(LifecycleTestEvent::RawOccupationListUnlinked);
                if self.clear_common_raw_occupation(category, &cells, raw_position) {
                    #[cfg(test)]
                    self.trace_lifecycle_for_test(LifecycleTestEvent::RawOccupationCleared);
                }
            }
            if category == EntityCategory::Unit {
                self.substrate.cell_occupation.clear_vehicle_on_layer(
                    current_cell.0,
                    current_cell.1,
                    stable_id,
                    layer,
                );
            }
        }
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.lifecycle.cell_marked = false;
            if let Some(drive) = entity.drive_locomotion.as_mut() {
                drive.current_occupation_cleared = true;
            }
        }
        true
    }

    /// Test/fixture helper retained at the transaction boundary.  It is
    /// idempotent and updates the authoritative `cell_marked` fact.
    pub(crate) fn add_entity_occupancy(&mut self, stable_id: u64) {
        self.mark_entity_put(stable_id);
    }

    /// Existing movement and fixture boundary; common lifecycle code calls the
    /// private unmark transaction instead.
    pub(crate) fn remove_entity_occupancy(&mut self, stable_id: u64) {
        self.unmark_entity_remove(stable_id);
    }

    /// Run one production air-process visit with the active Fly
    /// remove-before/process/add-after cell-list transaction around it.
    pub(crate) fn tick_air_movement_with_cell_lists_one(
        &mut self,
        stable_id: u64,
    ) -> crate::sim::movement::air_movement::AirMovementTickStats {
        use crate::rules::locomotor_type::LocomotorKind;
        use crate::sim::movement::locomotor::MovementLayer;

        let transact_fly = self.substrate.entities.get(stable_id).is_some_and(|entity| {
            entity.category == EntityCategory::Aircraft
                && entity.lifecycle.object_alive
                && !entity.lifecycle.in_limbo
                && entity.locomotor.as_ref().is_some_and(|locomotor| {
                    locomotor.kind == LocomotorKind::Fly && locomotor.layer == MovementLayer::Air
                })
        });
        if transact_fly {
            self.remove_entity_occupancy(stable_id);
        }

        let stats = crate::sim::movement::air_movement::tick_air_movement(
            &mut self.substrate.entities,
            &[stable_id],
            self.session.tick,
        );

        if transact_fly
            && self.substrate.entities.get(stable_id).is_some_and(|entity| {
                entity.lifecycle.object_alive && !entity.lifecycle.in_limbo
            })
        {
            self.add_entity_occupancy(stable_id);
        }
        stats
    }

    fn register_logic_object(&mut self, stable_id: u64) -> bool {
        let is_anim = self.substrate.anims.contains_key(stable_id);
        let is_particle_system = self.substrate.particle_systems.contains_key(stable_id);
        let already_member = if is_anim {
            self.substrate
                .anims
                .get(stable_id)
                .is_some_and(|anim| anim.in_logic_vector)
        } else if is_particle_system {
            self.substrate
                .particle_systems
                .get(stable_id)
                .is_some_and(|system| system.in_logic_vector)
        } else {
            self.substrate
                .entities
                .get(stable_id)
                .is_some_and(|entity| entity.in_logic_vector)
        };
        if already_member {
            return true;
        }
        if (!is_anim && !is_particle_system && !self.substrate.entities.contains(stable_id))
            || self.substrate.logic.try_push(stable_id).is_err()
        {
            return false;
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::LogicAppended);
        if is_anim {
            if let Some(anim) = self.substrate.anims.get_mut(stable_id) {
                anim.in_logic_vector = true;
            }
        } else if is_particle_system {
            if let Some(system) = self.substrate.particle_systems.get_mut(stable_id) {
                system.in_logic_vector = true;
            }
        } else if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.in_logic_vector = true;
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::LogicMembershipSet);
        true
    }

    fn unregister_logic_object(&mut self, stable_id: u64) -> bool {
        let is_anim = self.substrate.anims.contains_key(stable_id);
        let is_particle_system = self.substrate.particle_systems.contains_key(stable_id);
        let flagged = if is_anim {
            self.substrate
                .anims
                .get(stable_id)
                .is_some_and(|anim| anim.in_logic_vector)
        } else if is_particle_system {
            self.substrate
                .particle_systems
                .get(stable_id)
                .is_some_and(|system| system.in_logic_vector)
        } else {
            self.substrate
                .entities
                .get(stable_id)
                .is_some_and(|entity| entity.in_logic_vector)
        };
        if !flagged {
            return false;
        }
        let _ = self.substrate.logic.remove_first(stable_id);
        if is_anim {
            if let Some(anim) = self.substrate.anims.get_mut(stable_id) {
                anim.in_logic_vector = false;
            }
        } else if is_particle_system {
            if let Some(system) = self.substrate.particle_systems.get_mut(stable_id) {
                system.in_logic_vector = false;
            }
        } else if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.in_logic_vector = false;
        }
        true
    }

    /// Test-only access to the exact LogicVector helper ordering.
    #[cfg(test)]
    pub(crate) fn register_live_object(&mut self, stable_id: u64) {
        let _ = self.register_logic_object(stable_id);
    }

    /// Test-only access to the exact LogicVector helper ordering.
    #[cfg(test)]
    pub(crate) fn unregister_live_object(&mut self, stable_id: u64) {
        let _ = self.unregister_logic_object(stable_id);
    }

    pub(crate) fn reveal_anim(&mut self, stable_id: u64) -> bool {
        if !self
            .substrate
            .anims
            .get(stable_id)
            .is_some_and(|anim| !anim.runtime.inactive)
        {
            return false;
        }
        self.register_logic_object(stable_id)
    }

    pub(crate) fn conceal_anim(&mut self, stable_id: u64) -> bool {
        self.unregister_logic_object(stable_id)
    }

    pub(crate) fn reveal_particle_system(&mut self, stable_id: u64) -> bool {
        if !self.substrate.particle_systems.contains_key(stable_id) {
            return false;
        }
        self.register_logic_object(stable_id)
    }

    pub(crate) fn conceal_particle_system(&mut self, stable_id: u64) -> bool {
        self.unregister_logic_object(stable_id)
    }

    /// Open-topped cargo entry hides the passenger but then directly restores
    /// its active membership. This is deliberately not Reveal: the passenger
    /// remains limbo/unmarked while its AI stays in the live object order.
    pub(crate) fn register_open_topped_passenger(&mut self, stable_id: u64) -> bool {
        if !self.substrate.entities.contains(stable_id) {
            return false;
        }
        self.register_logic_object(stable_id)
    }

    /// Compatibility dispatch which keeps AnimClass logic-only and routes every
    /// GameEntity through the complete common Object Conceal transaction.
    pub(crate) fn conceal(&mut self, stable_id: u64) -> ConcealOutcome {
        if self.substrate.anims.contains_key(stable_id) {
            return if self.conceal_anim(stable_id) {
                ConcealOutcome::Concealed
            } else {
                ConcealOutcome::AlreadyConcealed
            };
        }
        if self.substrate.particle_systems.contains_key(stable_id) {
            return if self.conceal_particle_system(stable_id) {
                ConcealOutcome::Concealed
            } else {
                ConcealOutcome::AlreadyConcealed
            };
        }
        self.object_conceal(stable_id)
    }

    /// ObjectClass::Conceal represented order.  Native alive remains true.
    pub(crate) fn object_conceal(&mut self, stable_id: u64) -> ConcealOutcome {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return ConcealOutcome::MissingOrDead;
        };
        if !entity.lifecycle.object_alive {
            return ConcealOutcome::MissingOrDead;
        }
        if entity.lifecycle.in_limbo {
            return ConcealOutcome::AlreadyConcealed;
        }

        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.selected = false;
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealDeselected);

        if self.unmark_entity_remove(stable_id) {
            #[cfg(test)]
            self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealUnmarked);
        }

        self.lifecycle_outputs
            .push(LifecycleOutput::DisplayRemove { stable_id });
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealDisplayBoundary);
        self.lifecycle_outputs
            .push(LifecycleOutput::DetachAttachedAnims { stable_id });
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealAnimBoundary);
        self.lifecycle_outputs
            .push(LifecycleOutput::StopVoc { stable_id });
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealVocBoundary);

        if self.unregister_logic_object(stable_id) {
            #[cfg(test)]
            self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealLogicRemoved);
        }

        let dirty_rect_eligible = self
            .substrate
            .entities
            .get(stable_id)
            .is_some_and(|entity| entity.dirty_rect_eligible);
        if dirty_rect_eligible {
            self.lifecycle_outputs
                .push(LifecycleOutput::DirtyTacticalRect { stable_id });
            #[cfg(test)]
            self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealDirtyTacticalRectBoundary);
        }
        self.lifecycle_outputs
            .push(LifecycleOutput::ClearDrawnState { stable_id });
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealClearDrawnStateBoundary);

        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.lifecycle.in_limbo = true;
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealLimboSet);
        self.lifecycle_outputs
            .push(LifecycleOutput::ClearRedraw { stable_id });
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealClearRedrawBoundary);
        ConcealOutcome::Concealed
    }

    /// TechnoClass Limbo sends synchronous BREAK to every contact before the
    /// common Object Conceal transaction.
    pub(crate) fn techno_limbo(&mut self, stable_id: u64) -> ConcealOutcome {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return ConcealOutcome::MissingOrDead;
        };
        if !entity.lifecycle.object_alive {
            return ConcealOutcome::MissingOrDead;
        }
        if entity.lifecycle.in_limbo {
            return ConcealOutcome::AlreadyConcealed;
        }
        crate::sim::radio::broadcast_break(self, stable_id);
        self.object_conceal(stable_id)
    }

    /// Existing Rust owner-count mutation with an explicit exactly-once guard.
    pub(crate) fn release_owned_count_once(&mut self, stable_id: u64) {
        let Some((owner, category, already_released, destroyed, killed_by, award)) =
            self.substrate.entities.get(stable_id).map(|entity| {
                (
                    entity.owner,
                    entity.category,
                    entity.owned_count_released,
                    entity.health.current == 0,
                    entity.killed_by,
                    entity.kill_award_points,
                )
            })
        else {
            return;
        };
        if already_released {
            return;
        }
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.owned_count_released = true;
        }
        let owner_name = self.interner.resolve(owner).to_string();
        self.decrement_owned_count(&owner_name, category);
        if destroyed {
            self.record_match_kill_and_loss(owner, category, killed_by, award);
        }
    }

    /// Score-screen bookkeeping for one destroyed object: a loss for its owner, a
    /// kill for the house credited with destroying it, and that house's score
    /// award.
    ///
    /// This runs at the single owned-count release point rather than in the
    /// damage loop so it fires exactly once per object, but it does NOT
    /// re-derive the killer here — `killed_by` was captured at the instant of
    /// destruction, which is where gamemd records it.
    ///
    /// The kill is counted regardless of how the killer relates to the victim:
    /// gamemd increments the killing house's kill table for allied and
    /// self-inflicted destruction too, and suppresses only the *points*. (It also
    /// has a victim-type suppression flag with no VERA equivalent yet —
    /// UNCHECKED, not modelled.) Sold or otherwise despawned objects reach this
    /// helper with non-zero health and the caller filters them out.
    fn record_match_kill_and_loss(
        &mut self,
        owner: InternedId,
        category: EntityCategory,
        killed_by: Option<InternedId>,
        award: i32,
    ) {
        let structure = category == EntityCategory::Structure;
        if let Some(house) = self.houses.get_mut(&owner) {
            if structure {
                house.stats.buildings_lost = house.stats.buildings_lost.saturating_add(1);
            } else {
                house.stats.units_lost = house.stats.units_lost.saturating_add(1);
            }
        }
        let Some(killer) = killed_by else {
            return;
        };
        // Destroying an ally's object (or one's own) still counts as a kill but
        // is worth no score.
        let friendly = crate::map::houses::are_houses_friendly(
            &self.house_alliances,
            self.interner.resolve(killer),
            self.interner.resolve(owner),
        );
        if let Some(house) = self.houses.get_mut(&killer) {
            if structure {
                house.stats.buildings_killed = house.stats.buildings_killed.saturating_add(1);
            } else {
                house.stats.units_killed = house.stats.units_killed.saturating_add(1);
            }
            if !friendly {
                house.stats.score_points = house.stats.score_points.saturating_add(award);
            }
        }
    }

    pub(crate) fn apply_lifecycle_request(&mut self, request: LifecycleRequest) {
        match request {
            LifecycleRequest::Uninit {
                stable_id,
                reason: _,
            } => self.uninit(stable_id),
        }
    }

    fn run_represented_uninit_pre_hook(&mut self, stable_id: u64) {
        self.clear_building_damage_fire_slots(stable_id);
        self.release_owned_count_once(stable_id);
        crate::sim::docking::bunker_link::break_links_on_despawn(self, stable_id);
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::UninitClassPre { stable_id });
    }

    fn uninit_carried_passengers(&mut self, carrier_id: u64) {
        let passenger_ids = self
            .substrate
            .entities
            .get_mut(carrier_id)
            .and_then(|carrier| carrier.passenger_role.cargo_mut())
            .map_or_else(Vec::new, |cargo| cargo.take_for_uninit());

        for passenger_id in passenger_ids {
            debug_assert_ne!(
                passenger_id, carrier_id,
                "transport {carrier_id} contains itself"
            );
            if passenger_id == carrier_id {
                continue;
            }
            if let Some(passenger) = self.substrate.entities.get_mut(passenger_id) {
                if matches!(
                    passenger.passenger_role,
                    PassengerRole::Inside { transport_id } if transport_id == carrier_id
                ) {
                    passenger.passenger_role = PassengerRole::None;
                }
                passenger.health.current = 0;
            }
            self.uninit(passenger_id);
        }
    }

    fn nav_ref_targets_expired(target: &NavTargetRef, expired_id: u64) -> bool {
        matches!(
            target,
            NavTargetRef::Entity { id }
                | NavTargetRef::Object { id }
                | NavTargetRef::Building { id }
                if *id == expired_id
        )
    }

    /// Represented entries in global ObjectClass construction order. Stable
    /// IDs are monotonic and never reused, so merging the separate Rust stores
    /// by ID reproduces the native registration order without walking holes
    /// left by already-finalized objects.
    fn removal_listener_order(&self) -> Vec<u64> {
        let mut listeners = self.substrate.entities.keys_sorted();
        listeners.extend(self.substrate.anims.iter().map(|(&stable_id, _)| stable_id));
        listeners.extend(
            self.substrate
                .particle_systems
                .iter()
                .map(|(&stable_id, _)| stable_id),
        );
        listeners.sort_unstable();
        debug_assert!(
            listeners.windows(2).all(|pair| pair[0] != pair[1]),
            "object stable ID exists in more than one represented store"
        );
        listeners
    }

    fn notify_entity_pointer_expired(
        &mut self,
        listener_id: u64,
        expired_id: u64,
        expired_cell: (u16, u16),
        expired_is_high_flying: bool,
        expired_object_alive: bool,
        expired_health: u16,
        expired_is_selling: bool,
    ) {
        let Some(listener) = self.substrate.entities.get(listener_id) else {
            return;
        };
        let current_target_matches = listener.attack_target.as_ref().is_some_and(
            |target| matches!(target.target, TargetKind::Entity(id) if id == expired_id),
        );
        let passive_scan_remaining = listener
            .passive_scan_timer
            .remaining(self.session.binary_frame);
        let mission_is_suspended =
            listener.mission.suspended() != crate::sim::mission::MissionId::NONE;

        let passive_scan_delay = (current_target_matches && passive_scan_remaining > 10)
            .then(|| self.scenario_rng.next_range_u32_inclusive(4, 8));
        if let Some(listener) = self.substrate.entities.get_mut(listener_id) {
            if let Some(delay) = passive_scan_delay {
                listener
                    .passive_scan_timer
                    .arm(self.session.binary_frame, delay);
            }

            // RadioClass::PointerExpired nulls matching sparse slots in place.
            listener.clear_live_contact_with(expired_id);

            // TechnoClass removes an expiring passenger from its CargoClass before
            // clearing its target/archive/manager reference family.
            if let PassengerRole::Transport { cargo } = &mut listener.passenger_role {
                let _ = cargo.disembark(expired_id);
            }
        }

        if current_target_matches {
            self.set_archive_target_represented(listener_id, None)
                .expect("expiry listener remains present");
            if mission_is_suspended {
                self.mission_restore_after_target_expiry(listener_id)
                    .expect("represented expiry restore remains available");
            }
        }

        let Some(listener) = self.substrate.entities.get_mut(listener_id) else {
            return;
        };
        if matches!(
            listener.suspended_attack_target,
            Some(TargetKind::Entity(id)) if id == expired_id
        ) {
            listener.suspended_attack_target = None;
        }

        // FootClass clears SuspendedNavCom first, then its current/aux target,
        // and removes every matching queue entry. Cell targets are unaffected.
        if listener
            .navigation
            .suspended_nav_com
            .as_ref()
            .is_some_and(|target| Self::nav_ref_targets_expired(target, expired_id))
        {
            listener.navigation.suspended_nav_com = None;
        }
        let current_nav_matches = listener
            .navigation
            .nav_com
            .as_ref()
            .is_some_and(|target| Self::nav_ref_targets_expired(target, expired_id));
        let retain_capture_nav = current_nav_matches
            && listener.category == EntityCategory::Infantry
            && listener.occupier
            && listener.mission.current().known()
                == Some(crate::sim::mission::MissionType::Capture)
            && expired_object_alive
            && expired_health > 0
            && !expired_is_selling;
        if !retain_capture_nav {
            if listener
                .navigation
                .nav_com_aux
                .as_ref()
                .is_some_and(|target| Self::nav_ref_targets_expired(target, expired_id))
            {
                listener.navigation.nav_com_aux = None;
            }
            if current_nav_matches {
                listener.navigation.nav_com = None;
            }
        }
        listener
            .navigation
            .nav_queue
            .retain(|target| !Self::nav_ref_targets_expired(target, expired_id));

        if listener.capture_target == Some(expired_id) {
            listener.capture_target = None;
        }
        if listener
            .c4_plant
            .as_ref()
            .is_some_and(|plant| plant.target_building_id == expired_id)
        {
            listener.c4_plant = None;
        }

        if listener
            .dock_state
            .as_ref()
            .is_some_and(|dock| dock.dock_building_id == expired_id)
        {
            listener.dock_state = None;
        }
        if let Some(ammo) = listener.aircraft_ammo.as_mut() {
            if ammo.target_airfield == Some(expired_id) {
                ammo.target_airfield = None;
                ammo.target_pad = None;
            }
        }
        if let Some(miner) = listener.miner.as_mut() {
            if miner.home_refinery == Some(expired_id) {
                miner.home_refinery = None;
            }
            if miner.reserved_refinery == Some(expired_id) {
                miner.reserved_refinery = None;
                miner.dock_queued = false;
            }
        }

        let clear_passenger_role = match &listener.passenger_role {
            PassengerRole::Transport { .. } => false,
            PassengerRole::Boarding {
                target_transport_id,
                ..
            } => *target_transport_id == expired_id,
            PassengerRole::Inside { transport_id } => *transport_id == expired_id,
            PassengerRole::None => false,
        };
        if clear_passenger_role {
            listener.passenger_role = PassengerRole::None;
        }

        if let Some(homing) = listener.homing_state.as_mut() {
            homing.expire_object_target(expired_id, expired_cell, expired_is_high_flying);
        }

        // Deliberately retain last_attacker_id. Native retaliation reads the
        // dying object through the deferred-delete window; it is not one of
        // the proactively-cleared target roles above.
    }

    /// ObjectClass::Detach_From_All_Lists represented listener broadcast.
    ///
    /// The callback pass runs while the target remains alive, unconcealed,
    /// cell-marked, and resolvable. The represented callbacks below do not add
    /// or erase listener objects, so the native live-vector cursor and this
    /// monotonic construction-order walk have the same observable result.
    fn notify_pointer_expired(&mut self, expired_id: u64) {
        let Some((
            expired_cell,
            expired_is_high_flying,
            expired_object_alive,
            expired_health,
            expired_is_selling,
        )) = self.substrate.entities.get(expired_id).map(|expired| {
            let high_flying = expired.locomotor.as_ref().is_some_and(|locomotor| {
                // High-flying objects expire to null; lower objects preserve
                // GetHeight() >= 2 * LevelHeight (2 * 104 leptons).
                locomotor.is_airborne() && locomotor.altitude >= SimFixed::from_num(2 * 104)
            });
            (
                (expired.position.rx, expired.position.ry),
                high_flying,
                expired.lifecycle.object_alive,
                expired.health.current,
                expired.mission.current().known()
                    == Some(crate::sim::mission::MissionType::Selling),
            )
        })
        else {
            return;
        };

        for listener_id in self.removal_listener_order() {
            let is_entity = self.substrate.entities.contains(listener_id);
            let is_anim = self.substrate.anims.contains_key(listener_id);
            let is_particle = self.substrate.particle_systems.contains_key(listener_id);
            if !is_entity && !is_anim && !is_particle {
                continue;
            }

            #[cfg(test)]
            {
                let (target_alive, target_in_limbo) = self
                    .substrate
                    .entities
                    .get(expired_id)
                    .map(|target| (target.lifecycle.object_alive, target.lifecycle.in_limbo))
                    .unwrap_or((false, true));
                self.trace_lifecycle_for_test(LifecycleTestEvent::UninitRemovalListenerVisited {
                    expired_id,
                    listener_id,
                    target_alive,
                    target_in_limbo,
                });
            }

            if is_entity {
                self.notify_entity_pointer_expired(
                    listener_id,
                    expired_id,
                    expired_cell,
                    expired_is_high_flying,
                    expired_object_alive,
                    expired_health,
                    expired_is_selling,
                );
            } else if is_anim {
                self.expire_anim_owner_reference(listener_id, expired_id);
            } else if is_particle {
                let system = self
                    .substrate
                    .particle_systems
                    .get_mut(listener_id)
                    .expect("particle listener disappeared during expiry callback");
                if system.owner_entity == Some(expired_id) {
                    system.owner_entity = None;
                }
                if system.attached_entity == Some(expired_id) {
                    system.attached_entity = None;
                    system.marked_for_deletion = true;
                }
            }
        }
    }

    /// ObjectClass::UnInit represented ordering.  Physical removal is deferred.
    pub(crate) fn uninit(&mut self, stable_id: u64) {
        if !self.substrate.entities.contains(stable_id) {
            return;
        }

        self.run_represented_uninit_pre_hook(stable_id);
        self.uninit_carried_passengers(stable_id);

        #[cfg(test)]
        {
            let (object_alive, cell_marked) = self
                .substrate
                .entities
                .get(stable_id)
                .map(|entity| (entity.lifecycle.object_alive, entity.lifecycle.cell_marked))
                .unwrap_or((false, false));
            self.trace_lifecycle_for_test(LifecycleTestEvent::UninitRemovalNotifyBoundary {
                stable_id,
                object_alive,
                cell_marked,
            });
        }
        self.notify_pointer_expired(stable_id);

        let _ = self.techno_limbo(stable_id);
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.lifecycle.object_alive = false;
            entity.dying = true;
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::UninitAliveCleared { stable_id });

        // Native append has no duplicate suppression.  The drain collapses all
        // occurrences when this dead object becomes the selected ready entry.
        self.substrate.pending_delete.push(stable_id);
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::PendingDeleteQueued { stable_id });
    }

    pub(crate) fn despawn_entity(&mut self, stable_id: u64) {
        self.uninit(stable_id);
    }

    /// Particle systems stay resolvable until the ordinary common late drain.
    /// Their owned particles have already emptied before this transition.
    pub(crate) fn retire_particle_system(&mut self, stable_id: u64) {
        let ready = self
            .substrate
            .particle_systems
            .get(stable_id)
            .is_some_and(|system| system.marked_for_deletion && system.particles.is_empty());
        if !ready {
            return;
        }

        self.conceal_particle_system(stable_id);
        self.substrate.pending_delete.push(stable_id);
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::PendingDeleteQueued { stable_id });
    }

    fn pending_object_is_ready(&self, stable_id: u64) -> bool {
        if let Some(entity) = self.substrate.entities.get(stable_id) {
            return !entity.lifecycle.object_alive;
        }
        if let Some(anim) = self.substrate.anims.get(stable_id) {
            return anim.runtime.inactive;
        }
        if let Some(system) = self.substrate.particle_systems.get(stable_id) {
            return system.marked_for_deletion && system.particles.is_empty();
        }
        true
    }

    fn finalize_and_remove_common(&mut self, stable_id: u64) {
        if self.substrate.anims.contains_key(stable_id) {
            self.conceal_anim(stable_id);
            self.detach_anim_from_owner(stable_id);
        }
        let entity = self.substrate.entities.remove(stable_id);
        let anim = self.substrate.anims.remove(stable_id);
        let particle_system = self.substrate.particle_systems.finalize_remove(stable_id);
        debug_assert!(
            usize::from(entity.is_some())
                + usize::from(anim.is_some())
                + usize::from(particle_system.is_some())
                <= 1,
            "object id {stable_id} was removed from multiple stores"
        );
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::FinalizedCommon { stable_id });
    }

    fn finalize_multiplayer_feedback_anim(&mut self, stable_id: u64) {
        self.detach_anim_from_owner(stable_id);
        self.substrate.multiplayer_feedback_anims.remove(stable_id);
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::FinalizedCommon { stable_id });
    }

    /// Native-shaped pending-delete drain: preserve alive entries, collapse all
    /// duplicate ready IDs, and finalize each selected object exactly once.
    pub(crate) fn process_pending_delete(&mut self) {
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::PendingDeleteDrainStarted);
        let mut index = 0;
        while index < self.substrate.pending_delete.len() {
            let stable_id = self.substrate.pending_delete[index];
            if !self.pending_object_is_ready(stable_id) {
                index += 1;
                continue;
            }
            self.substrate
                .pending_delete
                .retain(|&queued| queued != stable_id);
            self.finalize_and_remove_common(stable_id);
        }

        while let Some(&stable_id) = self.substrate.multiplayer_feedback_pending_delete.first() {
            self.substrate
                .multiplayer_feedback_pending_delete
                .retain(|&queued| queued != stable_id);
            self.finalize_multiplayer_feedback_anim(stable_id);
        }
    }

    /// Test compatibility only.  Production has one ordinary tail drain.
    #[cfg(test)]
    pub(crate) fn flush_pending_delete(&mut self) {
        self.process_pending_delete();
    }
}
