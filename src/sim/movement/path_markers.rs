//! Temporary A* marker overlay construction from movement state.
//!
//! This module adapts live `MovementTarget` snapshots into the pathfinder's
//! search-scoped `SearchMarkerOverlay`. The overlay stays out of persistent
//! path/grid state and is rebuilt for each path request.

use crate::sim::entity_store::EntityStore;
use crate::sim::pathfinding::SearchMarkerOverlay;

const PEER_MARKER_REPLAY_LIMIT: usize = 24;
const PEER_MARKER_LOCAL_RADIUS: u16 = 3;

pub(super) fn build_peer_search_marker_overlay(
    entities: &EntityStore,
    mover_id: u64,
    request_start: (u16, u16),
) -> SearchMarkerOverlay {
    let mut overlay = SearchMarkerOverlay::new();
    for peer in entities.values() {
        // A Dying corpse keeps its movement_target but isn't moving; don't bias
        // the cooperative-pathing overlay with a dead peer's reserved path.
        if peer.dying || peer.stable_id == mover_id || peer.passenger_role.is_inside_transport() {
            continue;
        }
        if peer.position.rx.abs_diff(request_start.0) > PEER_MARKER_LOCAL_RADIUS
            || peer.position.ry.abs_diff(request_start.1) > PEER_MARKER_LOCAL_RADIUS
        {
            continue;
        }
        let Some(target) = peer.movement_target.as_ref() else {
            continue;
        };
        if target.bypass_grid {
            continue;
        }
        let start_index = target.next_index.max(1);
        for &cell in target
            .path
            .iter()
            .skip(start_index)
            .take(PEER_MARKER_REPLAY_LIMIT)
        {
            overlay.toggle(cell);
        }
    }
    overlay
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::components::Health;
    use crate::sim::components::MovementTarget;
    use crate::sim::game_entity::GameEntity;

    fn entity(id: u64, pos: (u16, u16), path: Vec<(u16, u16)>) -> GameEntity {
        let mut e = GameEntity::new(
            id,
            pos.0,
            pos.1,
            0,
            0,
            Default::default(),
            Health {
                current: 100,
                max: 100,
            },
            Default::default(),
            crate::map::entities::EntityCategory::Unit,
            0,
            0,
            true,
        );
        if !path.is_empty() {
            e.movement_target = Some(MovementTarget {
                path,
                next_index: 1,
                ..MovementTarget::default()
            });
        }
        e
    }

    #[test]
    fn peer_marker_overlay_replays_first_remaining_destination_and_caps_at_24() {
        let mut entities = EntityStore::new();
        entities.insert(entity(1, (0, 0), Vec::new()));
        let path: Vec<(u16, u16)> = (0..30).map(|x| (x, 1)).collect();
        entities.insert(entity(2, (1, 1), path));

        let overlay = build_peer_search_marker_overlay(&entities, 1, (0, 0));

        assert!(
            !overlay.contains((0, 1)),
            "path origin is not a destination mark"
        );
        assert!(
            overlay.contains((1, 1)),
            "first remaining destination must be marked"
        );
        assert!(
            overlay.contains((24, 1)),
            "24th replayed destination must be marked"
        );
        assert!(
            !overlay.contains((25, 1)),
            "peer marker replay must cap at 24 destinations"
        );
    }

    #[test]
    fn peer_marker_overlay_uses_xor_parity_for_duplicate_destinations() {
        let mut entities = EntityStore::new();
        entities.insert(entity(1, (0, 0), Vec::new()));
        entities.insert(entity(2, (1, 1), vec![(1, 1), (2, 1), (2, 1), (3, 1)]));

        let overlay = build_peer_search_marker_overlay(&entities, 1, (0, 0));

        assert!(
            !overlay.contains((2, 1)),
            "duplicate destination visits cancel like the original XOR marker bit"
        );
        assert!(overlay.contains((3, 1)));
    }

    #[test]
    fn peer_marker_overlay_excludes_mover_and_distant_peers() {
        let mut entities = EntityStore::new();
        entities.insert(entity(1, (0, 0), vec![(0, 0), (1, 0)]));
        entities.insert(entity(2, (20, 20), vec![(20, 20), (1, 0)]));
        entities.insert(entity(3, (1, 1), vec![(1, 1), (2, 0)]));

        let overlay = build_peer_search_marker_overlay(&entities, 1, (0, 0));

        assert!(
            !overlay.contains((1, 0)),
            "mover and distant peer paths are ignored"
        );
        assert!(overlay.contains((2, 0)));
    }
}
