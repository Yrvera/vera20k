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
//! ## The height lift
//!
//! gamemd applies the height lift **once**, inlined in `CoordsToClient`, as
//! part of the projection itself — every drawn thing goes through it:
//!
//! ```text
//! screen_y -= ftol(Z_leptons * k + (Z_leptons >= 728 ? 1 : 0) + 0.5)
//! ```
//!
//! The multiplier `k` is not a literal in the image. It is computed once at
//! startup from the camera model, and the global holding it reads as all
//! zeroes in the file (BSS) — which is why it took chasing three globals to
//! their single writers to recover:
//!
//! ```text
//! k = sin(60°) * 60.0 / (256 * sqrt(2))  ≈  0.14352
//! ```
//!
//! — the tile width in pixels spread over the cell diagonal in leptons, scaled
//! by the sine of the camera elevation angle.
//!
//! Verified via `decompile_function 0x006d1bdd` for the product,
//! `disassemble_bytes 0x006d1830` and `0x006d18c0` for the two operands, and
//! `read_memory` on `0x007e1708` / `0x007e1710` / `0x007e1728` / `0x007f4180` /
//! `0x007f4188` for the literals (2.0, 256.0, 60.0, 60.0, π/180). Each of the
//! three globals has exactly one writer, confirmed by `get_xrefs_to`. The sine
//! helper at `0x004cacb0` takes **radians**: it scales its argument by
//! `[0x008223b0]` = 2607.7 ≈ 16384/2π before indexing an 8192-entry table whose
//! `[0]` is 0.0 — a cosine table would start at 1.0, so the identification is
//! not ambiguous. That scaling step was missing from the helper's own plate
//! comment, which made the argument look like table units; corrected there.
//!
//! ### What this replaced
//!
//! Until this landed the lift was applied in two layers — the air movement pass
//! wrote `screen_y` with it, and then every app draw path subtracted it again —
//! so airborne units were lifted **twice**, at an effective 0.12 px/lepton,
//! while parachutes and scripted missiles got a single 0.06. Two different
//! scales for one quantity, and neither was gamemd's. At YR's stock
//! `FlightLevel=1500` an aircraft now sits ~215px up instead of ~180px, and a
//! paradrop or missile roughly 2.4× higher than before.
//!
//! Frequency: every airborne unit in every match, for as long as it is
//! airborne. Kirovs, Harriers, Rocketeers, Nighthawks, every jumpjet, every
//! paradrop and every missile in flight.
//!
//! ## Dependency rules
//! - Part of render/ — reads sim/ state read-only and writes none of it.
//! - sim/ NEVER depends on render/, so nothing here may be called from sim/.

use crate::map::entities::EntityCategory;
use crate::rules::locomotor_type::LocomotorKind;
use crate::sim::game_entity::GameEntity;
use crate::sim::movement::locomotor::MovementLayer;
use crate::util::fixed_math::sim_to_f32;

/// Upward screen lift per lepton of height. One value for every kind of
/// height — an air locomotor's altitude, a parachute's, a missile's — because
/// gamemd runs them all through the same projection.
///
/// `sin(60°) * 60.0 / (256 * sqrt(2))`. The algebraically exact product is
/// 0.1435237; the value here is 0.1435032, which is what the binary actually
/// computes once the sine comes off its 8192-entry table (index 1365 →
/// sin(1.046947) = 0.865898 rather than sin(π/3) = 0.866025). The 0.014% gap
/// is 0.03px at cruise altitude — the table value is used because it is the
/// one gamemd uses, not because the difference could matter.
pub const HEIGHT_LIFT_PX_PER_LEPTON: f32 = 0.143_503_2;

/// Height at or above which gamemd's `AdjustForZ` adds one extra pixel of lift
/// (`CMP ECX, 0x2D8` + `JL`, verified via `disassemble_function 0x006D20E0`).
///
/// Stock `FlightLevel=1500` clears this comfortably, so every cruising aircraft
/// gets the extra pixel; a parachute only picks it up early in its descent.
const EXTRA_PIXEL_HEIGHT_LEPTONS: f32 = 728.0;

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

/// This entity's height above the ground, in leptons.
///
/// Whichever state is carrying it — see [`HeightSource`] for why the order is
/// what it is.
fn height_leptons(entity: &GameEntity) -> f32 {
    match height_source(entity) {
        HeightSource::Parachute => entity
            .parachute_state
            .as_ref()
            .map(|state| sim_to_f32(state.altitude))
            .unwrap_or(0.0),
        HeightSource::Rocket => entity
            .rocket_state
            .as_ref()
            .map(|state| sim_to_f32(state.altitude))
            .unwrap_or(0.0),
        HeightSource::AirLocomotor => entity
            .locomotor
            .as_ref()
            .map(|loco| sim_to_f32(loco.altitude))
            .unwrap_or(0.0),
        HeightSource::Ground => 0.0,
    }
}

