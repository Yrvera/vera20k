//! Where an entity is drawn: the isometric projection plus every visual-only
//! vertical offset that sits on top of it.
//!
//! ## Why this module exists
//!
//! `Position` used to carry two cached `f32` screen coordinates, and five
//! places in `sim/` wrote them — three of those writing a *visual* height lift
//! that has no simulation meaning. Screen placement is a render concern, so it
//! lives here, and `sim/` no longer writes screen coordinates at all.
//!
//! Nothing is cached. The projection is a handful of multiplies and every
//! input is authoritative sim state, so deriving on read is both cheaper than
//! the invalidation it replaces and immune to the stale-cache class of bug
//! (position mutated by a pass that forgot to refresh).
//!
//! ## The height lift, and a recorded DRIFT
//!
//! gamemd applies the height lift **once**, inlined in `CoordsToClient`, as
//! part of the projection itself: every drawn thing goes through it and picks
//! up `screen_y -= Z_leptons * k + …`. The multiplier `k` is not a literal in
//! the image — it is computed at startup from the camera model as
//!
//! ```text
//! k = sin(60°) * 60.0 / (256 * sqrt(2))  ≈  0.14352
//! ```
//!
//! that is, the tile width in pixels spread over the cell diagonal in leptons,
//! scaled by the sine of the camera angle. (Verified via
//! `decompile_function 0x006d1bdd` for the product,
//! `disassemble_bytes 0x006d1830` and `0x006d18c0` for the two operands, and
//! `read_memory` on `0x007e1708` / `0x007e1710` / `0x007e1728` / `0x007f4180` /
//! `0x007f4188` for the literals: 2.0, 256.0, 60.0, 60.0 and π/180. Each of the
//! three globals involved has exactly one writer, confirmed by `get_xrefs_to`.)
//!
//! VERA does not match that, and the mismatch is **not uniform** — it differs
//! by which state carries the height:
//!
//! | height carried by | effective px/lepton | vs gamemd's ~0.1435 |
//! |---|---|---|
//! | an air locomotor (Fly, Jumpjet) | 0.12 | ~16% low |
//! | parachute descent, scripted missiles | 0.06 | ~58% low |
//!
//! The 0.12 is not a tuned value: it is 0.06 applied twice, once by the air
//! movement pass and once again by each app draw path, which is precisely the
//! accident that smearing this concern across two layers produced. Collapsing
//! it to one owner is what this module is for; the two constants below
//! reproduce today's output exactly so that the move is pixel-for-pixel
//! neutral. **Correcting the value is a separate, deliberate change** — it
//! shifts every aircraft, paradrop and missile on screen and wants a live look
//! before it lands.
//!
//! Frequency, since severity needs one: every airborne unit in every match, for
//! as long as it is airborne. Kirovs, Harriers, Rocketeers, Nighthawks, every
//! jumpjet, every paradrop and every missile in flight.
//!
//! ## Dependency rules
//! - Part of render/ — reads sim/ state read-only and writes none of it.
//! - sim/ NEVER depends on render/, so nothing here may be called from sim/.

use crate::map::entities::EntityCategory;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::game_entity::GameEntity;
use crate::sim::movement::locomotor::MovementLayer;
use crate::util::fixed_math::sim_to_f32;

/// Upward screen lift per lepton of height, for height carried by an **air
/// locomotor** (Fly, Jumpjet).
///
/// See the module header: this is 0.06 applied twice, preserved verbatim so the
/// move off `Position` changes no pixels. gamemd's single-application value is
/// ~0.14352.
pub const AIR_LIFT_PX_PER_LEPTON: f32 = 0.12;

/// Upward screen lift per lepton of height, for height carried by
/// **object-level state** — parachute descent and scripted missiles.
///
/// Half the air value, for no better reason than that these two passes wrote
/// `screen_y` themselves and the app draw paths did not add a second helping.
pub const OBJECT_LIFT_PX_PER_LEPTON: f32 = 0.06;

