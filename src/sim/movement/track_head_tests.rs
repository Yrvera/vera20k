use super::*;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::movement::locomotor::LocomotorState;
use crate::sim::pathfinding::PathGrid;
use crate::util::fixed_math::SimFixed;

fn corpus() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../tools/spatial_oracle/locomotor_head_coordinates.json"
    ))
    .unwrap()
}

fn coord(value: &serde_json::Value) -> DriveCoord {
    DriveCoord {
        x: value[0].as_i64().unwrap() as i32,
        y: value[1].as_i64().unwrap() as i32,
        z: value[2].as_i64().unwrap() as i32,
    }
}

#[test]
fn original_head_coordinate_expressions_match_all_288_saved_cases() {
    let corpus = corpus();
    let cases = corpus["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 288);
    for case in cases {
        let input = &case["input"];
        let source = if input["operation"] == "fresh" {
            &input["current"]
        } else {
            &input["base"]
        };
        assert_eq!(
            offset_head(coord(source), input["direction"].as_u64().unwrap() as u8),
            coord(&case["output"]),
            "{input}"
        );
    }
}

#[test]
fn actual_move_command_publishes_raw_head_and_matching_curve_for_drive_and_ship() {
    let corpus = corpus();
    for (kind, family) in [
        (LocomotorKind::Drive, "drive"),
        (LocomotorKind::Ship, "ship"),
    ] {
        let case = corpus["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| {
                let input = &case["input"];
                input["family"] == family
                    && input["operation"] == "fresh"
                    && input["name"] == "noncentered_retained_z"
                    && input["direction"] == 2
            })
            .unwrap();
        let initial = coord(&case["input"]["current"]);
        let expected = coord(&case["output"]);
        let mut entity = GameEntity::test_default(1, "MTNK", "Americans", 8, 8);
        entity.locomotor = Some(LocomotorState::for_test_kind(kind));
        entity.position.sub_x = SimFixed::from_num(initial.x % 256);
        entity.position.sub_y = SimFixed::from_num(initial.y % 256);
        entity.position.z = 1; // Deliberately differs from the retained raw Z.
        entity.position.exact_z_leptons = Some(initial.z);
        entity.facing = 64;
        let mut entities = EntityStore::new();
        entities.insert(entity);
        let grid = PathGrid::new(20, 20);
        assert!(crate::sim::movement::issue_move_command(
            &mut entities,
            &grid,
            1,
            (11, 8),
            SimFixed::from_num(768),
            false,
            None,
            None,
            None,
            false,
        ));
        let entity = entities.get(1).unwrap();
        let head = match kind {
            LocomotorKind::Drive => entity.drive_locomotion.as_ref().unwrap().head_to,
            LocomotorKind::Ship => entity.ship_locomotion.as_ref().unwrap().head_to,
            _ => unreachable!(),
        };
        assert_eq!(head, Some(expected), "{kind:?}");
        let curve = entity.drive_track.as_ref().unwrap();
        assert_eq!(curve.head_offset_x + 8 * 256, expected.x);
        assert_eq!(curve.head_offset_y + 8 * 256, expected.y);
        assert_eq!(curve.point_index, 0);
    }
}

#[test]
fn destination_change_preserves_the_committed_head() {
    let mut entity = GameEntity::test_default(1, "MTNK", "Americans", 8, 8);
    entity.locomotor = Some(LocomotorState::for_test_kind(LocomotorKind::Drive));
    let head = DriveCoord {
        x: 2389,
        y: 2201,
        z: 731,
    };
    entity.drive_locomotion = Some(crate::sim::components::DriveLocomotionRuntime {
        head_to: Some(head),
        ..Default::default()
    });
    crate::sim::movement::navcom::set_destination_internal_cell(&mut entity, (13, 8), None);
    assert_eq!(
        entity.drive_locomotion.as_ref().unwrap().head_to,
        Some(head)
    );
    assert_eq!(
        entity.drive_locomotion.as_ref().unwrap().destination,
        Some(DriveCoord::cell(13, 8, 0))
    );
    crate::sim::movement::navcom::refresh_drive_destination_coord(
        &mut entity,
        DriveCoord {
            x: 3109,
            y: 2201,
            z: -347,
        },
        None,
    );
    assert_eq!(
        entity.drive_locomotion.as_ref().unwrap().head_to,
        Some(head)
    );
}

#[test]
fn chained_queue_consumption_preserves_the_native_replay_reference() {
    let mut queue = crate::sim::components::DrivePathQueue {
        directions: vec![2, 3, 4],
        cursor: 2,
        reference_cell: Some((10, 9)),
        ..Default::default()
    };
    crate::sim::movement::path_markers::consume_path_replay(&mut queue, 1);
    assert_eq!(queue.cursor, 3);
    assert_eq!(queue.reference_cell, Some((10, 9)));
}
