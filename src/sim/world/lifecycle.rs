//! Ordered ObjectClass-style lifecycle authority.
//!
//! Owns the independent Reveal, Conceal/Limbo, UnInit, LogicVector membership,
//! and pending-delete transitions.  Upper-layer work is emitted as ordered data;
//! this module never depends on render, UI, sidebar, audio, or net.

use crate::map::entities::EntityCategory;
use crate::sim::lifecycle_request::LifecycleRequest;
use crate::sim::occupancy::{CellListInsertion, entity_occupancy_cells};
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
    NotAlive,
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
    CellMarked,
    RevealDisplayBoundary,
    LogicAppended,
    LogicMembershipSet,
    ConcealDeselected,
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

    /// Compatibility convenience for already-admitted current-position callers.
    /// It still executes the complete result-bearing Reveal transaction.
    pub(crate) fn reveal(&mut self, stable_id: u64) -> RevealOutcome {
        if self.substrate.anims.contains_key(stable_id) {
            let registered = self.reveal_anim(stable_id);
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
        if !entity.lifecycle.object_alive {
            return RevealOutcome::Failed(RevealFailure::NotAlive);
        }
        if !entity.lifecycle.in_limbo {
            return RevealOutcome::AlreadyRevealed;
        }
        if request.placement == PlacementEvidence::RejectedEarly {
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
            entity.position.refresh_screen_coords();
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
        let layer = entity.occupancy_list_layer();
        let sub_cell = if entity.category == EntityCategory::Infantry {
            entity.sub_cell
        } else {
            None
        };
        let insertion = CellListInsertion::from_category(entity.category);
        let inside_transport = entity.passenger_role.is_inside_transport();
        let order = self.substrate.next_occupancy_enter_order.next();

        if !inside_transport {
            if let Some(layer) = layer {
                for (rx, ry) in cells {
                    self.substrate
                        .occupancy
                        .add(rx, ry, stable_id, layer, sub_cell, insertion);
                }
            }
        }
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.occupancy_enter_order = order;
            entity.lifecycle.cell_marked = true;
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
        for (rx, ry) in cells {
            self.substrate.occupancy.remove(rx, ry, stable_id);
        }
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.lifecycle.cell_marked = false;
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

    fn register_logic_object(&mut self, stable_id: u64) -> bool {
        let is_anim = self.substrate.anims.contains_key(stable_id);
        let already_member = if is_anim {
            self.substrate
                .anims
                .get(stable_id)
                .is_some_and(|anim| anim.in_logic_vector)
        } else {
            self.substrate
                .entities
                .get(stable_id)
                .is_some_and(|entity| entity.in_logic_vector)
        };
        if already_member {
            return true;
        }
        if (!is_anim && !self.substrate.entities.contains(stable_id))
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
        } else if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.in_logic_vector = true;
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::LogicMembershipSet);
        true
    }

    fn unregister_logic_object(&mut self, stable_id: u64) -> bool {
        let is_anim = self.substrate.anims.contains_key(stable_id);
        let flagged = if is_anim {
            self.substrate
                .anims
                .get(stable_id)
                .is_some_and(|anim| anim.in_logic_vector)
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

    /// Transitional app-owned animated-death bridge.  This deliberately changes
    /// only LogicVector membership and must disappear with Mission/Foot authority.
    pub(crate) fn legacy_unregister_logic_only_for_app_death(&mut self, stable_id: u64) {
        let _ = self.unregister_logic_object(stable_id);
    }

    /// Existing Rust owner-count mutation with an explicit exactly-once guard.
    pub(crate) fn release_owned_count_once(&mut self, stable_id: u64) {
        let Some((owner, category, already_released)) = self
            .substrate
            .entities
            .get(stable_id)
            .map(|entity| (entity.owner, entity.category, entity.owned_count_released))
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

    /// ObjectClass::UnInit represented ordering.  Physical removal is deferred.
    pub(crate) fn uninit(&mut self, stable_id: u64) {
        let Some(object_alive) = self
            .substrate
            .entities
            .get(stable_id)
            .map(|entity| entity.lifecycle.object_alive)
        else {
            return;
        };

        if object_alive {
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

            let _ = self.techno_limbo(stable_id);
            if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                entity.lifecycle.object_alive = false;
                entity.dying = true;
            }
            #[cfg(test)]
            self.trace_lifecycle_for_test(LifecycleTestEvent::UninitAliveCleared { stable_id });
        }

        // Native append has no duplicate suppression.  The drain collapses all
        // occurrences when this dead object becomes the selected ready entry.
        self.substrate.pending_delete.push(stable_id);
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::PendingDeleteQueued { stable_id });
    }

    pub(crate) fn despawn_entity(&mut self, stable_id: u64) {
        self.uninit(stable_id);
    }

    fn pending_object_is_ready(&self, stable_id: u64) -> bool {
        if let Some(entity) = self.substrate.entities.get(stable_id) {
            return !entity.lifecycle.object_alive;
        }
        if let Some(anim) = self.substrate.anims.get(stable_id) {
            return anim.runtime.inactive;
        }
        true
    }

    fn finalize_and_remove_common(&mut self, stable_id: u64) {
        let entity = self.substrate.entities.remove(stable_id);
        let anim = self.substrate.anims.remove(stable_id);
        debug_assert!(
            !(entity.is_some() && anim.is_some()),
            "object id {stable_id} was removed from both stores"
        );
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
    }

    /// Test compatibility only.  Production has one ordinary tail drain.
    #[cfg(test)]
    pub(crate) fn flush_pending_delete(&mut self) {
        self.process_pending_delete();
    }
}
