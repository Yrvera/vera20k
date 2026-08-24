//! Ordered ObjectClass-style lifecycle authority.
//!
//! Owns the independent Reveal, Conceal/Limbo, UnInit, LogicVector membership,
//! and pending-delete transitions.  Upper-layer work is emitted as ordered data;
//! this module never depends on render, UI, sidebar, audio, or net.

use std::collections::BTreeSet;

use crate::map::entities::EntityCategory;
use crate::sim::cell_rect::{CellRect, canonical_cell_coord, scan_cell_rect};
use crate::sim::combat::TargetKind;
use crate::sim::components::NavTargetRef;
use crate::sim::intern::InternedId;
use crate::sim::lifecycle_request::LifecycleRequest;
use crate::sim::map::bridge_topology::BRIDGE_DECK_HEIGHT_LEPTONS;
use crate::sim::occupancy::{
    BUILDING_OCCUPATION_BIT, CellListInsertion, OBJECT_OCCUPATION_BIT, VEHICLE_OCCUPATION_BIT,
    air_spatial_bucket_index, air_spatial_tracks_entity, cell_list_layer_for_entity,
    entity_occupancy_cells, infantry_raw_occupation_mask,
};
use crate::sim::passenger::PassengerRole;
use crate::sim::projectile::ProjectileTarget;
use crate::rules::ruleset::RuleSet;
use crate::util::fixed_math::SimFixed;
use crate::util::lepton::{LEPTONS_PER_LEVEL, ground_height_leptons};

use super::Simulation;
use super::substrate::ObjectKind;

/// Borrowed map authority carried through one synchronous ObjectClass UnInit
/// tree. Ordinary entry points use the Simulation-owned terrain; combat uses
/// this context while that same terrain is staged outside `Simulation`.
#[derive(Clone, Copy, Default)]
pub(crate) struct UninitContext<'a> {
    terrain: Option<&'a crate::map::resolved_terrain::ResolvedTerrainGrid>,
    rules: Option<&'a RuleSet>,
}

impl<'a> UninitContext<'a> {
    pub(crate) const fn with_terrain(
        terrain: Option<&'a crate::map::resolved_terrain::ResolvedTerrainGrid>,
    ) -> Self {
        Self {
            terrain,
            rules: None,
        }
    }

    pub(crate) fn with_terrain_and_rules(
        terrain: Option<&'a crate::map::resolved_terrain::ResolvedTerrainGrid>,
        rules: &'a RuleSet,
    ) -> Self {
        let mut context = Self::with_terrain(terrain);
        context.rules = Some(rules);
        context
    }

    pub(crate) const fn with_rules(rules: &'a RuleSet) -> Self {
        Self {
            terrain: None,
            rules: Some(rules),
        }
    }

    pub(crate) const fn terrain(
        self,
    ) -> Option<&'a crate::map::resolved_terrain::ResolvedTerrainGrid> {
        self.terrain
    }

    pub(crate) const fn rules(self) -> Option<&'a RuleSet> {
        self.rules
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlacementEvidence {
    RejectedEarly,
    MarkFailed,
    MarkSucceeded,
    /// Run the modeled Mark(PUT) transaction and consume its result. Production
    /// Unit Unlimbo uses this after exact-zero CanEnter admission instead of
    /// asserting or caller-hardcoding Mark success.
    EvaluateMark,
}

fn building_base_reservation_rect(rx: u16, ry: u16, foundation: &str, spacing: i32) -> CellRect {
    let (width, height) = crate::rules::foundation::foundation_dimensions(foundation);
    CellRect::new(
        i32::from(rx).wrapping_sub(spacing),
        i32::from(ry).wrapping_sub(spacing),
        i32::from(width).wrapping_add(spacing.wrapping_mul(2)),
        i32::from(height).wrapping_add(spacing.wrapping_mul(2)),
    )
}

/// The "no valid cell here" sentinel `BulletClass::PointerExpired` compares its
/// truncated target cell against, at `0x0046856E` and `0x0046857C`. Both
/// `DAT_0089DDF0`/`DAT_0089DDF2` words read zero in the image, and the only
/// writer in the program — the four-instruction routine at `0x00466270` — zeroes
/// them, so the sentinel is the cell (0, 0) rather than a general off-map test.
pub(crate) const NULL_TARGET_CELL_SENTINEL: (u16, u16) = (0, 0);

/// Cell selected after the represented ObjectClass virtual `GetCoords` result
/// is truncated from world leptons. BuildingClass shifts its stored NW anchor
/// to the geometric foundation center before that truncation.
///
/// The `None` arm is unreachable at retail map sizes rather than by
/// construction. `position.rx`/`ry` are `u16` and the in-cell offsets stay in
/// `0..256`, but the Structure arm adds `(width - 1) * 128` leptons before
/// truncating, so a multi-cell structure at the very top of the `u16` cell range
/// would overflow and return `None`. No retail map approaches that bound.
/// Native's own guard against a bad coordinate is the (0, 0) sentinel, checked
/// at the callback instead, so this arm is not the sentinel's analogue.
fn object_get_coords_cell(entity: &crate::sim::game_entity::GameEntity) -> Option<(u16, u16)> {
    let mut world_x = i32::from(entity.position.rx)
        .wrapping_mul(crate::sim::cell_kernel::LEPTONS_PER_CELL)
        .wrapping_add(entity.position.sub_x.to_num::<i32>());
    let mut world_y = i32::from(entity.position.ry)
        .wrapping_mul(crate::sim::cell_kernel::LEPTONS_PER_CELL)
        .wrapping_add(entity.position.sub_y.to_num::<i32>());
    if entity.category == EntityCategory::Structure {
        let (width, height) = crate::rules::foundation::foundation_dimensions(&entity.foundation);
        world_x = world_x.wrapping_add(i32::from(width.saturating_sub(1)).wrapping_mul(128));
        world_y = world_y.wrapping_add(i32::from(height.saturating_sub(1)).wrapping_mul(128));
    }
    Some((
        u16::try_from(crate::sim::cell_kernel::world_to_cell_trunc(world_x)).ok()?,
        u16::try_from(crate::sim::cell_kernel::world_to_cell_trunc(world_y)).ok()?,
    ))
}

