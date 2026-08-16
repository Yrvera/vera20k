//! Boundary tests (F14): raw lockstep command records converging with
//! semantic simulation execution.
//!
//! These tests exercise `net::lockstep` admission/dispatch against the full
//! `Simulation`, so they live on the net side of the sim/net boundary —
//! `sim/` itself never names `crate::net`. Moved verbatim from the
//! `sim::world` test module (whose fixtures they still share).

use crate::rules::ruleset::RuleSet;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::world::Simulation;
use crate::sim::world::tests::{
    empty_heights, gsi_04_07_wall_sell_rules, gsi_04_07_wall_sell_seed_houses,
};

#[test]
fn gsi_04_07_wall_sell_raw_lockstep_replay_converges_with_semantic_execution() {
    fn seeded_world() -> (
        Simulation,
        RuleSet,
        crate::map::overlay_types::OverlayTypeRegistry,
        crate::sim::intern::InternedId,
    ) {
        let (rules, overlays) = gsi_04_07_wall_sell_rules(false, false);
        let mut sim = Simulation::new();
        let (wall_owner, receiver) = gsi_04_07_wall_sell_seed_houses(&mut sim);
        sim.houses.get_mut(&receiver).unwrap().player_control = true;
        let mut grid = crate::sim::overlay_grid::OverlayGrid::new(3, 3);
        grid.place_owned_wall(1, 1, 2, 0, wall_owner);
        sim.overlay_grid = Some(grid);
        (sim, rules, overlays, receiver)
    }

    let (mut semantic, semantic_rules, semantic_overlays, semantic_receiver) = seeded_world();
    semantic.advance_tick(
        &[CommandEnvelope::new(
            semantic_receiver,
            0,
            Command::SellWallAtCell { x: 1, y: 1 },
        )],
        Some(&semantic_rules),
        &empty_heights(),
        None,
        Some(&semantic_overlays),
        67,
    );
    assert_eq!(
        semantic
            .overlay_grid
            .as_ref()
            .unwrap()
            .cell(1, 1)
            .overlay_id,
        None
    );

    let (mut raw, raw_rules, raw_overlays, raw_receiver) = seeded_world();
    let issued = raw
        .encode_sell_wall_at_cell_record(raw_receiver, 1, 1)
        .expect("registered receiver encodes");
    let replay = crate::sim::replay::NativeReplay {
        header: crate::sim::replay::NativeReplayHeader::new(1, "wall.map"),
        frames: vec![crate::sim::replay::NativeReplayFrame::record(
            crate::sim::replay::NativeReplayPresentation::new([0; 8], Vec::new(), [0; 2]),
            0,
            true,
            [&issued],
        )],
    };
    let replay_bytes = replay.encode().expect("native replay bytes");
    let decoded =
        crate::sim::replay::NativeReplay::decode_with_command_schedule(&replay_bytes, |_, _| true)
            .expect("native replay decode");
    let replay_record = decoded.frames[0].commands.as_ref().unwrap()[0].clone();

    let mut queue = crate::net::lockstep::SynchronizedCommandQueue::new();
    assert!(
        queue.admit(crate::net::lockstep::SynchronizedCommand::opaque(
            replay_record
        ))
    );
    let dispatch_houses = raw
        .session
        .house_order
        .iter()
        .copied()
        .enumerate()
        .map(|(index, owner)| {
            crate::net::lockstep::CommandDispatchHouse::new(
                owner,
                index as i8,
                raw.houses[&owner].event_dispatch_eligible(),
            )
        })
        .collect::<Vec<_>>();
    let mut due = Vec::new();
    let early = queue.dispatch_due_offline(
        -1,
        &dispatch_houses,
        |_, _| {},
        |_, command, _| {
            if let Some(envelope) = command.decode_for_simulation(&raw, raw.session.tick) {
                due.push(envelope);
            }
        },
    );
    assert_eq!(early.executed, 0, "frame -1 cannot execute frame-0 bytes");
    assert!(due.is_empty());
    let due_summary = queue.dispatch_due_offline(
        0,
        &dispatch_houses,
        |_, _| {},
        |_, command, _| {
            if let Some(envelope) = command.decode_for_simulation(&raw, raw.session.tick) {
                due.push(envelope);
            }
        },
    );
    assert_eq!(due_summary.executed, 1);
    assert_eq!(due.len(), 1);
    raw.advance_tick(
        &due,
        Some(&raw_rules),
        &empty_heights(),
        None,
        Some(&raw_overlays),
        67,
    );
    assert_eq!(
        raw.overlay_grid.as_ref().unwrap().cell(1, 1).overlay_id,
        None
    );
    assert_eq!(raw.state_hash(), semantic.state_hash());

    let invalid_house = crate::sim::command::SellWallAtCellRecord {
        house_id: 9,
        frame: 0,
        x: 1,
        y: 1,
    }
    .encode()
    .unwrap();
    assert!(
        crate::net::lockstep::SynchronizedCommand::opaque(invalid_house)
            .decode_for_simulation(&raw, raw.session.tick)
            .is_none()
    );
    let invalid_opcode =
        crate::sim::command::CommandRecord::encode(0x16, 1, 0, &[1, 0, 1, 0]).unwrap();
    assert!(
        crate::net::lockstep::SynchronizedCommand::opaque(invalid_opcode)
            .decode_for_simulation(&raw, raw.session.tick)
            .is_none()
    );
}

