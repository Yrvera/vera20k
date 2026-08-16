//! Search-scoped high-bridge passability markers.
//!
//! Retail `PathfinderClass::UpdateBridgePassability` XORs a temporary bit into
//! selected cells immediately before an urgency 1/2 A* search and XORs the same
//! cells again on every normal search exit.  Rust represents that transaction
//! as an owned [`SearchMarkerOverlay`], so persistent map state is never
//! mutated and cleanup is automatic when the search returns.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::cell_rect::{PlayfieldBounds, cell_is_in_playfield};
use crate::sim::components::DrivePathQueue;
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::movement::FacingClass;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::{OccupancyGrid, RawCellOccupationGrid};
use crate::sim::pathfinding::{PathGrid, SearchMarkerOverlay};
use crate::util::direction::{DIRECTION_DELTAS, TUBE_STEP_DIRECTION};
use crate::util::lepton::GROUND_LEVEL_HEIGHT_LEPTONS;

const PEER_MARKER_REPLAY_LIMIT: usize = 24;

#[derive(Debug, Clone)]
struct BridgeMarkerPeer {
    category: EntityCategory,
    foot_derived: bool,
    locomotor_kind: Option<LocomotorKind>,
    type_ref: InternedId,
    speed: i32,
    path_start: (i16, i16),
    path_directions: Vec<u8>,
    is_at_coord_track_cell: Option<(i16, i16)>,
    is_at_coord_head_cell: (i16, i16),
    /// Hover's non-null Head_To takes its Z from the accepted path cell, not
    /// from the linked Foot's current coordinate. Other verified receivers
    /// compare against the linked Foot Z and leave this unset.
    is_at_coord_head_layer: Option<MovementLayer>,
    current_height_leptons: i32,
}

/// Read-only entity facts needed while a mover itself is mutably borrowed.
///
/// Object-list order remains authoritative in [`OccupancyGrid`]; this map is
/// only an ID-to-facts lookup and therefore cannot reorder a native list.
#[derive(Debug, Clone, Default)]
pub(super) struct BridgeMarkerPeerSnapshot {
    peers: BTreeMap<u64, BridgeMarkerPeer>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct BridgeMarkerMover {
    pub current_cell: (u16, u16),
    pub facing: u8,
    pub body_facing: Option<FacingClass>,
    pub on_bridge: bool,
    pub type_ref: InternedId,
    pub speed: i32,
}

#[derive(Debug, Default)]
pub(super) struct BridgeMarkerSearch {
    pub overlay: SearchMarkerOverlay,
    /// Urgency actually supplied to A*.  Retail downgrades urgency 1 to zero
    /// when no eligible peer path was replayed, before the raw-byte phase.
    pub effective_urgency: u8,
    #[cfg(test)]
    processed_peer_path: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct BridgeMarkerContext<'a> {
    pub enabled: bool,
    pub peers: &'a BridgeMarkerPeerSnapshot,
    pub raw_occupation: &'a RawCellOccupationGrid,
    pub grid: &'a PathGrid,
    pub terrain: Option<&'a ResolvedTerrainGrid>,
    pub playfield_bounds: Option<PlayfieldBounds>,
    pub native_frame: u32,
}

impl BridgeMarkerContext<'_> {
    pub fn build(
        self,
        occupancy: &OccupancyGrid,
        entity_id: u64,
        current_cell: (u16, u16),
        facing: u8,
        body_facing: Option<FacingClass>,
        on_bridge: bool,
        requested_urgency: u8,
    ) -> BridgeMarkerSearch {
        let Some(mover) = self.peers.peers.get(&entity_id) else {
            return BridgeMarkerSearch {
                effective_urgency: requested_urgency,
                ..BridgeMarkerSearch::default()
            };
        };
        build_bridge_passability_search(
            self.enabled,
            self.peers,
            occupancy,
            self.raw_occupation,
            self.grid,
            self.terrain,
            self.playfield_bounds,
            BridgeMarkerMover {
                current_cell,
                facing,
                body_facing,
                on_bridge,
                type_ref: mover.type_ref,
                speed: mover.speed,
            },
            requested_urgency,
            self.native_frame,
        )
    }
}

fn direction_from_step(from: (i16, i16), to: (u16, u16)) -> u8 {
    let dx = i32::from(to.0 as i16) - i32::from(from.0);
    let dy = i32::from(to.1 as i16) - i32::from(from.1);
    DIRECTION_DELTAS
        .iter()
        .position(|&delta| delta == (dx, dy))
        .map_or(TUBE_STEP_DIRECTION, |index| index as u8)
}