pub(super) fn building_base_reservation_repair_rect(
    rx: u16,
    ry: u16,
    foundation_width: u16,
    foundation_height: u16,
    spacing: i32,
) -> CellRect {
    let twice_spacing = spacing.wrapping_mul(2);
    let five_times_spacing = spacing.wrapping_mul(5);
    CellRect::new(
        i32::from(rx).wrapping_sub(twice_spacing),
        i32::from(ry).wrapping_sub(twice_spacing),
        i32::from(foundation_width).wrapping_add(five_times_spacing),
        i32::from(foundation_height).wrapping_add(five_times_spacing),
    )
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
    HiddenOccupationEntered,
    BaseReservationMarked,
    RawOccupationMarked,
    CellMarked,
    RevealDisplayBoundary,
    LogicAppended,
    LogicMembershipSet,
    ConcealDeselected,
    ConcealDestroyNotifyBoundary {
        stable_id: u64,
        object_alive: bool,
        cell_marked: bool,
        resolvable: bool,
    },
    ConcealAlreadyLimboReturn {
        stable_id: u64,
        object_alive: bool,
        resolvable: bool,
    },
    BaseReservationCleared,
    RawOccupationListUnlinked,
    HiddenOccupationExited,
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
        resolvable: bool,
    },
    UninitRemovalListenerVisited {
        expired_id: u64,
        listener_id: u64,
        target_alive: bool,
        target_in_limbo: bool,
    },
    ProjectilePointerExpiredVisited {
        expired_id: u64,
        projectile_id: u64,
        expired_resolvable: bool,
        projectile_resolvable: bool,
        source_id: u64,
        target: ProjectileTarget,
    },
    WaveDamageReceiverSelected {
        wave_id: u64,
        target_id: u64,
    },
    UninitAliveCleared {
        stable_id: u64,
    },
    PostMortemKillBookkeeping {
        stable_id: u64,
    },
    PostMortemRadioBreakCompleted {
        stable_id: u64,
    },
    PostMortemDeselected {
        stable_id: u64,
    },
    PostMortemDestroyNotifyBoundary {
        stable_id: u64,
    },
    /// One visited listener of the live-detach targeting sweep, in the order
    /// the sweep visited it. Recorded for every listener that was pointed at
    /// the detaching object, so a test can pin the descending walk.
    DetachTargetingSweepVisited {
        detach_id: u64,
        listener_id: u64,
        restored: bool,
        target_cleared: bool,
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

    #[cfg(test)]
    pub(crate) fn lifecycle_test_events_for_test(&self) -> &[LifecycleTestEvent] {
        &self.lifecycle_test_events
    }

    #[cfg(test)]
    pub(crate) fn clear_lifecycle_test_events_for_test(&mut self) {
        self.lifecycle_test_events.clear();
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

    fn raw_occupation_cell_facts(&self, position: RevealPosition) -> (i32, i32, bool) {
        let Some(terrain_cell) = self
            .resolved_terrain
            .as_ref()
            .and_then(|terrain| terrain.cell(position.rx, position.ry))
        else {
            return (0, 0, false);
        };
        let ground_level = i32::from(terrain_cell.level as i8);
        let world_x = i32::from(position.rx)
            .wrapping_mul(crate::sim::cell_kernel::LEPTONS_PER_CELL)
            .wrapping_add(position.sub_x.to_num::<i32>());
        let world_y = i32::from(position.ry)
            .wrapping_mul(crate::sim::cell_kernel::LEPTONS_PER_CELL)
            .wrapping_add(position.sub_y.to_num::<i32>());
        let ground_z = ground_height_leptons(
            terrain_cell.level,
            terrain_cell.slope_type,
            world_x,
            world_y,
        )
        .unwrap_or_else(|_| {
            i32::from(terrain_cell.level as i8).wrapping_mul(LEPTONS_PER_LEVEL as i32)
        });
        let live_structural_bridge = terrain_cell.bridge_facts.has_structural_bridge()
            && self
                .bridge_state
                .as_ref()
                .is_some_and(|state| state.is_bridge_walkable(position.rx, position.ry));
        (ground_level, ground_z, live_structural_bridge)
    }

    /// gamemd-derived: active YR `ObjectClass__Mark_Put @ 0x005F60A0` and
    /// `ObjectClass__Mark_Remove @ 0x005F6120` compare the signed absolute
    /// Object coordinate Z with exact ground Z plus the 416-lepton deck height.
    fn raw_occupation_reaches_deck(
        position: RevealPosition,
        exact_z_leptons: Option<i32>,
        ground_level: i32,
        ground_z: i32,
    ) -> bool {
        exact_z_leptons.map_or_else(
            || i32::from(position.z as i8) >= ground_level.wrapping_add(4),
            |object_z| object_z >= ground_z.wrapping_add(BRIDGE_DECK_HEIGHT_LEPTONS),
        )
    }

    fn mark_common_raw_occupation(
        &mut self,
        stable_id: u64,
        category: EntityCategory,
        cells: &[(u16, u16)],
        position: RevealPosition,
        exact_z_leptons: Option<i32>,
    ) -> bool {
        match category {
            EntityCategory::Unit => {
                let (ground_level, ground_z, live_structural_bridge) =
                    self.raw_occupation_cell_facts(position);
                if Self::raw_occupation_reaches_deck(
                    position,
                    exact_z_leptons,
                    ground_level,
                    ground_z,
                ) && live_structural_bridge
                {
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
                let (ground_level, ground_z, live_structural_bridge) =
                    self.raw_occupation_cell_facts(position);
                let mask = infantry_raw_occupation_mask(position.sub_x, position.sub_y);
                // Native: `InfantryClass::MarkCellOccupancy` @ `0x005217C0`
                // selects the deck only at/above the bridge plane and only
                // while `CellClass+0x140 & 0x100` holds. (`0x00743FC0` is
                // inside `UnitClass`, not `InfantryClass`.)
                if Self::raw_occupation_reaches_deck(
                    position,
                    exact_z_leptons,
                    ground_level,
                    ground_z,
                ) && live_structural_bridge
                {
                    self.substrate.raw_cell_occupation.mark_deck_infantry(
                        position.rx,
                        position.ry,
                        mask,
                        stable_id,
                    );
                } else {
                    self.substrate.raw_cell_occupation.mark_ground_infantry(
                        position.rx,
                        position.ry,
                        mask,
                        stable_id,
                    );
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
                let (ground_level, ground_z, live_structural_bridge) =
                    self.raw_occupation_cell_facts(position);
                if Self::raw_occupation_reaches_deck(
                    position,
                    exact_z_leptons,
                    ground_level,
                    ground_z,
                ) && live_structural_bridge
                {
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
        _stable_id: u64,
        category: EntityCategory,
        cells: &[(u16, u16)],
        position: RevealPosition,
        exact_z_leptons: Option<i32>,
    ) -> bool {
        match category {
            EntityCategory::Unit => {
                let (ground_level, ground_z, _) = self.raw_occupation_cell_facts(position);
                if Self::raw_occupation_reaches_deck(
                    position,
                    exact_z_leptons,
                    ground_level,
                    ground_z,
                ) {
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
                let (ground_level, ground_z, live_structural_bridge) =
                    self.raw_occupation_cell_facts(position);
                if Self::raw_occupation_reaches_deck(
                    position,
                    exact_z_leptons,
                    ground_level,
                    ground_z,
                ) && live_structural_bridge
                {
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
            EntityCategory::Infantry => {
                let (ground_level, ground_z, _) = self.raw_occupation_cell_facts(position);
                let mask = infantry_raw_occupation_mask(position.sub_x, position.sub_y);
                // Native: InfantryClass::Unmark (+0x744170) picks its plane from
                // height alone, retaining the proven mark/unmark bridge-bit asymmetry.
                if Self::raw_occupation_reaches_deck(
                    position,
                    exact_z_leptons,
                    ground_level,
                    ground_z,
                ) {
                    self.substrate.raw_cell_occupation.clear_deck_infantry(
                        position.rx,
                        position.ry,
                        mask,
                    );
                } else {
                    self.substrate.raw_cell_occupation.clear_ground_infantry(
                        position.rx,
                        position.ry,
                        mask,
                    );
                }
                true
            }
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
            entity.position.exact_z_leptons = None;
            entity.position.sub_x = request.position.sub_x;
            entity.position.sub_y = request.position.sub_y;
        }
        // `TechnoClass::Unlimbo @ 0x006F6CFE` establishes the canonical
        // TechnoClass+0x3D5 byte from mode-one MapClass membership. Headless
        // fixtures have no MapClass authority, so they retain the constructor
        // default and their consumers explicitly leave the byte unenforced.
        self.establish_entity_playfield_membership_on_unlimbo(stable_id);
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

        if !self.mark_entity_put(stable_id) {
            if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                entity.lifecycle.in_limbo = true;
            }
            return RevealOutcome::Failed(RevealFailure::MarkFailed);
        }
        if let Some(entity) = self.substrate.entities.get_mut(stable_id)
            && entity.spotlight_capable
            && entity.category == crate::map::entities::EntityCategory::Structure
            && entity.building_light.is_none()
        {
            // `BuildingClass::Unlimbo @ 0x00441187` constructs after placement succeeds.
            entity.building_light = Some(crate::sim::game_entity::BuildingLightRuntime {
                behavior: 1,
                target_id: None,
            });
        }
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
        self.refresh_waypoint_edge_from_committed_structure(stable_id);
        self.mark_building_base_reservation(stable_id);
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

    /// Refresh the owning house's navigation edge from a live, map-committed
    /// structure. Launch base-center authority remains the assigned start cell.
    pub(crate) fn refresh_waypoint_edge_from_committed_structure(&mut self, stable_id: u64) {
        let Some(bounds) = self.playfield_bounds else {
            return;
        };
        let Some((owner, anchor)) = self.substrate.entities.get(stable_id).and_then(|entity| {
            (entity.category == EntityCategory::Structure
                && entity.determines_waypoint_edge
                && entity.lifecycle.object_alive
                && !entity.lifecycle.in_limbo
                && entity.lifecycle.cell_marked)
                .then_some((entity.owner, (entity.position.rx, entity.position.ry)))
        }) else {
            return;
        };
        let edge = crate::sim::house_state::determine_waypoint_edge(anchor, bounds);
        if let Some(house) = self.houses.get_mut(&owner) {
            house.waypoint_edge = edge;
        }
    }

    fn mark_entity_put(&mut self, stable_id: u64) -> bool {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return false;
        };
        if entity.lifecycle.cell_marked {
            return false;
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
        let foundation = entity.foundation.clone();
        let hidden_profile = entity.building_hidden_occupancy;
        let current_cell = (entity.position.rx, entity.position.ry);
        let raw_position = RevealPosition {
            rx: entity.position.rx,
            ry: entity.position.ry,
            z: entity.position.z,
            sub_x: entity.position.sub_x,
            sub_y: entity.position.sub_y,
        };
        let exact_z_leptons = entity.position.exact_z_leptons;
        let inside_transport = entity.passenger_role.is_inside_transport();
        let air_spatial_bucket =
            (!inside_transport && air_spatial_tracks_entity(entity)).then(|| {
                air_spatial_bucket_index(
                    entity.position.rx,
                    entity.position.ry,
                    self.session.map_width,
                    self.session.map_height,
                )
            });
        let order = self.substrate.next_occupancy_enter_order.next();

        if category == EntityCategory::Structure {
            let (width, height) = crate::rules::foundation::foundation_dimensions(&foundation);
            let mut intersections = Vec::with_capacity(usize::from(width) * usize::from(height));
            for dy in 0..height {
                for dx in 0..width {
                    let Some(rx) = current_cell.0.checked_add(dx) else {
                        continue;
                    };
                    let Some(ry) = current_cell.1.checked_add(dy) else {
                        continue;
                    };
                    intersections.push((rx, ry));
                }
            }
            if let Some(smudge_grid) = self.smudge_grid.as_mut() {
                smudge_grid.clear_intersecting_footprints(&intersections);
            }
            self.flush_smudge_dirty();
        }

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
                    if category == EntityCategory::Structure
                        && layer == crate::sim::movement::locomotor::MovementLayer::Ground
                        && hidden_profile.is_some_and(|profile| {
                            self.substrate.hidden_occupation.enter_building(
                                current_cell,
                                &foundation,
                                profile,
                                Some((self.session.map_width, self.session.map_height)),
                            )
                        })
                    {
                        #[cfg(test)]
                        self.trace_lifecycle_for_test(LifecycleTestEvent::HiddenOccupationEntered);
                    }
                    if self.mark_common_raw_occupation(
                        stable_id,
                        category,
                        &cells,
                        raw_position,
                        exact_z_leptons,
                    ) {
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
            match air_spatial_bucket {
                Some(bucket) if entity.air_spatial_bucket != Some(bucket) => {
                    entity.air_spatial_bucket = Some(bucket);
                    entity.air_spatial_enter_order = order;
                }
                Some(_) => {}
                None => {
                    entity.air_spatial_bucket = None;
                    entity.air_spatial_enter_order = 0;
                }
            }
            entity.lifecycle.cell_marked = true;
            if let Some(drive) = entity.drive_locomotion.as_mut() {
                drive.current_occupation_cleared = false;
            }
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::CellMarked);
        self.substrate
            .entities
            .get(stable_id)
            .is_some_and(|entity| entity.lifecycle.cell_marked)
    }

    pub(crate) fn base_reservation_house_index(&self, owner: InternedId) -> Option<i32> {
        // Active YR Full_Init constructs the complete HouseClass array before
        // Terrain/Techno map sections reveal objects. Scenario loading preserves
        // that order, so Reveal writes the final house-index bit immediately.
        if self.session.house_order.is_empty() {
            return None;
        }
        let index = self
            .session
            .house_order
            .iter()
            .position(|registered| *registered == owner)
            .and_then(|index| i32::try_from(index).ok());
        debug_assert!(
            index.is_some(),
            "base-reservation owner must be present in ScenarioSession.house_order"
        );
        index
    }

    fn mark_building_base_reservation(&mut self, stable_id: u64) -> bool {
        let Some((owner, rect)) = self.base_reservation_writer(stable_id) else {
            return false;
        };
        let Some(house_index) = self.base_reservation_house_index(owner) else {
            return false;
        };
        self.substrate.base_reservations.reserve_rect(
            self.resolved_terrain.as_ref(),
            rect,
            house_index,
        );
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::BaseReservationMarked);
        true
    }

    fn base_reservation_writer(&self, stable_id: u64) -> Option<(InternedId, CellRect)> {
        let entity = self.substrate.entities.get(stable_id)?;
        let spacing = entity.base_reservation_spacing?;
        (entity.category == EntityCategory::Structure
            && entity.lifecycle.object_alive
            && !entity.lifecycle.in_limbo
            && entity.lifecycle.cell_marked
            && cell_list_layer_for_entity(entity)
                == Some(crate::sim::movement::locomotor::MovementLayer::Ground))
        .then(|| {
            (
                entity.owner,
                building_base_reservation_rect(
                    entity.position.rx,
                    entity.position.ry,
                    &entity.foundation,
                    spacing,
                ),
            )
        })
    }

    fn clear_building_base_reservation_and_repair(&mut self, stable_id: u64) -> bool {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return false;
        };
        let Some(spacing) = entity.base_reservation_spacing else {
            return false;
        };
        if entity.category != EntityCategory::Structure
            || !entity.lifecycle.cell_marked
            || cell_list_layer_for_entity(entity)
                != Some(crate::sim::movement::locomotor::MovementLayer::Ground)
        {
            return false;
        }
        let owner = entity.owner;
        let rect = building_base_reservation_rect(
            entity.position.rx,
            entity.position.ry,
            &entity.foundation,
            spacing,
        );
        let (foundation_width, foundation_height) =
            crate::rules::foundation::foundation_dimensions(&entity.foundation);
        let repair_rect = building_base_reservation_repair_rect(
            entity.position.rx,
            entity.position.ry,
            foundation_width,
            foundation_height,
            spacing,
        );
        let Some(house_index) = self.base_reservation_house_index(owner) else {
            return false;
        };

        self.substrate.base_reservations.clear_rect(
            self.resolved_terrain.as_ref(),
            rect,
            house_index,
        );
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::BaseReservationCleared);

        // The native repair scan happens while this building is still linked in
        // the ground object lists. Gather stable identities first, then re-run
        // each neighbor's own exact writer without holding a grid borrow.
        let mut repair_ids = BTreeSet::new();
        scan_cell_rect(repair_rect, |x, y| {
            if let Some((rx, ry)) = canonical_cell_coord(x, y)
                && self
                    .resolved_terrain
                    .as_ref()
                    .is_none_or(|terrain| terrain.cell(rx, ry).is_some())
                && let Some(cell) = self.substrate.occupancy.get(rx, ry)
            {
                repair_ids.extend(
                    cell.iter_layer(crate::sim::movement::locomotor::MovementLayer::Ground)
                        .map(|occupant| occupant.entity_id),
                );
            }
            true
        });
        repair_ids.remove(&stable_id);
        for neighbor_id in repair_ids {
            self.mark_building_base_reservation(neighbor_id);
        }
        true
    }

    fn unmark_entity_remove(&mut self, stable_id: u64) -> bool {
        self.unmark_entity_remove_impl(stable_id, true)
    }

    fn unmark_entity_remove_impl(&mut self, stable_id: u64, clear_air_spatial: bool) -> bool {
        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return false;
        };
        if !entity.lifecycle.cell_marked {
            return false;
        }
        let cells = entity_occupancy_cells(entity);
        let layer = cell_list_layer_for_entity(entity);
        let category = entity.category;
        let foundation = entity.foundation.clone();
        let hidden_profile = entity.building_hidden_occupancy;
        let current_cell = (entity.position.rx, entity.position.ry);
        let raw_position = RevealPosition {
            rx: entity.position.rx,
            ry: entity.position.ry,
            z: entity.position.z,
            sub_x: entity.position.sub_x,
            sub_y: entity.position.sub_y,
        };
        let exact_z_leptons = entity.position.exact_z_leptons;
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
                if category == EntityCategory::Structure
                    && layer == crate::sim::movement::locomotor::MovementLayer::Ground
                    && hidden_profile.is_some_and(|profile| {
                        self.substrate.hidden_occupation.exit_building(
                            current_cell,
                            &foundation,
                            profile,
                            Some((self.session.map_width, self.session.map_height)),
                        )
                    })
                {
                    #[cfg(test)]
                    self.trace_lifecycle_for_test(LifecycleTestEvent::HiddenOccupationExited);
                }
                if self.clear_common_raw_occupation(
                    stable_id,
                    category,
                    &cells,
                    raw_position,
                    exact_z_leptons,
                ) {
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
            if clear_air_spatial {
                entity.air_spatial_bucket = None;
                entity.air_spatial_enter_order = 0;
            }
            if let Some(drive) = entity.drive_locomotion.as_mut() {
                drive.current_occupation_cleared = true;
            }
        }
        true
    }

    /// Mirror the native air-vector move producer: retain vector position while
    /// the object stays in one bucket, otherwise remove from the old vector and
    /// append to the destination vector's tail.
    fn sync_air_spatial_membership(&mut self, stable_id: u64) {
        let desired_bucket = self.substrate.entities.get(stable_id).and_then(|entity| {
            (entity.lifecycle.object_alive
                && !entity.lifecycle.in_limbo
                && entity.lifecycle.cell_marked
                && !entity.passenger_role.is_inside_transport()
                && air_spatial_tracks_entity(entity))
            .then(|| {
                air_spatial_bucket_index(
                    entity.position.rx,
                    entity.position.ry,
                    self.session.map_width,
                    self.session.map_height,
                )
            })
        });
        let current_bucket = self
            .substrate
            .entities
            .get(stable_id)
            .and_then(|entity| entity.air_spatial_bucket);
        if current_bucket == desired_bucket {
            return;
        }
        let enter_order = desired_bucket
            .map(|_| self.substrate.next_occupancy_enter_order.next())
            .unwrap_or(0);
        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.air_spatial_bucket = desired_bucket;
            entity.air_spatial_enter_order = enter_order;
        }
    }

    /// Test/fixture helper retained at the transaction boundary.  It is
    /// idempotent and updates the authoritative `cell_marked` fact.
    pub(crate) fn add_entity_occupancy(&mut self, stable_id: u64) {
        let _ = self.mark_entity_put(stable_id);
    }

    /// Existing movement and fixture boundary; common lifecycle code calls the
    /// private unmark transaction instead.
    pub(crate) fn remove_entity_occupancy(&mut self, stable_id: u64) {
        self.unmark_entity_remove(stable_id);
    }

    /// gamemd-derived: active YR `FlyLocomotionClass__Process @ 0x004CD600`
    /// reaches `FootClass__Set_Height_On_Bridge @ 0x005F5FA0` through the
    /// Object vtable. It commits signed absolute Object Z from exact ground,
    /// the `OnBridge` deck offset, and Fly altitude before the final Mark(PUT).
    fn sync_fly_object_height(&mut self, stable_id: u64) {
        use crate::rules::locomotor_type::LocomotorKind;
        use crate::sim::movement::locomotor::MovementLayer;

        let Some(entity) = self.substrate.entities.get(stable_id) else {
            return;
        };
        let Some(locomotor) = entity.locomotor.as_ref() else {
            return;
        };
        if locomotor.layer != MovementLayer::Air || locomotor.kind != LocomotorKind::Fly {
            return;
        }

        let world_x = i32::from(entity.position.rx)
            .wrapping_mul(crate::sim::cell_kernel::LEPTONS_PER_CELL)
            .wrapping_add(entity.position.sub_x.to_num::<i32>());
        let world_y = i32::from(entity.position.ry)
            .wrapping_mul(crate::sim::cell_kernel::LEPTONS_PER_CELL)
            .wrapping_add(entity.position.sub_y.to_num::<i32>());
        // CellClass__GetGroundHeight @ 0x00578080 resolves a missing/out-of-map
        // lookup through the zero-height dummy CellClass, not Object's coarse Z.
        let terrain_cell = self
            .resolved_terrain
            .as_ref()
            .and_then(|terrain| terrain.cell(entity.position.rx, entity.position.ry));
        let ground_z = if let Some(cell) = terrain_cell {
            let Ok(ground_z) = ground_height_leptons(cell.level, cell.slope_type, world_x, world_y)
            else {
                return;
            };
            ground_z
        } else {
            0
        };
        let surface_z = if entity.on_bridge {
            ground_z.wrapping_add(BRIDGE_DECK_HEIGHT_LEPTONS)
        } else {
            ground_z
        };
        let exact_z = surface_z.wrapping_add(locomotor.altitude.to_num::<i32>());

        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.position.exact_z_leptons = Some(exact_z);
        }
    }

    /// Run one production air-process visit with the active Fly
    /// remove-before/process/add-after cell-list transaction around it.
    pub(crate) fn tick_air_movement_with_cell_lists_one(
        &mut self,
        stable_id: u64,
    ) -> crate::sim::movement::air_movement::AirMovementTickStats {
        use crate::rules::locomotor_type::LocomotorKind;
        use crate::sim::movement::locomotor::MovementLayer;

        let transact_fly = self
            .substrate
            .entities
            .get(stable_id)
            .is_some_and(|entity| {
                entity.category == EntityCategory::Aircraft
                    && entity.lifecycle.object_alive
                    && !entity.lifecycle.in_limbo
                    && entity.locomotor.as_ref().is_some_and(|locomotor| {
                        locomotor.kind == LocomotorKind::Fly
                            && locomotor.layer == MovementLayer::Air
                    })
            });
        if transact_fly {
            self.unmark_entity_remove_impl(stable_id, false);
        }

        let stats = crate::sim::movement::air_movement::tick_air_movement(
            &mut self.substrate.entities,
            &[stable_id],
            self.session.tick,
        );

        self.sync_fly_object_height(stable_id);

        if transact_fly
            && self
                .substrate
                .entities
                .get(stable_id)
                .is_some_and(|entity| entity.lifecycle.object_alive && !entity.lifecycle.in_limbo)
        {
            self.add_entity_occupancy(stable_id);
        }
        self.sync_air_spatial_membership(stable_id);
        stats
    }

    /// Object-kind classification for the LogicVector dispatch (F13). Probes
    /// the stores in the exact pre-consolidation order (anims → particle
    /// systems → terrain → projectiles → waves → entities) and returns `None`
    /// when the id is represented nowhere. Object IDs are unique across
    /// stores, so first-match equals only-match.
    pub(crate) fn classify_object(&self, stable_id: u64) -> Option<ObjectKind> {
        if self.substrate.anims.contains_key(stable_id) {
            Some(ObjectKind::Anim)
        } else if self.substrate.particle_systems.contains_key(stable_id) {
            Some(ObjectKind::ParticleSystem)
        } else if self.production.terrain_objects.contains_key(&stable_id) {
            Some(ObjectKind::Terrain)
        } else if self.projectiles.get(stable_id).is_some() {
            Some(ObjectKind::Projectile)
        } else if self.waves.get(stable_id).is_some() {
            Some(ObjectKind::Wave)
        } else if self.substrate.entities.contains(stable_id) {
            Some(ObjectKind::Entity)
        } else {
            None
        }
    }

    /// Read the per-object `in_logic_vector` membership flag for a classified
    /// object. The flag lives on the object in its own store; this is the
    /// single dispatch the registration/removal contract reads through.
    fn logic_membership_flag(&self, stable_id: u64, kind: ObjectKind) -> bool {
        match kind {
            ObjectKind::Anim => self
                .substrate
                .anims
                .get(stable_id)
                .is_some_and(|anim| anim.in_logic_vector),
            ObjectKind::ParticleSystem => self
                .substrate
                .particle_systems
                .get(stable_id)
                .is_some_and(|system| system.in_logic_vector),
            ObjectKind::Terrain => self
                .production
                .terrain_objects
                .get(&stable_id)
                .is_some_and(|terrain| terrain.in_logic_vector),
            ObjectKind::Projectile => self
                .projectiles
                .get(stable_id)
                .is_some_and(|projectile| projectile.in_logic_vector),
            ObjectKind::Wave => self
                .waves
                .get(stable_id)
                .is_some_and(|wave| wave.in_logic_vector),
            ObjectKind::Entity => self
                .substrate
                .entities
                .get(stable_id)
                .is_some_and(|entity| entity.in_logic_vector),
        }
    }

    /// Write the per-object `in_logic_vector` membership flag for a classified
    /// object. The single dispatch the registration/removal contract repairs
    /// the flag through.
    fn set_logic_membership_flag(&mut self, stable_id: u64, kind: ObjectKind, member: bool) {
        match kind {
            ObjectKind::Anim => {
                if let Some(anim) = self.substrate.anims.get_mut(stable_id) {
                    anim.in_logic_vector = member;
                }
            }
            ObjectKind::ParticleSystem => {
                if let Some(system) = self.substrate.particle_systems.get_mut(stable_id) {
                    system.in_logic_vector = member;
                }
            }
            ObjectKind::Terrain => {
                if let Some(terrain) = self.production.terrain_objects.get_mut(&stable_id) {
                    terrain.in_logic_vector = member;
                }
            }
            ObjectKind::Projectile => {
                if let Some(projectile) = self.projectiles.get_mut(stable_id) {
                    projectile.in_logic_vector = member;
                }
            }
            ObjectKind::Wave => {
                if let Some(wave) = self.waves.get_mut(stable_id) {
                    wave.in_logic_vector = member;
                }
            }
            ObjectKind::Entity => {
                if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
                    entity.in_logic_vector = member;
                }
            }
        }
    }

    /// gamemd-derived: active YR `LogicClass__RegisterObject @ 0x0055BAA0`
    /// gates on the object's membership flag, appends at the live tail, then
    /// sets the flag only after insertion succeeds.
    fn register_logic_object(&mut self, stable_id: u64) -> bool {
        let kind = self.classify_object(stable_id);
        if let Some(kind) = kind {
            if self.logic_membership_flag(stable_id, kind) {
                return true;
            }
        }
        let Some(kind) = kind else {
            return false;
        };
        if self.substrate.logic.try_push(stable_id).is_err() {
            return false;
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::LogicAppended);
        self.set_logic_membership_flag(stable_id, kind, true);
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::LogicMembershipSet);
        true
    }

    /// gamemd-derived: active YR `LogicClass__UnregisterObject @ 0x0055BAE0`
    /// gates removal on the object's membership flag, performs the first-match
    /// stable erase, then repairs that flag.
    fn unregister_logic_object(&mut self, stable_id: u64) -> bool {
        let Some(kind) = self.classify_object(stable_id) else {
            return false;
        };
        if !self.logic_membership_flag(stable_id, kind) {
            return false;
        }
        let _ = self.substrate.logic.remove_first(stable_id);
        self.set_logic_membership_flag(stable_id, kind, false);
        true
    }

    /// Test-only access to the exact LogicVector helper ordering. Returns the
    /// dispatch outcome so order/gate tests can assert it directly.
    #[cfg(test)]
    pub(crate) fn register_live_object(&mut self, stable_id: u64) -> bool {
        self.register_logic_object(stable_id)
    }

    /// Test-only access to the exact LogicVector helper ordering. Returns the
    /// dispatch outcome so order/gate tests can assert it directly.
    #[cfg(test)]
    pub(crate) fn unregister_live_object(&mut self, stable_id: u64) -> bool {
        self.unregister_logic_object(stable_id)
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

    pub(crate) fn register_terrain_object(&mut self, stable_id: u64) -> bool {
        self.production
            .terrain_objects
            .get(&stable_id)
            .is_some_and(|terrain| terrain.is_live())
            && self.register_logic_object(stable_id)
    }

    pub(crate) fn register_projectile(&mut self, stable_id: u64) -> bool {
        self.projectiles.get(stable_id).is_some() && self.register_logic_object(stable_id)
    }

    pub(crate) fn register_wave(&mut self, stable_id: u64) -> bool {
        self.waves.get(stable_id).is_some() && self.register_logic_object(stable_id)
    }

    pub(crate) fn unregister_non_entity_object(&mut self, stable_id: u64) -> bool {
        self.unregister_logic_object(stable_id)
    }

    /// Terminal Terrain/Bullet/Wave objects leave Logic immediately but retain
    /// their physical store identity until the common late delete drain.
    ///
    /// gamemd-derived: active YR `DrainDeferredFinalizationQueue @ 0x00725C70`
    /// performs scalar destruction/freeing only after the frame commit.
    pub(crate) fn retire_non_entity_object(&mut self, stable_id: u64) -> bool {
        let represented = matches!(
            self.classify_object(stable_id),
            Some(ObjectKind::Terrain | ObjectKind::Projectile | ObjectKind::Wave)
        );
        if !represented {
            return false;
        }
        let _ = self.unregister_logic_object(stable_id);
        self.substrate.pending_delete.push(stable_id);
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::PendingDeleteQueued { stable_id });
        true
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
    #[cfg(test)]
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

    /// ObjectClass::Conceal represented order. Conceal does not mutate Alive.
    #[cfg(test)]
    pub(crate) fn object_conceal(&mut self, stable_id: u64) -> ConcealOutcome {
        self.object_conceal_with_context(stable_id, UninitContext::default())
    }

    fn object_conceal_with_context(
        &mut self,
        stable_id: u64,
        context: UninitContext<'_>,
    ) -> ConcealOutcome {
        let Some((in_limbo, object_alive)) = self
            .substrate
            .entities
            .get(stable_id)
            .map(|entity| (entity.lifecycle.in_limbo, entity.lifecycle.object_alive))
        else {
            return ConcealOutcome::MissingOrDead;
        };
        #[cfg(not(test))]
        let _ = object_alive;
        // gamemd-derived: `ObjectClass::Conceal @ 0x005F4D30` tests InLimbo
        // at `0x005F4D45` and returns before Destroy on the already-limbo path.
        if in_limbo {
            #[cfg(test)]
            self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealAlreadyLimboReturn {
                stable_id,
                object_alive,
                resolvable: self.substrate.entities.contains(stable_id),
            });
            return ConcealOutcome::AlreadyConcealed;
        }

        if let Some(entity) = self.substrate.entities.get_mut(stable_id) {
            entity.selected = false;
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealDeselected);

        // gamemd-derived: active YR `ObjectClass::Conceal @ 0x005F4D30`
        // enters `ObjectClass::Destroy(1) @ 0x005F5280` after deselection and
        // before `Mark(REMOVE)`, so the expiry broadcast observes the target
        // alive, resolvable, and still cell-marked.
        #[cfg(test)]
        {
            let (object_alive, cell_marked) = self
                .substrate
                .entities
                .get(stable_id)
                .map(|entity| (entity.lifecycle.object_alive, entity.lifecycle.cell_marked))
                .unwrap_or((false, false));
            self.trace_lifecycle_for_test(LifecycleTestEvent::ConcealDestroyNotifyBoundary {
                stable_id,
                object_alive,
                cell_marked,
                resolvable: self.substrate.entities.contains(stable_id),
            });
        }
        self.notify_pointer_expired(stable_id, context);

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
            // `BuildingClass::Limbo` destroys its owned BuildingLight before
            // the remaining building-count/base-node teardown.
            entity.building_light = None;
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
        self.techno_limbo_with_context(stable_id, UninitContext::default())
    }

    pub(crate) fn techno_limbo_with_rules(
        &mut self,
        stable_id: u64,
        rules: &RuleSet,
    ) -> ConcealOutcome {
        self.techno_limbo_with_context(stable_id, UninitContext::with_rules(rules))
    }

    fn techno_limbo_with_context(
        &mut self,
        stable_id: u64,
        context: UninitContext<'_>,
    ) -> ConcealOutcome {
        if !self.substrate.entities.contains(stable_id) {
            return ConcealOutcome::MissingOrDead;
        }
        // FootClass::Limbo @ 0x004DB260 and BuildingClass::Limbo @
        // 0x00445880 remove the exact deposited footprint before conceal.
        if let Some(rules) = context.rules() {
            self.remove_sensor_before_limbo_with_rules(stable_id, rules);
        } else {
            self.remove_sensor_before_limbo(stable_id);
        }
        // Dead and InLimbo are independent native state. TechnoClass::Limbo
        // still reaches ObjectClass::Conceal for a stored dead object; the
        // latter's InLimbo branch alone decides whether Conceal is a no-op.
        self.clear_building_base_reservation_and_repair(stable_id);
        crate::sim::radio::broadcast_break(self, stable_id);
        self.object_conceal_with_context(stable_id, context)
    }

    /// Existing Rust owner-count mutation with an explicit exactly-once guard.
    pub(crate) fn release_owned_count_once(&mut self, stable_id: u64) {
        let Some((owner, category, already_released, destroyed, killed_by, award, dont_score)) =
            self.substrate.entities.get(stable_id).map(|entity| {
                (
                    entity.owner,
                    entity.category,
                    entity.owned_count_released,
                    entity.health.current == 0,
                    entity.killed_by,
                    entity.kill_award_points,
                    entity.dont_score,
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
        if destroyed && !dont_score {
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
    /// A `DontScore=` victim never reaches this recorder at all — its loss is
    /// suppressed alongside its kill and points, matching the single early return
    /// gamemd takes before any of the three.
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
        // is worth no score. `Record_The_Kill @ 0x00702D40` computes the award
        // once — behind `HouseClass::IsAlly @ 0x004F9A90`, asked BY THE KILLER —
        // and feeds the same value to the score add at 0x0070300F and to the
        // veterancy accumulator, so this test must be the one-way one and must
        // match `combat::award_kill_experience`.
        let friendly = crate::map::houses::is_allied_with(
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

    /// ObjectClass's exact-zero callback transaction for an eligible
    /// `CausesDelayKill` building. Active gamemd runs the routed kill callback
    /// and virtual Destroy/reference notification before TechnoClass arms the
    /// timer and restores Alive/Health=1. This deliberately does not call
    /// UnInit, Limbo, release owned counts, or enqueue physical deletion.
    pub(crate) fn postmortem_exact_zero_callbacks(
        &mut self,
        stable_id: u64,
        killer_owner: Option<InternedId>,
        rules: &crate::rules::ruleset::RuleSet,
        context: UninitContext<'_>,
    ) {
        let Some((owner, category, dont_score, type_ref, veterancy)) =
            self.substrate.entities.get(stable_id).map(|target| {
                (
                    target.owner,
                    target.category,
                    target.dont_score,
                    target.type_ref,
                    target.veterancy,
                )
            })
        else {
            return;
        };

        debug_assert_eq!(
            self.substrate
                .entities
                .get(stable_id)
                .map(|target| target.health.current),
            Some(0),
            "PostMortem Object callbacks run at exact zero"
        );
        if !dont_score {
            let award = crate::sim::combat::score_award_for_victim(
                rules.object(self.interner.resolve(type_ref)),
                veterancy,
            );
            self.record_match_kill_and_loss(owner, category, killer_owner, award);
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::PostMortemKillBookkeeping { stable_id });

        // BuildingClass::Destroy broadcasts BREAK before entering the common
        // ObjectClass::Destroy body. The per-building native factory pointer has
        // no Rust representation; eligible stock barrels have none.
        crate::sim::radio::broadcast_break(self, stable_id);
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::PostMortemRadioBreakCompleted {
            stable_id,
        });

        // ObjectClass::Destroy(1) unconditionally deselects before broadcasting
        // pointer expiry. It leaves Logic/occupancy/liveness intact.
        if let Some(target) = self.substrate.entities.get_mut(stable_id) {
            target.selected = false;
        }
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::PostMortemDeselected { stable_id });

        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::PostMortemDestroyNotifyBoundary {
            stable_id,
        });
        self.notify_pointer_expired(stable_id, context);
    }

    pub(crate) fn apply_lifecycle_request(&mut self, request: LifecycleRequest) {
        match request {
            LifecycleRequest::Uninit {
                stable_id,
                reason: _,
            } => self.uninit(stable_id),
        }
    }

    pub(crate) fn apply_lifecycle_request_with_rules(
        &mut self,
        request: LifecycleRequest,
        rules: &RuleSet,
    ) {
        match request {
            LifecycleRequest::Uninit {
                stable_id,
                reason: _,
            } => self.uninit_with_rules(stable_id, rules),
        }
    }

    fn run_represented_uninit_pre_hook(&mut self, stable_id: u64) {
        self.clear_building_damage_fire_slots(stable_id);
        self.release_owned_count_once(stable_id);
        crate::sim::docking::bunker_link::break_links_on_despawn(self, stable_id);
        #[cfg(test)]
        self.trace_lifecycle_for_test(LifecycleTestEvent::UninitClassPre { stable_id });
    }

    fn uninit_carried_passengers(&mut self, carrier_id: u64, context: UninitContext<'_>) {
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
            self.uninit_with_context(passenger_id, context);
        }
    }

    /// TechnoClass fatal-receiver passenger rung. The native death helper
    /// enters only after carried objects have completed their own authoritative
    /// UnInit transactions; the carrier itself remains represented for its
    /// DeathWeapon and category-specific UnInit that follows.
    pub(crate) fn purge_carried_passengers_for_fatal(
        &mut self,
        carrier_id: u64,
        context: UninitContext<'_>,
    ) {
        self.uninit_carried_passengers(carrier_id, context);
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
    ///
    /// gamemd-derived: active YR `ObjectClass` construction/destruction at
    /// `0x005F3900` / `0x005F3B80` maintains the listener roster in object
    /// construction order.
    fn removal_listener_order(&self) -> Vec<u64> {
        let mut listeners = self.substrate.entities.keys_sorted();
        listeners.extend(self.substrate.anims.iter().map(|(&stable_id, _)| stable_id));
        listeners.extend(
            self.substrate
                .particle_systems
                .iter()
                .map(|(&stable_id, _)| stable_id),
        );
        listeners.extend(self.projectiles.iter().map(|(&stable_id, _)| stable_id));
        listeners.sort_unstable();
        debug_assert!(
            listeners.windows(2).all(|pair| pair[0] != pair[1]),
            "object stable ID exists in more than one represented store"
        );
        listeners
    }

    /// The detach-time targeting sweep: release every object currently shooting
    /// at `detach_id`, which is leaving play *while still alive*.
    ///
    /// This is not the pointer-expiry broadcast. The detaching object survives —
    /// it is being sold, changing owner, teleporting, or being detached by area
    /// damage — so nothing else nulls the references pointed at it, and this is
    /// the only route by which a live-but-detached target releases its
    /// attackers. In an ordinary skirmish it fires tens of times per match:
    /// every building sale, every engineer capture, every mind-control or
    /// Psychic Beacon owner change, every Chrono Legionnaire or Chronosphere
    /// teleport.
    ///
    /// Three clauses are NOT copies of the pointer-expiry sweep and are
    /// reproduced verbatim because each is observable:
    ///
    /// 1. **Descending stable-ID iteration.** The native walk runs the global
    ///    techno vector from its last entry down to its first. When two objects
    ///    share the detaching target, the higher ID restores first, so its
    ///    restored mission dispatches first and draws from the scenario RNG
    ///    first. An ascending walk would reorder the global draw sequence.
    /// 2. **Restore runs first, with no suspended-mission pre-check.** The
    ///    expiry sweep asks whether a mission is suspended and clears the target
    ///    before restoring; this one restores unconditionally and clears after.
    /// 3. **The target clear is conditional on the Restore not having replaced
    ///    the target.** A successful Restore re-installs the archived target, and
    ///    the null-out then does not run.
    ///
    /// **ONLY ONE OF SIX NATIVE ENTRY POINTS IS WIRED.** The sweep is reached
    /// from six distinct functions in the original — area damage (six call
    /// sites in one function), building sale, an unnamed pair, the building
    /// occupy-map placement, the owner change, and the teleport locomotor's
    /// state machine. VERA calls it only from the owner change. The earlier
    /// "1 of 3 Restore sites" framing counted CALL sites through the Restore
    /// vtable slot, which is a different and smaller set than the sweep's own
    /// callers, and read more complete than it was. Consequence: an attacker
    /// stays latched onto a target that is being sold or chrono-teleported
    /// until the target is actually removed and the pointer-expiry broadcast
    /// catches it. Frequency: every building sale and every Chrono
    /// Legionnaire / Chronosphere use — tens of times a match — though the
    /// window is short and the blast radius today is limited to objects that
    /// went through a blocked-step Override.
    ///
    /// RESIDUALS, recorded rather than guessed:
    /// - The native suppression clause skips the whole block for a listener
    ///   whose manager sub-object points at the detaching object while that
    ///   object is still alive and active — a mind-control / capture link
    ///   holding its live victim as its target. It exists precisely so that the
    ///   owner change *performed by* mind control does not make the controller
    ///   drop its own new victim. VERA models mind control as a per-entity flag
    ///   with no controller-to-victim link, so the clause cannot be evaluated
    ///   and is omitted. Consequence: once VERA gives mind control a controller
    ///   link, a controller will lose its target here where the original keeps
    ///   it — every control event. Today the arm is unreachable because no
    ///   controller link exists to hold the victim as a target.
    /// - The aircraft-Patrol arm, which clears two patrol-cursor fields on an
    ///   aircraft whose committed mission is Patrol. Neither field is
    ///   represented; the arm is a no-op for every ground object.
    /// - A second native table swept after the techno vector, nulling two
    ///   pointer slots that match the detaching object. Its element class is
    ///   UNKNOWN, so it is not modelled.
    pub(crate) fn stop_all_targeting_on_detach(&mut self, detach_id: u64) {
        let mut listeners = self.substrate.entities.keys_sorted();
        listeners.reverse();

        for listener_id in listeners {
            if !self.listener_targets(listener_id, detach_id) {
                continue;
            }

            let restored = self
                .mission_restore_on_target_detach(listener_id)
                .expect("detach sweep listener was resolved immediately before the Restore");

            let target_cleared = self.listener_targets(listener_id, detach_id);
            if target_cleared {
                self.set_archive_target_represented(listener_id, None)
                    .expect("detach sweep listener remains present for the target clear");
            }

            let _ = restored;
            #[cfg(test)]
            self.trace_lifecycle_for_test(LifecycleTestEvent::DetachTargetingSweepVisited {
                detach_id,
                listener_id,
                restored,
                target_cleared,
            });
        }
    }

    /// Whether `listener_id` currently holds `detach_id` as its shoot-at target.
    ///
    /// Cell targets never match: the native comparison is against an object
    /// pointer, so an object overridden onto a wall cell is invisible to both
    /// detach sweeps.
    fn listener_targets(&self, listener_id: u64, detach_id: u64) -> bool {
        self.substrate
            .entities
            .get(listener_id)
            .is_some_and(|listener| {
                matches!(
                    listener.attack_target.as_ref().map(|target| target.target),
                    Some(TargetKind::Entity(id)) if id == detach_id
                )
            })
    }

    fn notify_entity_pointer_expired(
        &mut self,
        listener_id: u64,
        expired_id: u64,
        // The expiring object's `ObjectClass::GetCoords` cell — the single
        // derivation `BulletClass::PointerExpired @ 0x004684E0` uses for both
        // the entity-hosted and store-hosted missile arms.
        expired_get_coords_cell: Option<(u16, u16)>,
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
            homing.expire_object_target(
                expired_id,
                expired_get_coords_cell,
                expired_is_high_flying,
            );
        }

        if let Some(pending) = listener.pending_c4_detonation.as_mut()
            && pending.source_entity_id == Some(expired_id)
        {
            pending.source_entity_id = None;
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
    ///
    /// gamemd-derived: active YR `DispatchPointerExpiredCleanup @ 0x007258D0`
    /// is called directly by `ObjectClass__UnInit @ 0x005F65F0` and again by
    /// `ObjectClass::Destroy @ 0x005F5280` inside the virtual Conceal path.
    /// The caller's legacy terrain context stays unread: production queries the
    /// Simulation-resident resolved grid below as its MapClass table. The other
    /// listener arms still read the object's liveness, health and mission from
    /// the same destructure.
    fn notify_pointer_expired(&mut self, expired_id: u64, _context: UninitContext<'_>) {
        let Some((
            expired_target_cell,
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
                object_get_coords_cell(expired),
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

        // `BulletClass::PointerExpired @ 0x004684E0` performs the packed
        // `MapClass::Get_CellClass @ 0x005657A0` lookup only for a matching
        // target. Its result pointer is stored at Bullet+0x10C: an allocated
        // slot therefore remains a stable Cell target, while a miss stores the
        // one shared dummy at `0x00ABDC50`. Later `BulletClass::AI @ 0x004666E0`
        // dispatches that live pointer and observes its most recent coord stamp.

        for listener_id in self.removal_listener_order() {
            let is_entity = self.substrate.entities.contains(listener_id);
            let is_anim = self.substrate.anims.contains_key(listener_id);
            let is_particle = self.substrate.particle_systems.contains_key(listener_id);
            let is_projectile = self.projectiles.get(listener_id).is_some();
            if !is_entity && !is_anim && !is_particle && !is_projectile {
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
                    expired_target_cell,
                    expired_is_high_flying,
                    expired_object_alive,
                    expired_health,
                    expired_is_selling,
                );
                // `TechnoClass::PointerExpired` forwards to the listener's
                // SpawnManager (`0x00707A6F`). This is the only mechanism that
                // drops a destroyed wing target, so without it a Carrier keeps
                // sending its Hornets at a corpse.
                crate::sim::spawn_manager::notify_pointer_expired(self, listener_id, expired_id);
                if let Some(manager) = self
                    .substrate
                    .entities
                    .get_mut(listener_id)
                    .and_then(|entity| entity.capture_manager.as_mut())
                {
                    manager.pointer_expired(expired_id);
                }
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
                    // vtable `+0xF8`, the same mark the lifetime and
                    // spawn-cutoff paths set.
                    system.done_spawning = true;
                }
            } else if is_projectile {
                let target_matches = self
                    .projectiles
                    .get(listener_id)
                    .is_some_and(|projectile| {
                        projectile.target == ProjectileTarget::Entity(expired_id)
                    });
                let projectile_replacement_target = if !target_matches
                    || expired_is_high_flying
                    || expired_target_cell.is_none()
                    || expired_target_cell == Some(NULL_TARGET_CELL_SENTINEL)
                {
                    ProjectileTarget::None
                } else {
                    let (rx, ry) = expired_target_cell.expect("checked target cell");
                    match self.resolved_terrain.as_ref() {
                        Some(terrain) => match crate::sim::cell_rect::get_cellclass_fallback(
                            Some(terrain),
                            i32::from(rx),
                            i32::from(ry),
                        ) {
                            crate::sim::cell_rect::CellRef::Real(_) => {
                                ProjectileTarget::Cell { rx, ry }
                            }
                            crate::sim::cell_rect::CellRef::Dummy { .. } => {
                                ProjectileTarget::DummyCell
                            }
                        },
                        // Terrainless synthetic fixtures keep their historical
                        // stable-cell fallback. Production is terrain-backed.
                        None => ProjectileTarget::Cell { rx, ry },
                    }
                };
                let present = self.projectiles.pointer_expired(
                    listener_id,
                    expired_id,
                    projectile_replacement_target,
                );
                debug_assert!(present);
                #[cfg(test)]
                if let Some((source_id, target)) = self
                    .projectiles
                    .get(listener_id)
                    .map(|projectile| (projectile.source_id, projectile.target))
                {
                    self.trace_lifecycle_for_test(
                        LifecycleTestEvent::ProjectilePointerExpiredVisited {
                            expired_id,
                            projectile_id: listener_id,
                            expired_resolvable: self.substrate.entities.contains(expired_id),
                            projectile_resolvable: true,
                            source_id,
                            target,
                        },
                    );
                }
            }
        }
    }

    /// ObjectClass::UnInit represented ordering.  Physical removal is deferred.
    pub(crate) fn uninit(&mut self, stable_id: u64) {
        self.uninit_with_context(stable_id, UninitContext::default());
    }

    pub(crate) fn uninit_with_rules(&mut self, stable_id: u64, rules: &RuleSet) {
        self.uninit_with_context(stable_id, UninitContext::with_rules(rules));
    }

    pub(crate) fn uninit_with_context(&mut self, stable_id: u64, context: UninitContext<'_>) {
        if !self.substrate.entities.contains(stable_id) {
            return;
        }

        self.run_represented_uninit_pre_hook(stable_id);
        self.uninit_carried_passengers(stable_id, context);
        // `SpawnManagerClass::PointerExpired`, owner arm: `Kill_All_Spawns()`
        // then `ClearAllTargets()`. Docked/reloading children and any missile
        // still in its post-launch window die with the parent; aircraft already
        // out are released. The target clear is the second, separate call —
        // `Kill_All_Spawns` alone never touches the targets.
        if self
            .substrate
            .entities
            .get(stable_id)
            .is_some_and(|entity| entity.spawn_manager.is_some())
        {
            crate::sim::spawn_manager::kill_all_spawns_with_context(self, stable_id, context);
            crate::sim::spawn_manager::clear_all_spawn_targets(self, stable_id);
        }

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
                resolvable: self.substrate.entities.contains(stable_id),
            });
        }
        self.notify_pointer_expired(stable_id, context);

        let _ = self.techno_limbo_with_context(stable_id, context);
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

    #[cfg(test)]
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
            .is_some_and(|system| system.done_spawning && system.particles.is_empty());
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
            return system.done_spawning && system.particles.is_empty();
        }
        if let Some(terrain) = self.production.terrain_objects.get(&stable_id) {
            return !terrain.is_live() && !terrain.in_logic_vector;
        }
        if let Some(projectile) = self.projectiles.get(stable_id) {
            return !projectile.in_logic_vector;
        }
        if let Some(wave) = self.waves.get(stable_id) {
            return !wave.in_logic_vector;
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
        let terrain = self.production.terrain_objects.remove(&stable_id);
        if let Some(terrain) = terrain.as_ref() {
            let cell = terrain.cell();
            if self.production.terrain_object_cells.get(&cell) == Some(&stable_id) {
                self.production.terrain_object_cells.remove(&cell);
                self.production.terrain_spawners.remove(&cell);
                self.production.terrain_occupation_bits.remove(&cell);
                self.production
                    .tiberium_spawning_terrain_cells
                    .remove(&cell);
            }
        }
        let projectile = self.projectiles.remove(stable_id);
        let wave = self.waves.remove(stable_id);
        if let Some(system) = particle_system.as_ref()
            && let Some(owner_id) = system.owner_entity
            && let Some(owner) = self.substrate.entities.get_mut(owner_id)
            && owner.damage_smoke_system_id == Some(stable_id)
        {
            // Native pointer expiry clears TechnoClass +0x310 only when the
            // marked system physically leaves object storage. Keeping the
            // identity through retirement prevents same-frame duplicates.
            owner.damage_smoke_system_id = None;
        }
        debug_assert!(
            usize::from(entity.is_some())
                + usize::from(anim.is_some())
                + usize::from(particle_system.is_some())
                + usize::from(terrain.is_some())
                + usize::from(projectile.is_some())
                + usize::from(wave.is_some())
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
    ///
    /// gamemd-derived: active YR `DrainDeferredFinalizationQueue @ 0x00725C70`
    /// is reached from `Main_Tick` at `0x0055DE9F` after the frame commit; it
    /// preserves non-ready entries, collapses selected duplicates, and finalizes
    /// each selected ready object once.
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