/// Infantry walking bob amplitude, in screen pixels.
///
/// ~1 px is barely perceptible — just enough to feel alive. The phase it reads
/// still accumulates in `sim/` (see [`infantry_bob_px`]).
pub const INFANTRY_WOBBLE_AMPLITUDE: f32 = 1.0;

/// What is carrying this entity's visual height, if anything.
///
/// The variants are ordered by the precedence the sim tick used to establish by
/// write order: the ground pass ran first and the later passes overwrote it, so
/// the last pass to touch an entity won. Preserved here as an explicit `match`
/// rather than left implicit in pass ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeightSource {
    /// Object-level falling state — a paradropped unit under a parachute.
    Parachute,
    /// A scripted missile in flight.
    Rocket,
    /// An air-layer locomotor holding the unit up.
    AirLocomotor,
    /// On the ground. Only the infantry walking bob applies.
    Ground,
}

fn height_source(entity: &GameEntity) -> HeightSource {
    if entity.parachute_state.is_some() {
        return HeightSource::Parachute;
    }
    if entity.rocket_state.is_some() {
        return HeightSource::Rocket;
    }
    let airborne = entity
        .locomotor
        .as_ref()
        .is_some_and(|loco| loco.layer == MovementLayer::Air && loco.kind != LocomotorKind::Rocket);
    if airborne {
        HeightSource::AirLocomotor
    } else {
        HeightSource::Ground
    }
}

/// The infantry walking bob, in screen pixels (positive = lift).
///
/// The phase this reads is still owned and accumulated by the movement tick.
/// Moving it here would mean render keeping per-entity state and advancing it
/// per *frame* instead of per *tick*, which changes the bob's cadence — a
/// visible change, and not one this slice is buying. The phase is a pure
/// render input in the meantime: it feeds this offset and nothing else.
fn infantry_bob_px(entity: &GameEntity) -> f32 {
    if entity.category != EntityCategory::Infantry {
        return 0.0;
    }
    let Some(loco) = entity.locomotor.as_ref() else {
        return 0.0;
    };
    if loco.infantry_wobble_phase == 0.0 {
        return 0.0;
    }
    loco.infantry_wobble_phase.cos() * INFANTRY_WOBBLE_AMPLITUDE
}

/// Total upward screen lift for an entity, in pixels.
///
/// Positive lifts the entity toward the top of the screen (screen Y decreases).
pub fn height_lift_px(entity: &GameEntity) -> f32 {
    let locomotor_altitude: f32 = entity
        .locomotor
        .as_ref()
        .map(|loco| sim_to_f32(loco.altitude))
        .unwrap_or(0.0);

    match height_source(entity) {
        HeightSource::Parachute => {
            let altitude = entity
                .parachute_state
                .as_ref()
                .map(|state| sim_to_f32(state.altitude))
                .unwrap_or(0.0);
            altitude * OBJECT_LIFT_PX_PER_LEPTON
        }
        HeightSource::Rocket => {
            let altitude = entity
                .rocket_state
                .as_ref()
                .map(|state| sim_to_f32(state.altitude))
                .unwrap_or(0.0);
            altitude * OBJECT_LIFT_PX_PER_LEPTON
        }
        HeightSource::AirLocomotor => locomotor_altitude * AIR_LIFT_PX_PER_LEPTON,
        HeightSource::Ground => infantry_bob_px(entity),
    }
}

/// Where this entity is drawn, in world-space screen pixels.
///
/// This is the one place that answers the question. Everything that draws an
/// entity, brackets it, hangs a health bar over it or anchors an effect to it
/// goes through here, so they cannot drift apart.
pub fn screen_position(entity: &GameEntity) -> (f32, f32) {
    let (sx, sy) = ground_screen_position(&entity.position);
    (sx, sy - height_lift_px(entity))
}