pub(super) fn install_path_replay(
    queue: &mut DrivePathQueue,
    reference: (u16, u16),
    path: &[(u16, u16)],
    first_destination: usize,
) {
    let mut from = (reference.0 as i16, reference.1 as i16);
    queue.directions.clear();
    for &destination in path.iter().skip(first_destination) {
        queue
            .directions
            .push(direction_from_step(from, destination));
        from = (destination.0 as i16, destination.1 as i16);
    }
    queue.cursor = 0;
    queue.reference_cell = Some((reference.0 as i16, reference.1 as i16));
}

pub(super) fn accept_path_replay(
    queue: &mut DrivePathQueue,
    endpoint: (i16, i16),
    consumed_directions: usize,
) {
    queue.reference_cell = Some(endpoint);
    let cursor = usize::from(queue.cursor)
        .saturating_add(consumed_directions)
        .min(queue.directions.len());
    queue.cursor = cursor.min(u16::MAX as usize) as u16;
}

fn remaining_path_from_entity(
    entity: &crate::sim::game_entity::GameEntity,
) -> ((i16, i16), Vec<u8>) {
    let replay = match entity.locomotor.as_ref().map(|locomotor| locomotor.kind) {
        Some(LocomotorKind::Drive) => entity.drive_locomotion.as_ref().map(|drive| &drive.path),
        Some(LocomotorKind::Ship) => entity.ship_locomotion.as_ref().map(|ship| &ship.path),
        _ => None,
    };
    if let Some(queue) = replay
        && let Some(reference) = queue.reference_cell
    {
        let cursor = usize::from(queue.cursor).min(queue.directions.len());
        return (reference, queue.directions[cursor..].to_vec());
    }

    let mut reference = (entity.position.rx as i16, entity.position.ry as i16);
    let path_start = reference;
    let mut directions = Vec::new();
    if let Some(target) = entity.movement_target.as_ref() {
        for &destination in target.path.iter().skip(target.next_index) {
            directions.push(direction_from_step(reference, destination));
            reference = (destination.0 as i16, destination.1 as i16);
        }
    }
    (path_start, directions)
}

fn is_at_coord_cells(
    entity: &crate::sim::game_entity::GameEntity,
) -> (Option<(i16, i16)>, (i16, i16), Option<MovementLayer>) {
    const CELL_LEPTONS: i32 = 256;
    let current = (
        ((i32::from(entity.position.rx) * CELL_LEPTONS + entity.position.sub_x.to_num::<i32>())
            / CELL_LEPTONS) as i16,
        ((i32::from(entity.position.ry) * CELL_LEPTONS + entity.position.sub_y.to_num::<i32>())
            / CELL_LEPTONS) as i16,
    );
    let Some(locomotor) = entity.locomotor.as_ref() else {
        return (None, current, None);
    };
    match locomotor.kind {
        LocomotorKind::Drive => {
            let drive = entity.drive_locomotion.as_ref();
            let active_track = entity.drive_track.as_ref().or_else(|| {
                entity
                    .forced_drive_track
                    .as_ref()
                    .map(|forced| &forced.track)
            });
            if let Some(track) = active_track {
                // Every active ordinary and forced Drive constructor currently
                // installs the normal RawTrack.  A reversed/short provenance
                // suppresses the native transformed-handoff candidate.
                let (track_cell, head_cell) = super::drive_track::is_at_coord_track_cells(
                    track,
                    (entity.position.rx, entity.position.ry),
                    !drive.is_some_and(|drive| drive.is_reversed),
                );
                return (track_cell, head_cell, None);
            }
            let head = drive
                .and_then(|drive| drive.head_to)
                .map(|head| ((head.x / 256) as i16, (head.y / 256) as i16))
                .unwrap_or(current);
            (None, head, None)
        }
        LocomotorKind::Ship => {
            let active_track = entity.drive_track.as_ref().filter(|track| {
                track.raw_track_index <= 13
                    && entity
                        .forced_drive_track
                        .as_ref()
                        .is_none_or(|forced| forced.track.raw_track_index != track.raw_track_index)
            });
            if let Some(track) = active_track {
                let (track_cell, head_cell) = super::drive_track::is_at_coord_track_cells(
                    track,
                    (entity.position.rx, entity.position.ry),
                    true,
                );
                return (track_cell, head_cell, None);
            }
            let head = entity
                .ship_locomotion
                .as_ref()
                .and_then(|ship| ship.head_to)
                .map(|head| ((head.x / 256) as i16, (head.y / 256) as i16))
                .unwrap_or(current);
            (None, head, None)
        }
        LocomotorKind::Walk => (
            None,
            entity
                .movement_target
                .as_ref()
                .and_then(|target| target.path.get(target.next_index).copied())
                .map_or(current, |cell| (cell.0 as i16, cell.1 as i16)),
            None,
        ),
        LocomotorKind::Hover => {
            let head_to = entity.movement_target.as_ref().and_then(|target| {
                let cell = target.path.get(target.next_index).copied()?;
                let layer = target
                    .path_layers
                    .get(target.next_index)
                    .copied()
                    .unwrap_or(MovementLayer::Ground);
                Some(((cell.0 as i16, cell.1 as i16), layer))
            });
            head_to.map_or((None, current, None), |(cell, layer)| {
                (None, cell, Some(layer))
            })
        }
        _ => (None, current, None),
    }
}