/// Total upward screen lift for an entity, in pixels.
///
/// Positive lifts the entity toward the top of the screen (screen Y decreases).
///
/// The height term reproduces gamemd's `AdjustForZ` exactly, quantisation
/// included: the scale, the extra pixel above [`EXTRA_PIXEL_HEIGHT_LEPTONS`],
/// and the `+ 0.5` truncate that rounds it to a whole pixel. The engine's own
/// blitter is integer-pixel, so a climbing aircraft steps a pixel at a time
/// there too. The infantry bob is *not* quantised — it is a VERA-internal
/// flourish with no gamemd equivalent, and rounding a ±1px sine to whole pixels
/// would turn it into a square wave.
pub fn height_lift_px(entity: &GameEntity) -> f32 {
    if height_source(entity) == HeightSource::Ground {
        return infantry_bob_px(entity);
    }
    let leptons = height_leptons(entity);
    let extra = if leptons >= EXTRA_PIXEL_HEIGHT_LEPTONS {
        1.0
    } else {
        0.0
    };
    // `ftol` truncates toward zero, so `+ 0.5` is gamemd's rounding.
    (leptons * HEIGHT_LIFT_PX_PER_LEPTON + extra + 0.5).trunc()
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

// A HALF-TILE TRAP, measured 2026-08-05. Do not "fix" a building's draw
// position here on its own — it was tried and it made things worse.
//
// gamemd really does give buildings their own answer to "where do I draw": of
// every class, only `BuildingClass` overrides that virtual, and its override
// takes half a cell off both coordinate axes, moving the anchor from the centre
// of the north-west footprint cell to that cell's corner. Equal steps on both
// axes cancel horizontally and come to exactly half a tile up.
//
// That is a true statement about gamemd and applying it here still broke the
// picture, because the frame underneath it is already wrong by the same amount
// in the other direction. Measured, relative rows for one cell:
//
//     layer            gamemd              VERA
//     terrain tile     box top             box top          agrees
//     ore / overlay    diamond centre      diamond centre   agrees
//     unit / vehicle   diamond centre      HALF A TILE UP   wrong
//     building         box top             box top          agrees
//
// So buildings and terrain were already right relative to each other, and the
// broken layer is the units — drawn half a tile north of the ground they stand
// on. Adding the building shift on top double-counted it and left every
// building floating.
//
// The real fix is to move the ENTITY anchor down half a tile to the cell's
// diamond centre, in `util::lepton::lepton_to_screen` and its `terrain` twin,
// after which this override becomes correct and necessary. That moves every
// unit on the map and several constants that bridge the entity and tile frames
// depend on the current relation, so it is its own piece of work.


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
    /// twice. At YR's stock `FlightLevel=1500` a cruising aircraft sits
    /// `trunc(1500 * 0.1435032 + 1 + 0.5)` = 216px up — the `+1` because 1500
    /// clears the extra-pixel threshold.
    #[test]
    fn a_cruising_aircraft_sits_at_gamemds_height() {
        let entity = air_unit(LocomotorKind::Fly, 1500);
        let (_, ground_sy) = ground_screen_position(&entity.position);
        let (_, sy) = screen_position(&entity);
        assert_eq!(ground_sy - sy, 216.0);
    }

    /// Every kind of height goes through one scale. A parachute and an aircraft
    /// at the same altitude must be drawn at the same place — before this they
    /// differed by a factor of two.
    #[test]
    fn every_height_source_uses_the_same_scale() {
        use crate::sim::entity_store::EntityStore;
        use crate::sim::movement::parachute_descent::begin_parachute_descent;

        let aircraft = air_unit(LocomotorKind::Fly, 400);

        let mut entities = EntityStore::default();
        let mut infantry = GameEntity::test_default(1, "E1", "Americans", 5, 5);
        infantry.category = EntityCategory::Infantry;
        entities.insert(infantry);
        assert!(begin_parachute_descent(
            &mut entities,
            1,
            SimFixed::from_num(400)
        ));

        assert_eq!(
            height_lift_px(&aircraft),
            height_lift_px(entities.get(1).expect("entity")),
        );
    }

    /// The extra pixel above the threshold is gamemd's, not ours: one lepton
    /// either side of 728 must differ by more than the scale alone accounts for.
    #[test]
    fn the_extra_pixel_switches_on_at_the_threshold() {
        let below = height_lift_px(&air_unit(LocomotorKind::Fly, 727));
        let at = height_lift_px(&air_unit(LocomotorKind::Fly, 728));
        assert_eq!(below, 104.0, "trunc(727 * k + 0.5)");
        assert_eq!(at, 105.0, "trunc(728 * k + 1 + 0.5) — the threshold fires");
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
        // trunc(400 * 0.1435032 + 0.5) — the parachute's 400, not the
        // locomotor's 1500, and below the extra-pixel threshold.
        assert_eq!(
            ground_sy - sy,
            57.0,
            "the parachute altitude must be the only height source"
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