/// The isometric projection alone, with no height lift applied.
///
/// For callers that place something on the ground under an entity rather than
/// on the entity itself, and for entities that have no height state at all.
pub fn ground_screen_position(position: &crate::sim::components::Position) -> (f32, f32) {
    crate::util::lepton::lepton_to_screen(
        position.rx,
        position.ry,
        position.sub_x,
        position.sub_y,
        position.z,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::locomotor_type::LocomotorKind;
    use crate::sim::movement::locomotor::LocomotorState;
    use crate::util::fixed_math::SimFixed;

    fn air_unit(kind: LocomotorKind, altitude: i32) -> GameEntity {
        let mut entity = GameEntity::test_default(1, "BEAG", "Americans", 5, 5);
        entity.category = EntityCategory::Aircraft;
        let mut loco = LocomotorState::for_test_kind(kind);
        loco.layer = MovementLayer::Air;
        loco.altitude = SimFixed::from_num(altitude);
        entity.locomotor = Some(loco);
        entity
    }

    #[test]
    fn a_grounded_unit_draws_at_its_projection() {
        let entity = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        assert_eq!(
            screen_position(&entity),
            ground_screen_position(&entity.position),
            "nothing is holding this unit up, so nothing may lift it"
        );
    }

    /// The whole point of the module: one owner, so the lift cannot be applied
    /// twice. At YR's stock `FlightLevel=1500` the air lift is 180px.
    #[test]
    fn air_altitude_lifts_once_at_the_air_rate() {
        let entity = air_unit(LocomotorKind::Fly, 1500);
        let (_, ground_sy) = ground_screen_position(&entity.position);
        let (_, sy) = screen_position(&entity);
        assert!(
            (ground_sy - sy - 1500.0 * AIR_LIFT_PX_PER_LEPTON).abs() < 0.01,
            "expected exactly one application of the air lift, got {}px",
            ground_sy - sy
        );
    }

    #[test]
    fn height_does_not_move_an_entity_sideways() {
        let entity = air_unit(LocomotorKind::Jumpjet, 900);
        assert_eq!(
            screen_position(&entity).0,
            ground_screen_position(&entity.position).0,
            "the lift is vertical only"
        );
    }

    /// A ground locomotor's altitude is not a height source. Only the air layer
    /// lifts, which is what keeps a deployed or docked unit on the floor.
    #[test]
    fn a_ground_layer_locomotor_never_lifts() {
        let mut entity = air_unit(LocomotorKind::Drive, 1500);
        entity.category = EntityCategory::Unit;
        if let Some(loco) = entity.locomotor.as_mut() {
            loco.layer = MovementLayer::Ground;
        }
        assert_eq!(
            screen_position(&entity),
            ground_screen_position(&entity.position),
        );
    }

    /// Object-level falling wins over the locomotor, matching the write order
    /// the sim tick used to have: the parachute pass ran after the air pass.
    #[test]
    fn parachute_state_takes_precedence_over_an_air_locomotor() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::movement::parachute_descent::begin_parachute_descent;

        let mut entities = EntityStore::default();
        let mut entity = air_unit(LocomotorKind::Jumpjet, 1500);
        entity.category = EntityCategory::Infantry;
        entities.insert(entity);
        assert!(begin_parachute_descent(
            &mut entities,
            1,
            SimFixed::from_num(400)
        ));
        let entity = entities.get(1).expect("entity");

        let (_, ground_sy) = ground_screen_position(&entity.position);
        let (_, sy) = screen_position(&entity);
        assert!(
            (ground_sy - sy - 400.0 * OBJECT_LIFT_PX_PER_LEPTON).abs() < 0.01,
            "the parachute altitude must be the only height source, got {}px",
            ground_sy - sy
        );
    }

    #[test]
    fn the_infantry_bob_only_applies_on_the_ground() {
        let mut entity = GameEntity::test_default(1, "E1", "Americans", 5, 5);
        entity.category = EntityCategory::Infantry;
        let mut loco = LocomotorState::for_test_kind(LocomotorKind::Walk);
        loco.infantry_wobble_phase = 0.0;
        entity.locomotor = Some(loco);
        assert_eq!(height_lift_px(&entity), 0.0, "phase 0 is the resting state");

        if let Some(loco) = entity.locomotor.as_mut() {
            loco.infantry_wobble_phase = std::f32::consts::PI;
        }
        assert!(
            (height_lift_px(&entity) + INFANTRY_WOBBLE_AMPLITUDE).abs() < 0.001,
            "cos(pi) = -1, so the bob is at the bottom of its travel"
        );
    }

    /// The slice's exit criterion, and the thing that keeps it from growing
    /// back: no file under `sim/` may assign a screen coordinate. Screen
    /// placement is derived here, so a write over there is by definition a
    /// second owner — which is how the height lift came to be applied twice.
    ///
    /// Deliberately NOT asserted: that `sim/` holds no `f32` at all.
    /// `LocomotorState::infantry_wobble_phase` is still there and still `f32`.
    /// It is a pure render input — it feeds [`infantry_bob_px`] and nothing
    /// else — but moving it would change the bob from per-tick to per-frame
    /// cadence, which is a visible change this slice did not buy.
    #[test]
    fn sim_writes_no_screen_coordinates() {
        fn walk(dir: &std::path::Path, out: &mut Vec<(String, usize, String)>) {
            for entry in std::fs::read_dir(dir).expect("readable") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let text = std::fs::read_to_string(&path).expect("utf-8 source");
                    for (i, line) in text.lines().enumerate() {
                        if is_screen_coord_write(line) {
                            out.push((path.display().to_string(), i + 1, line.trim().to_string()));
                        }
                    }
                }
            }
        }

        let sim = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/sim");
        let mut writes = Vec::new();
        walk(&sim, &mut writes);
        assert!(
            writes.is_empty(),
            "sim/ must not assign screen coordinates — that is \
             render::locomotor_visual's job. Found:\n{}",
            writes
                .iter()
                .map(|(f, l, t)| format!("  {f}:{l}: {t}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn the_guard_recognises_a_write_when_it_sees_one() {
        // Guards that can never fire are worse than no guard, so exercise the
        // matcher against the shapes the old code actually used.
        for line in [
            "entity.position.screen_x = sx;",
            "self.screen_y = sy;",
            "entity.position.screen_y -= bob;",
            "        e.position.screen_x  =  nx;",
        ] {
            assert!(is_screen_coord_write(line), "missed: {line}");
        }
        for line in [
            "pub screen_x: f32,",
            "screen_x: position.screen_x + dx,",
            "if a.screen_y == b.screen_y {",
            "let dx = screen_x - start_x;",
            "pub fn begin_drag(&mut self, screen_x: f32, screen_y: f32) {",
        ] {
            assert!(!is_screen_coord_write(line), "false hit: {line}");
        }
    }

    /// An assignment to a `screen_x`/`screen_y` binding — plain `=` or any
    /// compound operator. A comparison is not a write, and neither is a
    /// `screen_x:` field declaration, struct initialiser or parameter.
    fn is_screen_coord_write(line: &str) -> bool {
        for name in ["screen_x", "screen_y"] {
            let mut from = 0;
            while let Some(at) = line[from..].find(name) {
                let after = from + at + name.len();
                let rest = line[after..].trim_start();
                let op = rest.trim_start_matches(['+', '-', '*', '/']);
                if op.starts_with('=') && !op.starts_with("==") {
                    return true;
                }
                from = after;
            }
        }
        false
    }

    #[test]
    fn a_vehicle_never_bobs() {
        let mut entity = GameEntity::test_default(1, "MTNK", "Americans", 5, 5);
        entity.category = EntityCategory::Unit;
        let mut loco = LocomotorState::for_test_kind(LocomotorKind::Drive);
        loco.infantry_wobble_phase = std::f32::consts::PI;
        entity.locomotor = Some(loco);
        assert_eq!(height_lift_px(&entity), 0.0);
    }
}