pub(super) fn snapshot_bridge_marker_peers(
    entities: &EntityStore,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    interner: &StringInterner,
) -> BridgeMarkerPeerSnapshot {
    let peers = entities
        .values()
        .map(|entity| {
            let (path_start, path_directions) = remaining_path_from_entity(entity);
            let (is_at_coord_track_cell, is_at_coord_head_cell, is_at_coord_head_layer) =
                is_at_coord_cells(entity);
            let base_height_leptons =
                i32::from(entity.position.z as i8).wrapping_mul(GROUND_LEVEL_HEIGHT_LEPTONS);
            let current_height_leptons = entity
                .locomotor
                .as_ref()
                .filter(|locomotor| locomotor.kind == LocomotorKind::Hover)
                .map_or(base_height_leptons, |locomotor| {
                    base_height_leptons.wrapping_add(locomotor.altitude.to_num::<i32>())
                });
            let speed = rules
                .and_then(|rules| rules.object(interner.resolve(entity.type_ref)))
                .map_or(0, |object| object.speed);
            let foot_derived = matches!(
                entity.category,
                EntityCategory::Unit | EntityCategory::Infantry | EntityCategory::Aircraft
            );
            (
                entity.stable_id,
                BridgeMarkerPeer {
                    category: entity.category,
                    foot_derived,
                    locomotor_kind: entity.locomotor.as_ref().map(|locomotor| locomotor.kind),
                    type_ref: entity.type_ref,
                    speed,
                    path_start,
                    path_directions,
                    is_at_coord_track_cell,
                    is_at_coord_head_cell,
                    is_at_coord_head_layer,
                    current_height_leptons,
                },
            )
        })
        .collect();
    BridgeMarkerPeerSnapshot { peers }
}

fn signed_cell_add(cell: (i16, i16), delta: (i32, i32)) -> (i16, i16) {
    (
        cell.0.wrapping_add(delta.0 as i16),
        cell.1.wrapping_add(delta.1 as i16),
    )
}

fn unsigned_cell(cell: (i16, i16)) -> (u16, u16) {
    (cell.0 as u16, cell.1 as u16)
}

fn signed_ground_level(grid: &PathGrid, cell: (i16, i16)) -> i16 {
    let cell = unsigned_cell(cell);
    grid.cell(cell.0, cell.1)
        .map_or(0, |cell| cell.signed_level())
}

fn has_structural_bridge(grid: &PathGrid, cell: (i16, i16)) -> bool {
    let cell = unsigned_cell(cell);
    grid.cell(cell.0, cell.1)
        .is_some_and(|cell| cell.has_structural_bridge())
}

fn list_ids(occupancy: &OccupancyGrid, cell: (i16, i16), layer: MovementLayer) -> Vec<u64> {
    let cell = unsigned_cell(cell);
    occupancy
        .get(cell.0, cell.1)
        .map(|occupancy| {
            occupancy
                .iter_layer(layer)
                .map(|occupant| occupant.entity_id)
                .collect()
        })
        .unwrap_or_default()
}