#[test]
fn gsi_04_07_wall_sell_raw_signed_linear_coordinates_use_canonical_cell() {
    fn apply_raw(
        sim: &mut Simulation,
        rules: &RuleSet,
        overlays: &crate::map::overlay_types::OverlayTypeRegistry,
        x: i16,
        y: i16,
    ) -> bool {
        let record = crate::sim::command::SellWallAtCellRecord {
            house_id: 1,
            frame: sim.session.binary_frame,
            x,
            y,
        }
        .encode()
        .expect("wall-sale record");
        let envelope = crate::net::lockstep::SynchronizedCommand::opaque(record)
            .decode_for_simulation(sim, sim.session.tick)
            .expect("registered receiver decodes");
        let receiver = sim.interner.resolve(envelope.owner).to_string();
        sim.apply_command_with_overlays(
            &receiver,
            &envelope.payload,
            Some(rules),
            None,
            &empty_heights(),
            Some(overlays),
        )
    }

    let (rules, overlays) = gsi_04_07_wall_sell_rules(false, false);
    let mut sim = Simulation::new();
    let (wall_owner, _) = gsi_04_07_wall_sell_seed_houses(&mut sim);
    let mut grid = crate::sim::overlay_grid::OverlayGrid::new(512, 2);
    for cell in [(1, 1), (511, 0), (0, 0), (2, 1)] {
        grid.place_owned_wall(cell.0, cell.1, 2, 0, wall_owner);
    }
    sim.overlay_grid = Some(grid);

    assert!(apply_raw(&mut sim, &rules, &overlays, 513, 0));
    assert_eq!(
        sim.overlay_grid.as_ref().unwrap().cell(1, 1).overlay_id,
        None,
        "linear index 513 canonicalizes to cell (1,1)"
    );

    assert!(apply_raw(&mut sim, &rules, &overlays, -1, 1));
    assert_eq!(
        sim.overlay_grid.as_ref().unwrap().cell(511, 0).overlay_id,
        None,
        "a negative component remains valid when its signed linear index is 511"
    );

    assert!(apply_raw(&mut sim, &rules, &overlays, 512, -1));
    assert_eq!(
        sim.overlay_grid.as_ref().unwrap().cell(0, 0).overlay_id,
        None,
        "only original packed (0,0), not an alias to it, is the null sentinel"
    );

    assert!(!apply_raw(&mut sim, &rules, &overlays, -1, 0));
    assert!(!apply_raw(&mut sim, &rules, &overlays, 0, 512));
    assert_eq!(
        sim.overlay_grid.as_ref().unwrap().cell(2, 1).overlay_id,
        Some(2),
        "negative and above-0x3ffff linear indices leave the allocated grid untouched"
    );
}