fn find_nearby_bridge_peer_suffix(
    peers: &BridgeMarkerPeerSnapshot,
    occupancy: &OccupancyGrid,
    grid: &PathGrid,
    probe: (i16, i16),
    requested_height: i16,
) -> Vec<u64> {
    let requested_height_leptons =
        i32::from(requested_height).wrapping_mul(GROUND_LEVEL_HEIGHT_LEPTONS);
    // Retail helper nesting is dy outer, dx inner.
    for dy in -2..=2 {
        for dx in -2..=2 {
            let candidate = signed_cell_add(probe, (dx, dy));
            let candidate_level = signed_ground_level(grid, candidate);
            let layer = if has_structural_bridge(grid, candidate)
                && (candidate_level - requested_height).abs() > 2
            {
                MovementLayer::Bridge
            } else {
                MovementLayer::Ground
            };
            let list = list_ids(occupancy, candidate, layer);
            for (index, entity_id) in list.iter().copied().enumerate() {
                let Some(peer) = peers.peers.get(&entity_id) else {
                    continue;
                };
                if !peer.foot_derived
                    || !matches!(
                        peer.locomotor_kind,
                        Some(
                            LocomotorKind::Drive
                                | LocomotorKind::Ship
                                | LocomotorKind::Walk
                                | LocomotorKind::Hover
                        )
                    )
                    || (peer.is_at_coord_track_cell != Some(probe)
                        && peer.is_at_coord_head_cell != probe)
                {
                    continue;
                }
                let receiver_height_leptons = if peer.is_at_coord_head_cell == probe {
                    match peer.is_at_coord_head_layer {
                        Some(layer) => {
                            let cell = unsigned_cell(peer.is_at_coord_head_cell);
                            let Some(path_cell) = grid.cell(cell.0, cell.1) else {
                                continue;
                            };
                            i32::from(path_cell.effective_cell_z_for_layer(layer) as i8)
                                .wrapping_mul(GROUND_LEVEL_HEIGHT_LEPTONS)
                        }
                        None => peer.current_height_leptons,
                    }
                } else {
                    peer.current_height_leptons
                };
                if (receiver_height_leptons - requested_height_leptons).abs()
                    > GROUND_LEVEL_HEIGHT_LEPTONS
                {
                    continue;
                }
                // Caller receives this node and follows its +0x30 suffix only.
                return list[index..].to_vec();
            }
        }
    }
    Vec::new()
}

fn replay_peer_path(
    overlay: &mut SearchMarkerOverlay,
    peer: &BridgeMarkerPeer,
    terrain: Option<&ResolvedTerrainGrid>,
) {
    let mut replay = peer.path_start;
    for &direction in peer.path_directions.iter().take(PEER_MARKER_REPLAY_LIMIT) {
        replay = match direction {
            0..=7 => signed_cell_add(replay, DIRECTION_DELTAS[direction as usize]),
            TUBE_STEP_DIRECTION => {
                let current = unsigned_cell(replay);
                terrain
                    .and_then(|terrain| terrain.tube_at_cell(current.0, current.1))
                    .map_or((0, 0), |tube| (tube.exit.0 as i16, tube.exit.1 as i16))
            }
            _ => continue,
        };
        overlay.toggle(unsigned_cell(replay));
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_bridge_passability_search(
    enabled: bool,
    peers: &BridgeMarkerPeerSnapshot,
    occupancy: &OccupancyGrid,
    raw_occupation: &RawCellOccupationGrid,
    grid: &PathGrid,
    terrain: Option<&ResolvedTerrainGrid>,
    playfield_bounds: Option<PlayfieldBounds>,
    mover: BridgeMarkerMover,
    requested_urgency: u8,
    native_frame: u32,
) -> BridgeMarkerSearch {
    if !enabled || requested_urgency == 0 {
        return BridgeMarkerSearch {
            effective_urgency: requested_urgency,
            ..BridgeMarkerSearch::default()
        };
    }

    let facing = mover
        .body_facing
        .map_or(u16::from(mover.facing) << 8, |facing| {
            facing.current(native_frame)
        });
    let direction = crate::util::direction_tables::quantize::dir_from_facing16(facing);
    let current = (mover.current_cell.0 as i16, mover.current_cell.1 as i16);
    let probe = signed_cell_add(current, DIRECTION_DELTAS[direction as usize]);
    let current_level = signed_ground_level(grid, current);
    let probe_level = signed_ground_level(grid, probe);
    let direct_deck = has_structural_bridge(grid, probe)
        && ((current_level - probe_level).abs() > 3 || mover.on_bridge);
    let direct_layer = if direct_deck {
        MovementLayer::Bridge
    } else {
        MovementLayer::Ground
    };
    let requested_height = probe_level + if direct_deck { 4 } else { 0 };
    let mut selected_list = list_ids(occupancy, probe, direct_layer);
    if selected_list.is_empty() {
        selected_list =
            find_nearby_bridge_peer_suffix(peers, occupancy, grid, probe, requested_height);
    }

    let mut overlay = SearchMarkerOverlay::new();
    let mut processed_peer_path = false;
    for entity_id in selected_list {
        let Some(peer) = peers.peers.get(&entity_id) else {
            continue;
        };
        if !peer.foot_derived
            || !matches!(
                peer.category,
                EntityCategory::Unit | EntityCategory::Infantry
            )
        {
            continue;
        }
        if requested_urgency != 2 {
            if peer.type_ref == mover.type_ref || mover.speed <= peer.speed {
                continue;
            }
            if !cell_is_in_playfield(
                (i32::from(peer.path_start.0), i32::from(peer.path_start.1)),
                playfield_bounds,
                terrain,
                Some((grid.width(), grid.height())),
            ) {
                continue;
            }
        }
        let minimum_directions = if peer.category == EntityCategory::Infantry {
            3
        } else {
            2
        };
        if peer.path_directions.len() < minimum_directions {
            continue;
        }
        processed_peer_path = true;
        replay_peer_path(&mut overlay, peer, terrain);
    }

    if requested_urgency == 1 && !processed_peer_path {
        return BridgeMarkerSearch {
            overlay: SearchMarkerOverlay::new(),
            effective_urgency: 0,
            #[cfg(test)]
            processed_peer_path,
        };
    }

    // Retail occupation nesting is dx outer, dy inner and always reads the
    // full ground byte, regardless of the direct-list layer selected above.
    for dx in -2..=2 {
        for dy in -2..=2 {
            let candidate = signed_cell_add(probe, (dx, dy));
            let cell = unsigned_cell(candidate);
            if candidate != current && raw_occupation.ground_is_occupied(cell.0, cell.1) {
                overlay.toggle(cell);
            }
        }
    }
    // Unconditional center toggle makes an occupied probe cancel itself.
    overlay.toggle(unsigned_cell(probe));

    BridgeMarkerSearch {
        overlay,
        effective_urgency: requested_urgency,
        #[cfg(test)]
        processed_peer_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::occupancy::CellListInsertion;

    fn peer(
        category: EntityCategory,
        type_ref: InternedId,
        speed: i32,
        start: (i16, i16),
        directions: &[u8],
    ) -> BridgeMarkerPeer {
        BridgeMarkerPeer {
            category,
            foot_derived: true,
            locomotor_kind: Some(if category == EntityCategory::Infantry {
                LocomotorKind::Walk
            } else {
                LocomotorKind::Drive
            }),
            type_ref,
            speed,
            path_start: start,
            path_directions: directions.to_vec(),
            is_at_coord_track_cell: None,
            is_at_coord_head_cell: start,
            is_at_coord_head_layer: None,
            current_height_leptons: 0,
        }
    }

    fn mover(type_ref: InternedId) -> BridgeMarkerMover {
        BridgeMarkerMover {
            current_cell: (5, 5),
            facing: 0,
            body_facing: None,
            on_bridge: false,
            type_ref,
            speed: 8,
        }
    }

    #[test]
    fn gsi_04_12_marker_urgency_zero_is_inert_and_urgency_one_without_peer_downgrades() {
        let interner = StringInterner::new();
        let mover_type = interner.get("MOVER").unwrap_or_default();
        let peers = BridgeMarkerPeerSnapshot::default();
        let occupancy = OccupancyGrid::new();
        let mut raw = RawCellOccupationGrid::new();
        raw.mark_ground(5, 4, 0x20);
        let grid = PathGrid::new(12, 12);

        let zero = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            mover(mover_type),
            0,
            0,
        );
        assert!(zero.overlay.is_empty());
        assert_eq!(zero.effective_urgency, 0);

        let one = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            mover(mover_type),
            1,
            0,
        );
        assert!(one.overlay.is_empty(), "raw phase is skipped on downgrade");
        assert_eq!(one.effective_urgency, 0);
    }

    #[test]
    fn gsi_04_12_marker_direct_list_uses_facing_layer_and_replay_xor() {
        let mut interner = StringInterner::new();
        let mover_type = interner.intern("FAST");
        let peer_type = interner.intern("SLOW");
        let mut peers = BridgeMarkerPeerSnapshot::default();
        peers.peers.insert(
            2,
            peer(EntityCategory::Unit, peer_type, 4, (5, 4), &[2, 2, 6]),
        );
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            5,
            4,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let grid = PathGrid::new(12, 12);
        let raw = RawCellOccupationGrid::new();

        let search = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            mover(mover_type),
            1,
            0,
        );
        assert!(search.processed_peer_path);
        assert!(!search.overlay.contains((6, 4)), "duplicate visit cancels");
        assert!(search.overlay.contains((7, 4)));
        assert!(
            search.overlay.contains((5, 4)),
            "unoccupied probe center toggles"
        );
    }

    #[test]
    fn gsi_04_12_marker_fallback_returns_first_is_at_coord_list_suffix() {
        let mut interner = StringInterner::new();
        let mover_type = interner.intern("FAST");
        let rejected_type = interner.intern("SAME");
        let accepted_type = interner.intern("SLOW");
        let suffix_type = interner.intern("SLOWER");
        let mut peers = BridgeMarkerPeerSnapshot::default();
        let mut rejected = peer(EntityCategory::Unit, rejected_type, 1, (3, 2), &[2, 2]);
        rejected.is_at_coord_head_cell = (0, 0);
        peers.peers.insert(2, rejected);
        let mut accepted = peer(EntityCategory::Unit, accepted_type, 2, (5, 4), &[2, 2]);
        accepted.is_at_coord_head_cell = (5, 4);
        peers.peers.insert(3, accepted);
        peers.peers.insert(
            4,
            peer(EntityCategory::Unit, suffix_type, 3, (5, 4), &[4, 4]),
        );
        let mut occupancy = OccupancyGrid::new();
        // Probe (5,4) is empty. Candidate (3,2) is first in dy/dx order.
        for id in [4, 3, 2] {
            occupancy.add(
                3,
                2,
                id,
                MovementLayer::Ground,
                None,
                CellListInsertion::PrependNonBuilding,
            );
        }
        let grid = PathGrid::new(12, 12);
        let raw = RawCellOccupationGrid::new();
        let search = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            mover(mover_type),
            1,
            0,
        );
        assert!(search.overlay.contains((6, 4)), "accepted peer replayed");
        assert!(search.overlay.contains((5, 6)), "same-list suffix replayed");
    }

    #[test]
    fn gsi_04_12_marker_drive_deck_track_fallback_uses_unconsumed_handoff() {
        let mut interner = StringInterner::new();
        let mover_type = interner.intern("MOVER");
        let mut entities = EntityStore::new();
        let mut peer =
            crate::sim::game_entity::GameEntity::test_default(2, "PEER", "Americans", 4, 4);
        peer.locomotor = Some(
            crate::sim::movement::locomotor::LocomotorState::for_test_kind(LocomotorKind::Drive),
        );
        peer.position.z = 4;
        peer.on_bridge = true;
        let mut drive = crate::sim::components::DriveLocomotionRuntime::default();
        drive.head_to = Some(crate::sim::components::DriveCoord::cell(9, 9, 4));
        drive.track_valid = true;
        drive.track_index = 3;
        drive.point_index = 12;
        drive.path.reference_cell = Some((5, 4));
        drive.path.directions = vec![2, 2];
        peer.drive_locomotion = Some(drive);
        // RawTrack 3 handoff point 22, transformed around head cell (6,3),
        // lies in probe cell (5,4).  A deck track deliberately owns no ground
        // occupation_head_to reservation, so that field cannot answer slot 40.
        peer.drive_track = super::super::drive_track::begin_drive_track(3, 0, 2, -1, 32);
        entities.insert(peer);

        let peers = snapshot_bridge_marker_peers(&entities, None, &interner);
        let peer = peers.peers.get(&2).expect("Drive peer snapshot");
        assert_eq!(peer.is_at_coord_track_cell, Some((5, 4)));
        assert_eq!(peer.is_at_coord_head_cell, (6, 3));

        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            4,
            4,
            2,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let mut grid = PathGrid::new(12, 12);
        grid.set_cell_for_test(5, 4, 0, true, false);
        grid.set_cell_for_test(4, 4, 0, true, false);
        let raw = RawCellOccupationGrid::new();
        let mut bridge_mover = mover(mover_type);
        bridge_mover.on_bridge = true;
        let search = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            bridge_mover,
            2,
            0,
        );

        assert!(search.processed_peer_path);
        assert!(search.overlay.contains((6, 4)));
        assert!(search.overlay.contains((7, 4)));
    }

    #[test]
    fn gsi_04_12_marker_hover_fallback_uses_live_head_to_and_inclusive_height() {
        let mut interner = StringInterner::new();
        let mover_type = interner.intern("MOVER");
        let hover_type = interner.intern("HOVER");
        let mut entities = EntityStore::new();
        let mut peer =
            crate::sim::game_entity::GameEntity::test_default(2, "HOVER", "Americans", 4, 3);
        peer.type_ref = hover_type;
        peer.position.z = 7;
        peer.locomotor = Some(
            crate::sim::movement::locomotor::LocomotorState::for_test_kind(LocomotorKind::Hover),
        );
        peer.movement_target = Some(crate::sim::components::MovementTarget {
            path: vec![(5, 4), (6, 4), (7, 4)],
            path_layers: vec![
                MovementLayer::Bridge,
                MovementLayer::Ground,
                MovementLayer::Ground,
            ],
            next_index: 0,
            ..crate::sim::components::MovementTarget::default()
        });
        entities.insert(peer);

        let peers = snapshot_bridge_marker_peers(&entities, None, &interner);
        let peer = peers.peers.get(&2).expect("Hover peer snapshot");
        assert_eq!(peer.is_at_coord_head_cell, (5, 4));
        assert_eq!(peer.is_at_coord_head_layer, Some(MovementLayer::Bridge));

        let mut occupancy = OccupancyGrid::new();
        // The probe itself is empty; only the nearby list can expose Hover.
        occupancy.add(
            4,
            3,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let flat = crate::sim::pathfinding::PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            bridge_structural: false,
            bridge_marker_0x80: false,
            transition: false,
            ground_level: 0,
            bridge_deck_level: 0,
            slope_type: 0,
            tube_index: None,
            low_bridge_tube_cell: false,
        };
        let mut cells = vec![flat; 12 * 12];
        // One level is the Rust map-height equivalent of the native inclusive
        // 0x68-lepton receiver/query Z tolerance. The current Foot Z above is
        // deliberately outside it, proving Head_To Z stays paired with XY.
        cells[4 * 12 + 5] = crate::sim::pathfinding::PathCell {
            bridge_walkable: true,
            bridge_structural: true,
            transition: true,
            bridge_deck_level: 1,
            ..flat
        };
        let grid = PathGrid::from_cells(cells, 12, 12);
        let raw = RawCellOccupationGrid::new();
        let search = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            mover(mover_type),
            1,
            0,
        );

        assert_eq!(search.effective_urgency, 1);
        assert!(search.processed_peer_path);
        assert!(search.overlay.contains((6, 4)));
        assert!(search.overlay.contains((7, 4)));
    }

    #[test]
    fn gsi_04_12_marker_idle_hover_fallback_keeps_exact_current_altitude() {
        let mut interner = StringInterner::new();
        let hover_type = interner.intern("HOVER");
        let mut entities = EntityStore::new();
        let mut peer =
            crate::sim::game_entity::GameEntity::test_default(2, "HOVER", "Americans", 5, 4);
        peer.type_ref = hover_type;
        let mut locomotor =
            crate::sim::movement::locomotor::LocomotorState::for_test_kind(LocomotorKind::Hover);
        locomotor.altitude = crate::util::fixed_math::SimFixed::from_num(120);
        peer.locomotor = Some(locomotor);
        entities.insert(peer);

        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            4,
            3,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let grid = PathGrid::new(12, 12);
        let above_tolerance = snapshot_bridge_marker_peers(&entities, None, &interner);
        let peer = above_tolerance.peers.get(&2).expect("idle Hover snapshot");
        assert_eq!(peer.is_at_coord_head_cell, (5, 4));
        assert_eq!(peer.is_at_coord_head_layer, None);
        assert_eq!(peer.current_height_leptons, 120);
        assert!(
            find_nearby_bridge_peer_suffix(&above_tolerance, &occupancy, &grid, (5, 4), 0,)
                .is_empty(),
            "120-lepton idle hover height exceeds the 104-lepton tolerance"
        );

        entities
            .get_mut(2)
            .unwrap()
            .locomotor
            .as_mut()
            .unwrap()
            .altitude = crate::util::fixed_math::SimFixed::from_num(104);
        let inclusive_boundary = snapshot_bridge_marker_peers(&entities, None, &interner);
        assert_eq!(
            find_nearby_bridge_peer_suffix(&inclusive_boundary, &occupancy, &grid, (5, 4), 0,),
            vec![2],
            "the native 104-lepton boundary is inclusive"
        );
    }

    #[test]
    fn gsi_04_12_marker_urgency_two_raw_phase_cancels_occupied_probe() {
        let mut interner = StringInterner::new();
        let mover_type = interner.intern("MOVER");
        let peers = BridgeMarkerPeerSnapshot::default();
        let occupancy = OccupancyGrid::new();
        let mut raw = RawCellOccupationGrid::new();
        raw.mark_ground(5, 4, 0x01);
        raw.mark_ground(4, 4, 0x80);
        raw.mark_ground(5, 5, 0x20);
        let grid = PathGrid::new(12, 12);
        let search = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            mover(mover_type),
            2,
            0,
        );
        assert_eq!(search.effective_urgency, 2);
        assert!(
            !search.overlay.contains((5, 4)),
            "occupied probe toggles twice"
        );
        assert!(search.overlay.contains((4, 4)));
        assert!(!search.overlay.contains((5, 5)), "mover current is skipped");
    }

    #[test]
    fn gsi_04_12_marker_direct_layer_uses_strict_level_four_or_on_bridge() {
        let mut interner = StringInterner::new();
        let mover_type = interner.intern("FAST");
        let ground_type = interner.intern("GROUND");
        let deck_type = interner.intern("DECK");
        let mut peers = BridgeMarkerPeerSnapshot::default();
        peers.peers.insert(
            2,
            peer(EntityCategory::Unit, ground_type, 2, (5, 4), &[2, 2]),
        );
        peers
            .peers
            .insert(3, peer(EntityCategory::Unit, deck_type, 2, (5, 4), &[6, 6]));
        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            5,
            4,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        occupancy.add(
            5,
            4,
            3,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let mut grid = PathGrid::new(12, 12);
        grid.set_cell_for_test(5, 4, 0, true, false);
        grid.set_cell_for_test(5, 5, 3, false, false);
        let raw = RawCellOccupationGrid::new();

        let strict_three = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            mover(mover_type),
            1,
            0,
        );
        assert!(strict_three.overlay.contains((7, 4)));
        assert!(!strict_three.overlay.contains((3, 4)));

        grid.set_cell_for_test(5, 5, 4, false, false);
        let level_four = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            mover(mover_type),
            1,
            0,
        );
        assert!(level_four.overlay.contains((3, 4)));
        assert!(!level_four.overlay.contains((7, 4)));

        grid.set_cell_for_test(5, 5, 0, false, false);
        let mut bridge_mover = mover(mover_type);
        bridge_mover.on_bridge = true;
        let on_bridge = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            bridge_mover,
            1,
            0,
        );
        assert!(on_bridge.overlay.contains((3, 4)));
    }

    #[test]
    fn gsi_04_12_marker_urgency_gates_and_path_prerequisites_are_exact() {
        let mut interner = StringInterner::new();
        let mover_type = interner.intern("FAST");
        let other_type = interner.intern("OTHER");
        let mut peers = BridgeMarkerPeerSnapshot::default();
        peers.peers.insert(
            2,
            peer(EntityCategory::Unit, mover_type, 1, (5, 4), &[2, 2]),
        );
        peers.peers.insert(
            3,
            peer(EntityCategory::Unit, other_type, 8, (5, 4), &[4, 4]),
        );
        peers
            .peers
            .insert(4, peer(EntityCategory::Unit, other_type, 1, (5, 4), &[6]));
        peers.peers.insert(
            5,
            peer(EntityCategory::Infantry, other_type, 1, (5, 4), &[0, 0]),
        );
        let mut occupancy = OccupancyGrid::new();
        for id in 2..=5 {
            occupancy.add(
                5,
                4,
                id,
                MovementLayer::Ground,
                None,
                CellListInsertion::PrependNonBuilding,
            );
        }
        let grid = PathGrid::new(12, 12);
        let raw = RawCellOccupationGrid::new();

        let urgency_one = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            mover(mover_type),
            1,
            0,
        );
        assert_eq!(urgency_one.effective_urgency, 0);
        assert!(!urgency_one.processed_peer_path);

        let urgency_two = build_bridge_passability_search(
            true,
            &peers,
            &occupancy,
            &raw,
            &grid,
            None,
            None,
            mover(mover_type),
            2,
            0,
        );
        assert!(urgency_two.processed_peer_path);
        assert!(
            urgency_two.overlay.contains((7, 4)),
            "same-type Unit bypassed"
        );
        assert!(
            urgency_two.overlay.contains((5, 6)),
            "equal-speed Unit bypassed"
        );
        assert!(
            !urgency_two.overlay.contains((4, 4)),
            "Unit one-direction prerequisite remains enforced"
        );
        assert!(
            !urgency_two.overlay.contains((5, 2)),
            "Infantry two-direction prerequisite remains enforced"
        );
    }

    #[test]
    fn gsi_04_12_marker_replay_caps_at_24_and_missing_tube_continues_from_zero() {
        let mut interner = StringInterner::new();
        let peer_type = interner.intern("PEER");
        let mut overlay = SearchMarkerOverlay::new();
        replay_peer_path(
            &mut overlay,
            &peer(EntityCategory::Unit, peer_type, 1, (0, 0), &vec![2; 25]),
            None,
        );
        assert!(overlay.contains((24, 0)));
        assert!(!overlay.contains((25, 0)));

        let mut tube_overlay = SearchMarkerOverlay::new();
        replay_peer_path(
            &mut tube_overlay,
            &peer(
                EntityCategory::Unit,
                peer_type,
                1,
                (8, 8),
                &[TUBE_STEP_DIRECTION, 2],
            ),
            None,
        );
        assert!(tube_overlay.contains((0, 0)));
        assert!(tube_overlay.contains((1, 0)));
    }
}
