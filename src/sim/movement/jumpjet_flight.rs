//! Jumpjet flight kernel: the per-frame altitude and translation body shared by
//! every moving Jumpjet state, and the cruise state that steers it.
//!
//! gamemd-derived (disassembly read 2026-09-15; YR 1.001, SHA-256 `1cdd1180…4298c`):
//!
//! - `JumpjetLocomotionClass::Update_Coordinates_And_Altitude @ 0x0054D0F0`,
//!   called from `Process @ 0x0054AEC0` whenever the locomotor reports moving.
//!   It ramps the speed double toward the target speed, advances the hover bob,
//!   integrates height toward the bobbed target height over a terrain reference,
//!   zeroes the speed while too low outside the destination cell, then steps the
//!   owner along the locomotor's own facing and copies that facing to the body.
//! - `JumpjetLocomotionClass::State3_Translate @ 0x0054BFF0`: desired facing
//!   toward the destination through the retail atan table, four distance speed
//!   zones with a turn-error slowdown, target-height choice, and arrival below
//!   20 leptons.
//! - The reference height helper `0x0054D820` and the cell top height
//!   `CellClass @ 0x00485080` (ground at the cell centre, plus a building's
//!   `Dimension2` height or 85 leptons for any other techno in the cell).
//!
//! Parity demonstrated for the flat-map subset by
//! `tools/spatial_oracle/jumpjet_flight.json` (native Unicorn execution; see the
//! `.meta.json` scope): speed ramps, zones, turn slowdowns, bob, climb and
//! descent, the low-altitude speed gate, arrival into state 4 or a claimed hold.
//! Bridges, building tops and cell objects are Rust-tested only.
//!
//! Numeric model: `WinMain` installs x87 control word `0x0E7F` (53-bit
//! precision, round toward zero; `_controlfp(0x300, 0x300)` at `0x006BBFC1`),
//! and every double here is evaluated in native operand order with
//! [`X87Chop53`]. Sine, cosine and arctangent read the retail tables; the
//! distance uses `Sqrt_Approx`; `Math::ftol @ 0x007C5F00` keeps the low dword.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on map/retail_trig, rules/jumpjet_params, util and
//!   sim/movement only.

use crate::map::retail_trig::{AtanTable, TrigTable};
use crate::rules::jumpjet_params::JumpjetParams;
use crate::sim::movement::facing_class::FacingClass;
use crate::util::lepton::GROUND_LEVEL_HEIGHT_LEPTONS;
use crate::util::native_x87::{NativeF32Bits, NativeF64Bits, X87Chop53, X87Ordering, X87Value};

/// `[0x00ABC5DC]`, the bridge deck height.
pub(crate) const BRIDGE_DECK_LEPTONS: i32 = 416;
/// Arrival radius in leptons (`CMP EBX,0x14` at `0x0054C163`).
const ARRIVAL_RADIUS: i32 = 20;
/// Extra reference height for a non-building techno in a cell (`ADD EBP,0x55`
/// at `0x004850EF`).
const CELL_OBJECT_LIFT: i32 = 0x55;
/// `0x008223B0`: binary32 16384/2pi, the radians-to-units scale of
/// `Math::SinFromTable @ 0x004CACB0` and `CosFromTable @ 0x004CAD00`.
const TRIG_UNITS_PER_RADIAN_F32: u32 = 0x4522_F983;
/// `0x007E2810`: -2pi/65536.
const NEG_RADIANS_PER_FACING_UNIT: u64 = 0xBF19_222D_989F_5E57;
/// `0x007E2818`: -65536/2pi.
const NEG_FACING_UNITS_PER_RADIAN: u64 = 0xC0C4_5F07_AF68_ECEF;
/// `0x007E2820`: pi/2.
const HALF_PI: u64 = 0x3FF9_21FB_5444_2D18;
/// `0x007E3CC0`: 2pi.
const TWO_PI: u64 = 0x4019_21FB_5444_2D18;
/// `0x007ECE60`: 15.0.
const FIFTEEN: u64 = 0x402E_0000_0000_0000;
/// `0x007E48F0`: 1.5.
const ONE_AND_A_HALF: u64 = 0x3FF8_0000_0000_0000;
/// `0x007E7FC0`: 0.75.
const THREE_QUARTERS: u64 = 0x3FE8_0000_0000_0000;
/// `0x007E1718`: 1.0.
const ONE: u64 = 0x3FF0_0000_0000_0000;
/// `g_DirectionDelta` at `0x0089F6D8`, filled by `0x0049F3A0`: north first,
/// clockwise, one cell in leptons.
const DIRECTION_DELTA: [(i32, i32); 8] = [
    (0, -256),
    (256, -256),
    (256, 0),
    (256, 256),
    (0, 256),
    (-256, 256),
    (-256, 0),
    (-256, -256),
];

/// Native state byte `+0x50`.
pub(crate) const STATE_GROUND: i32 = 0;
pub(crate) const STATE_HOLD: i32 = 2;
pub(crate) const STATE_TRANSLATE: i32 = 3;
pub(crate) const STATE_DESCEND: i32 = 4;

/// The type block `Link_To_Object @ 0x0054AD30` copies into the locomotor
/// (receiver `+0x1C..+0x3C`). Floats keep their binary32 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct JumpjetFlightParams {
    /// `+0x1C` `JumpjetTurnRate=`: the facing rate and the turn-error threshold.
    pub turn_rate: i32,
    /// `+0x20` `JumpjetSpeed=` (an int).
    pub speed: i32,
    /// `+0x24` `JumpjetClimb=`.
    pub climb_bits: u32,
    /// `+0x28` `JumpjetCrash=`.
    pub crash_bits: u32,
    /// `+0x2C` `JumpjetHeight=`, floored at two cell levels (`0x0054AD9B`).
    pub height: i32,
    /// `+0x30` `JumpjetAccel=`.
    pub accel_bits: u32,
    /// `+0x34` `JumpjetWobbles=`.
    pub wobbles_bits: u32,
    /// `+0x38` `JumpjetDeviation=`.
    pub deviation: i32,
    /// `+0x3C` `JumpjetNoWobbles=`.
    pub no_wobbles: bool,
}

impl Default for JumpjetFlightParams {
    fn default() -> Self {
        Self::link(&JumpjetParams::default())
    }
}

impl JumpjetFlightParams {
    /// `Link_To_Object @ 0x0054AD30`'s straight-line copy.
    ///
    /// `JumpjetSpeed=` is an int natively; VERA's rules keep a fraction, which
    /// truncates here (the reader residual is recorded in `jumpjet_params.rs`).
    pub fn link(params: &JumpjetParams) -> Self {
        Self {
            turn_rate: params.turn_rate,
            speed: params.speed.to_num::<i32>(),
            climb_bits: params.climb.to_bits(),
            crash_bits: params.crash.to_bits(),
            height: params.height.max(2 * GROUND_LEVEL_HEIGHT_LEPTONS),
            accel_bits: params.accel.to_bits(),
            wobbles_bits: params.wobbles.to_bits(),
            deviation: params.deviation,
            no_wobbles: params.no_wobbles,
        }
    }

    /// The locomotor facing `Link_To_Object` builds (`FUN_004C91E0` then
    /// `Set`/`UpdateFacing` to `0x4000`): the rate is `min(rate, 127)` taken as a
    /// byte and shifted; a byte of `0x80` or more is a non-positive short rate,
    /// which the facing treats as instant.
    pub fn linked_facing(&self) -> FacingClass {
        let clamped = if self.turn_rate > 0x7E {
            0x7F
        } else {
            self.turn_rate
        };
        let byte = clamped as u8;
        FacingClass::new(0x4000, if byte >= 0x80 { 0 } else { byte })
    }
}

/// Flight fields of the locomotor (receiver `+0x54..+0x8C`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct JumpjetFlight {
    /// `+0x54` the locomotor's own facing; the body copies it every Update.
    pub facing: FacingClass,
    /// `+0x70` current speed, binary64 bits.
    pub current_speed_bits: u64,
    /// `+0x78` target speed, binary64 bits.
    pub target_speed_bits: u64,
    /// `+0x80` target height above the reference.
    pub target_height: i32,
    /// `+0x88` hover bob phase in radians, binary64 bits.
    pub bob_phase_bits: u64,
}

impl JumpjetFlight {
    pub fn linked(params: &JumpjetFlightParams) -> Self {
        Self {
            facing: params.linked_facing(),
            current_speed_bits: 0,
            target_speed_bits: 0,
            target_height: 0,
            bob_phase_bits: 0,
        }
    }

    pub fn current_speed(&self) -> f64 {
        f64::from_bits(self.current_speed_bits)
    }

    pub fn target_speed(&self) -> f64 {
        f64::from_bits(self.target_speed_bits)
    }
}

impl Default for JumpjetFlight {
    fn default() -> Self {
        Self::linked(&JumpjetFlightParams::default())
    }
}

/// RTTI of the owner as the kernel distinguishes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlightOwnerKind {
    Unit,
    Infantry,
    Other,
}

/// Everything the flight kernel reads from or writes to the owner and the map.
/// Coordinates are world leptons.
pub(crate) trait JumpjetFlightHost {
    fn binary_frame(&self) -> u32;
    fn trig(&self) -> &TrigTable;
    fn atan(&self) -> &AtanTable;
    fn owner_kind(&self) -> FlightOwnerKind;
    /// Owner `+0x9C` coordinate.
    fn location(&self) -> [i32; 3];
    /// `FootClass::SetLocation` (vtable `+0x1B4`).
    fn set_location(&mut self, coord: [i32; 3]);
    /// `ObjectClass::SetZ @ 0x005F6060`.
    fn set_z(&mut self, z: i32);
    /// `ObjectClass::GetHeight @ 0x005F5F40`.
    fn height_above_ground(&self) -> i32;
    /// Owner `+0x8C`, on a bridge deck.
    fn on_bridge(&self) -> bool;
    /// Update's grounded reset at `0x0054D407..D438`: vtable `+0xF4` with the
    /// location, then `+0x8C = 0`.
    fn grounded_reset(&mut self);
    /// `CellClass::GetGroundHeight @ 0x00578080` at a coordinate, deck excluded.
    fn floor_height(&self, xy: [i32; 2]) -> i32;
    /// The cell holding `xy` carries a high bridge (`+0x140 & 0x100`).
    fn cell_high_bridge(&self, xy: [i32; 2]) -> bool;
    /// `CellClass @ 0x00485080` for the cell holding `xy`.
    fn cell_top_height(&self, xy: [i32; 2]) -> i32;
    /// `CellClass+0xEC` LandType of the cell holding `xy`.
    fn cell_land_type(&self, xy: [i32; 2]) -> u8;
    /// `TechnoTypeClass+0xD6A` `BalloonHover=`.
    fn balloon_hover(&self) -> bool;
    /// Owner `+0x2B4` TarCom is set.
    fn has_target(&self) -> bool;
    /// Foot `+0x6AD`, the locomotor piggyback/deploy flag.
    fn piggyback_active(&self) -> bool;
    /// Arrival with the piggyback flag: owner `+0x2B0` then `0x0070FEE0(1)`.
    fn piggyback_arrival(&mut self);
    /// `UnitTypeClass+0xE13` `IsSimpleDeployer=` on a Unit owner.
    fn simple_deployer(&self) -> bool;
    /// `TechnoTypeClass+0x6AD` on a Unit owner.
    fn type_flag_6ad(&self) -> bool;
    /// `SetSpeedFraction` (vtable `+0x544`), with the binary64 argument.
    fn set_speed_fraction(&mut self, fraction_bits: u64);
    /// `0x00705D60` on the owner after an arrival transition.
    fn arrival_notify(&mut self);
    /// `0x004135A0`: the owner's cell AltObject is another object.
    fn air_slot_taken(&mut self) -> bool;
    /// `0x00487D70` on the owner's cell with the owner.
    fn claim_air_slot(&mut self);
    /// `RandomRanged(0,7)` neighbour and `Set_Destination` (vtable `+0x480`).
    fn scatter_to_random_neighbour(&mut self);
    /// `FacingClass::UpdateFacing` on the body facing (`+0x388`).
    fn snap_body_facing(&mut self, facing: u16);
    /// Infantry with `InfantryTypeClass+0xECB` holding a target in state 2:
    /// the facing `0x005F3DB0` answers toward the target.
    fn hold_target_facing(&self) -> Option<u16>;
}

fn zero() -> X87Value {
    X87Chop53::load_i32(0)
}

fn int(value: i32) -> X87Value {
    X87Chop53::load_i32(value)
}

fn double(bits: u64) -> X87Value {
    X87Chop53::load_f64(NativeF64Bits::from_bits(bits)).unwrap_or_else(|_| zero())
}

fn single(bits: u32) -> X87Value {
    X87Chop53::load_f32(NativeF32Bits::from_bits(bits)).unwrap_or_else(|_| zero())
}

fn store_double(value: X87Value) -> u64 {
    X87Chop53::store_f64(value).map_or(0, |bits| bits.bits())
}

/// `Math::ftol @ 0x007C5F00`: the low dword of `FISTP qword`; an operand outside
/// the signed 64-bit domain stores the integer indefinite, whose low dword is 0.
fn ftol(value: X87Value) -> i32 {
    X87Chop53::ftol_i64(value).map_or(0, |integer| integer as i32)
}

fn ordering(lhs: X87Value, rhs: X87Value) -> X87Ordering {
    X87Chop53::compare(lhs, rhs)
}

fn table_index(radians: X87Value) -> i32 {
    ftol(X87Chop53::mul(radians, single(TRIG_UNITS_PER_RADIAN_F32)))
}

fn table_sin(trig: &TrigTable, radians: X87Value) -> X87Value {
    single(trig.sin(table_index(radians)).to_bits())
}

fn table_cos(trig: &TrigTable, radians: X87Value) -> X87Value {
    single(trig.cos(table_index(radians)).to_bits())
}

fn native_cell(value: i32) -> i16 {
    (value.wrapping_add((value >> 31) & 0xFF) >> 8) as i16
}

/// The eight-way direction `0x0054D897..D8AC` derives from a 16-bit facing.
fn facing_direction(facing: u16) -> usize {
    ((((u32::from(facing) >> 12) + 1) >> 1) & 7) as usize
}

/// Reference height `0x0054D820`: the current cell's top height, and while
/// moving the max-or-average with the cell one step ahead along the facing.
///
/// Both bridge tests add the deck to the *current* cell's value (`ADD EDI` at
/// `0x0054D875` and `0x0054D906`), so a bridge ahead raises the current sample.
fn reference_height(
    flight: &JumpjetFlight,
    host: &impl JumpjetFlightHost,
    location: [i32; 3],
) -> i32 {
    let here_xy = [location[0], location[1]];
    let mut here = host.cell_top_height(here_xy);
    if host.cell_high_bridge(here_xy) {
        here = here.wrapping_add(BRIDGE_DECK_LEPTONS);
    }
    if ordering(double(flight.current_speed_bits), zero()) != X87Ordering::Greater {
        return here;
    }
    let (dx, dy) = DIRECTION_DELTA[facing_direction(flight.facing.current(host.binary_frame()))];
    let ahead_xy = [location[0].wrapping_add(dx), location[1].wrapping_add(dy)];
    let ahead = host.cell_top_height(ahead_xy);
    if host.cell_high_bridge(ahead_xy) {
        here = here.wrapping_add(BRIDGE_DECK_LEPTONS);
    }
    if ahead > here {
        ahead
    } else {
        ahead.wrapping_add(here) / 2
    }
}

/// `Update_Coordinates_And_Altitude @ 0x0054D0F0`.
pub(crate) fn update_coordinates_and_altitude(
    phase: i32,
    destination: [i32; 3],
    params: &JumpjetFlightParams,
    flight: &mut JumpjetFlight,
    host: &mut impl JumpjetFlightHost,
) {
    let hold_or_translate = matches!(phase, STATE_HOLD | STATE_TRANSLATE);

    // Speed ramp 0x0054D138..D1AE. The acceleration test runs first and the
    // deceleration test reads the updated value, so a target below the cap can
    // be overshot and pulled back in the same frame.
    let cap = int(params.speed);
    let target = double(flight.target_speed_bits);
    let mut current = double(flight.current_speed_bits);
    if ordering(target, current) == X87Ordering::Greater {
        let next = X87Chop53::add(single(params.accel_bits), current);
        current = if ordering(next, cap) == X87Ordering::Less {
            next
        } else {
            cap
        };
    }
    if ordering(target, current) == X87Ordering::Less {
        let braking = X87Chop53::mul(single(params.accel_bits), double(ONE_AND_A_HALF));
        let next = X87Chop53::sub(current, braking);
        current = if ordering(next, zero()) == X87Ordering::Greater {
            next
        } else {
            zero()
        };
    }
    flight.current_speed_bits = store_double(current);
    // A zero speed divides by zero natively (masked, infinity or NaN); no stock
    // jumpjet has one, and VERA reports a zero fraction instead.
    host.set_speed_fraction(X87Chop53::div(current, cap).map_or(0, store_double));

    let location = host.location();
    let same_cell = native_cell(destination[0]) == native_cell(location[0])
        && native_cell(destination[1]) == native_cell(location[1]);

    // Hover bob 0x0054D1FB..D23A, then the bobbed target height.
    let bob = if hold_or_translate && !params.no_wobbles {
        let step = X87Chop53::div(double(FIFTEEN), single(params.wobbles_bits))
            .and_then(|period| X87Chop53::div(double(TWO_PI), period))
            .unwrap_or_else(|_| zero());
        X87Chop53::add(step, double(flight.bob_phase_bits))
    } else {
        zero()
    };
    flight.bob_phase_bits = store_double(bob);
    let bob_target = ftol(X87Chop53::add(
        X87Chop53::mul(table_sin(host.trig(), bob), int(params.deviation)),
        int(flight.target_height),
    ));

    // Ground under the owner, raised to the deck once near it (0x0054D29A..D314).
    let z = location[2];
    let xy = [location[0], location[1]];
    let mut ground = host.floor_height(xy);
    if host.cell_high_bridge(xy) {
        let deck_approach = X87Chop53::sub(
            int(ground.wrapping_add(4 * GROUND_LEVEL_HEIGHT_LEPTONS)),
            single(params.crash_bits),
        );
        if ordering(deck_approach, int(z)) != X87Ordering::Greater {
            ground = ground.wrapping_add(BRIDGE_DECK_LEPTONS);
        }
    }
    let reference =
        if matches!(phase, STATE_DESCEND | STATE_GROUND) || (same_cell && !host.balloon_hover()) {
            ground
        } else {
            reference_height(flight, host, location)
        };

    // Height integration 0x0054D354..D4BB.
    let climb = single(params.climb_bits);
    let mut altitude = z.wrapping_sub(reference);
    let mut new_z = None;
    if altitude < bob_target {
        let mut height = host.height_above_ground();
        if host.cell_high_bridge(xy)
            && !host.on_bridge()
            && z >= host.floor_height(xy).wrapping_add(BRIDGE_DECK_LEPTONS)
        {
            height = height.wrapping_sub(BRIDGE_DECK_LEPTONS);
        }
        if height == 0 {
            host.grounded_reset();
        }
        let reach = X87Chop53::add(int(altitude), climb);
        new_z = Some(if ordering(int(bob_target), reach) == X87Ordering::Less {
            z.wrapping_add(bob_target.wrapping_sub(altitude))
        } else {
            ftol(X87Chop53::add(int(z), climb))
        });
    } else if altitude > bob_target {
        let floor = X87Chop53::sub(int(altitude), climb);
        let mut next = if ordering(int(bob_target), floor) != X87Ordering::Greater {
            ftol(X87Chop53::sub(int(z), climb))
        } else {
            z.wrapping_add(bob_target.wrapping_sub(altitude))
        };
        if next <= ground {
            next = ground;
        }
        if altitude <= 0 {
            altitude = 0;
        }
        new_z = Some(next);
    }

    // Too low outside the destination cell: no horizontal speed (0x0054D4FD..D52D).
    if !same_cell && (altitude < bob_target / 2 || altitude < bob_target / 4) {
        flight.current_speed_bits = 0;
    }
    if let Some(next) = new_z {
        host.set_z(next);
    }

    // Horizontal step along the locomotor facing (0x0054D55A..D607).
    let location = host.location();
    let frame = host.binary_frame();
    let step = int(ftol(double(flight.current_speed_bits)));
    let facing = flight.facing.current(frame) as i16;
    let angle = X87Chop53::mul(
        int(i32::from(facing) - 0x3FFF),
        double(NEG_RADIANS_PER_FACING_UNIT),
    );
    let new_y = ftol(X87Chop53::sub(
        int(location[1]),
        X87Chop53::mul(table_sin(host.trig(), angle), step),
    ));
    let new_x = ftol(X87Chop53::add(
        X87Chop53::mul(table_cos(host.trig(), angle), step),
        int(location[0]),
    ));
    host.set_location([new_x, new_y, location[2]]);

    // Body facing (0x0054D60D..D692).
    if !host.piggyback_active() {
        let hold_facing = (host.owner_kind() == FlightOwnerKind::Infantry
            && phase == STATE_HOLD
            && host.has_target())
        .then(|| host.hold_target_facing())
        .flatten();
        match hold_facing {
            Some(toward_target) => {
                flight.facing.snap(toward_target, frame);
            }
            None => host.snap_body_facing(flight.facing.current(frame)),
        }
    }
}

/// `State3_Translate @ 0x0054BFF0`. Returns the new state.
pub(crate) fn state3_translate(
    destination: [i32; 3],
    params: &JumpjetFlightParams,
    flight: &mut JumpjetFlight,
    host: &mut impl JumpjetFlightHost,
) -> i32 {
    let mut state = STATE_TRANSLATE;
    let frame = host.binary_frame();
    let location = host.location();

    // Desired facing toward the destination (0x0054C081..C0CD).
    let angle = host.atan().atan2(
        X87Chop53::sub(int(location[1]), int(destination[1])),
        X87Chop53::sub(int(destination[0]), int(location[0])),
    );
    let desired = ftol(X87Chop53::mul(
        X87Chop53::sub(angle, double(HALF_PI)),
        double(NEG_FACING_UNITS_PER_RADIAN),
    )) as u16;
    flight.facing.set(desired, frame);
    // `FUN_004C9530` is destination minus animated current; the error byte
    // rounds the unsigned 16-bit difference to eighths of a byte, so any turn
    // to the left reads as a large error.
    let difference = flight
        .facing
        .destination()
        .wrapping_sub(flight.facing.current(frame));
    let turn_error = ((((u32::from(difference) >> 7) + 1) >> 1) & 0xFF) as i32;

    let distance = crate::sim::cell_kernel::native_xy_distance(
        location[0].wrapping_sub(destination[0]),
        location[1].wrapping_sub(destination[1]),
    );
    let unit_owner = host.owner_kind() == FlightOwnerKind::Unit;
    let has_target = host.has_target();

    if distance < ARRIVAL_RADIUS {
        flight.current_speed_bits = 0;
        flight.target_speed_bits = 0;
        host.set_location([destination[0], destination[1], location[2]]);
        if host.piggyback_active() {
            flight.target_height = 0;
            host.piggyback_arrival();
            state = STATE_DESCEND;
        } else if !has_target && !host.balloon_hover() {
            if unit_owner && host.simple_deployer() && host.type_flag_6ad() {
                if host.air_slot_taken() {
                    host.scatter_to_random_neighbour();
                } else {
                    host.claim_air_slot();
                    flight.target_height = params.height;
                    state = STATE_HOLD;
                }
            } else {
                flight.target_height = 0;
                state = STATE_DESCEND;
            }
            host.arrival_notify();
        } else if host.air_slot_taken() {
            host.scatter_to_random_neighbour();
        } else {
            host.claim_air_slot();
            state = STATE_HOLD;
        }
    } else {
        let speed = params.speed;
        // `FCOMP 1.0` then store 1.0 when below (`0x0054C432`, `0x0054C4CC`).
        let at_least_one = |value: i32| {
            if ordering(int(value), double(ONE)) == X87Ordering::Less {
                ONE
            } else {
                store_double(int(value))
            }
        };
        let target_bits = if distance < speed {
            if !has_target {
                flight.target_height = params.height / 2;
            }
            store_double(int(speed / 8))
        } else if distance < speed.wrapping_mul(2) {
            if !has_target {
                flight.target_height = params.height / 2;
            }
            if turn_error > params.turn_rate {
                at_least_one(speed / 10)
            } else {
                store_double(int(speed / 4))
            }
        } else if speed
            .wrapping_mul(50)
            .checked_div(params.turn_rate)
            .is_some_and(|slow_radius| distance < slow_radius)
        {
            // A zero turn rate faults natively (`IDIV` at `0x0054C45E`); no
            // stock type reaches it, and VERA skips the zone instead.
            if !has_target {
                flight.target_height =
                    ftol(X87Chop53::mul(int(params.height), double(THREE_QUARTERS)));
            }
            if turn_error > params.turn_rate.wrapping_mul(5) {
                at_least_one(speed / 5)
            } else {
                store_double(int(speed / 2))
            }
        } else {
            flight.target_height = params.height;
            store_double(int(speed))
        };
        flight.target_speed_bits = target_bits;
    }

    // Tail 0x0054C4FD..C544: hovering types, water and beach destinations and
    // flagged units keep full height.
    if host.balloon_hover()
        || matches!(host.cell_land_type([destination[0], destination[1]]), 2 | 6)
        || (unit_owner && host.type_flag_6ad())
    {
        flight.target_height = params.height;
    }
    state
}

/// `CellClass @ 0x00485080`: ground at the cell centre, plus the first
/// building's `BuildingTypeClass::Dimension2` height (vtable `+0x7C`,
/// `0x00464AF0`) or, with no building, 85 leptons when
/// `Find_Nearest_Object @ 0x0047C3D0` finds any techno in the ground list.
pub(crate) fn cell_top_height(
    centre_ground: i32,
    first_building_height: Option<i32>,
    any_techno: bool,
) -> i32 {
    match first_building_height {
        Some(height) => centre_ground.wrapping_add(height),
        None if any_techno => centre_ground.wrapping_add(CELL_OBJECT_LIFT),
        None => centre_ground,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::retail_trig::{required_atan_table, required_math_tables};
    use crate::util::fixed_math::SimFixed;
    use serde_json::{Value, json};

    /// The oracle's map fixture: cell (10,10) is level 0 and flat, cell (9,10)
    /// is level 2 with slope 1 (the shared `Original` harness), and every other
    /// lookup is the zero-height dummy cell. No cell has a bridge, building or
    /// object, and the owner flags are clear.
    struct FixtureHost<'a> {
        frame: u32,
        trig: &'a TrigTable,
        atan: &'a AtanTable,
        kind: FlightOwnerKind,
        location: [i32; 3],
        balloon_hover: bool,
        has_target: bool,
        fractions: Vec<u64>,
        events: Vec<&'static str>,
        /// Cell top heights that replace the terrain sample (buildings, objects).
        tops: Vec<((i16, i16), i32)>,
        /// Cells carrying a high bridge.
        bridges: Vec<(i16, i16)>,
        piggyback: bool,
        simple_deployer: bool,
        deploy_to_land: bool,
        /// LandType of fixture cell (10,10).
        destination_land_type: u8,
        /// Owner body facing `+0x388`, written by Update's `UpdateFacing`.
        body_facing: u16,
    }

    impl FixtureHost<'_> {
        fn cell_terrain(xy: [i32; 2]) -> (u8, u8) {
            if (native_cell(xy[0]), native_cell(xy[1])) == (9, 10) {
                (2, 1)
            } else {
                (0, 0)
            }
        }
    }

    impl JumpjetFlightHost for FixtureHost<'_> {
        fn binary_frame(&self) -> u32 {
            self.frame
        }
        fn trig(&self) -> &TrigTable {
            self.trig
        }
        fn atan(&self) -> &AtanTable {
            self.atan
        }
        fn owner_kind(&self) -> FlightOwnerKind {
            self.kind
        }
        fn location(&self) -> [i32; 3] {
            self.location
        }
        fn set_location(&mut self, coord: [i32; 3]) {
            self.location = coord;
        }
        fn set_z(&mut self, z: i32) {
            self.location[2] = z;
        }
        fn height_above_ground(&self) -> i32 {
            self.location[2] - self.floor_height([self.location[0], self.location[1]])
        }
        fn on_bridge(&self) -> bool {
            false
        }
        fn grounded_reset(&mut self) {
            self.events.push("grounded_reset");
        }
        fn floor_height(&self, xy: [i32; 2]) -> i32 {
            let (level, slope) = Self::cell_terrain(xy);
            crate::util::lepton::ground_height_leptons(level, slope, xy[0], xy[1])
                .expect("fixture slope is supported")
        }
        fn cell_high_bridge(&self, xy: [i32; 2]) -> bool {
            self.bridges
                .contains(&(native_cell(xy[0]), native_cell(xy[1])))
        }
        fn cell_top_height(&self, xy: [i32; 2]) -> i32 {
            let cell = (native_cell(xy[0]), native_cell(xy[1]));
            if let Some(&(_, top)) = self.tops.iter().find(|(key, _)| *key == cell) {
                return top;
            }
            let (level, slope) = Self::cell_terrain(xy);
            let centre = crate::util::lepton::ground_height_leptons(level, slope, 128, 128)
                .expect("fixture slope is supported");
            cell_top_height(centre, None, false)
        }
        fn cell_land_type(&self, xy: [i32; 2]) -> u8 {
            if (native_cell(xy[0]), native_cell(xy[1])) == (10, 10) {
                self.destination_land_type
            } else {
                0
            }
        }
        fn balloon_hover(&self) -> bool {
            self.balloon_hover
        }
        fn has_target(&self) -> bool {
            self.has_target
        }
        fn piggyback_active(&self) -> bool {
            self.piggyback
        }
        fn piggyback_arrival(&mut self) {}
        fn simple_deployer(&self) -> bool {
            self.simple_deployer
        }
        fn type_flag_6ad(&self) -> bool {
            self.deploy_to_land
        }
        fn set_speed_fraction(&mut self, fraction_bits: u64) {
            self.fractions.push(fraction_bits);
        }
        fn arrival_notify(&mut self) {
            self.events.push("mission_notify");
        }
        fn air_slot_taken(&mut self) -> bool {
            self.events.push("slot_query");
            false
        }
        fn claim_air_slot(&mut self) {
            self.events.push("slot_claim");
        }
        fn scatter_to_random_neighbour(&mut self) {
            self.events.push("set_destination");
        }
        fn snap_body_facing(&mut self, facing: u16) {
            self.body_facing = facing;
        }
        fn hold_target_facing(&self) -> Option<u16> {
            None
        }
    }

    fn int(value: &Value) -> i64 {
        value.as_i64().expect("integer field")
    }

    /// Parity with `tools/spatial_oracle/jumpjet_flight.json`: native Update
    /// `0x0054D0F0` then State3 `0x0054BFF0` per frame, every field bit-exact.
    #[test]
    fn flight_matches_the_native_update_and_translate_corpus() {
        let (trig, _) = required_math_tables();
        let atan = required_atan_table();
        if !trig.matches_retail() || !atan.matches_retail() {
            // With RA2_DIR set, a mismatched table is a failure, not a skip.
            assert!(
                std::env::var_os("RA2_DIR").is_none(),
                "RA2_DIR is set but the retail sine or atan table does not match"
            );
            eprintln!("skipped: set RA2_DIR to the retail install to run this");
            return;
        }
        let rows: Value = serde_json::from_str(include_str!(
            "../../../tools/spatial_oracle/jumpjet_flight.json"
        ))
        .expect("corpus parses");
        let rows = rows.as_array().expect("row list");
        assert_eq!(rows.len(), 14);
        for row in rows {
            let name = row["name"].as_str().expect("row name");
            let input = &row["input"];
            let float = |key: &str| input[key].as_f64().expect("float field") as f32;
            let type_params = JumpjetParams {
                turn_rate: int(&input["turn_rate"]) as i32,
                speed: SimFixed::from_num(int(&input["speed"])),
                climb: float("climb"),
                crash: float("crash"),
                height: int(&input["height"]) as i32,
                accel: float("accel"),
                wobbles: float("wobbles"),
                deviation: int(&input["deviation"]) as i32,
                no_wobbles: input["no_wobbles"].as_bool().expect("flag"),
            };
            let params = JumpjetFlightParams::link(&type_params);
            let first_frame = int(&input["first_frame"]) as u32;
            let mut flight = JumpjetFlight::linked(&params);
            flight
                .facing
                .snap(int(&input["facing"]) as u16, first_frame);
            flight.target_height = int(&input["target_height"]) as i32;
            let start = &input["start"];
            let destination = [
                int(&input["destination"][0]) as i32,
                int(&input["destination"][1]) as i32,
                0,
            ];
            let mut host = FixtureHost {
                frame: first_frame,
                trig,
                atan,
                kind: if int(&input["rtti"]) == 15 {
                    FlightOwnerKind::Infantry
                } else {
                    FlightOwnerKind::Unit
                },
                location: [
                    int(&start[0]) as i32,
                    int(&start[1]) as i32,
                    int(&start[2]) as i32,
                ],
                balloon_hover: input["balloon_hover"].as_bool().expect("flag"),
                has_target: input["tarcom"].as_bool().expect("flag"),
                fractions: Vec::new(),
                events: Vec::new(),
                tops: Vec::new(),
                bridges: Vec::new(),
                piggyback: input["piggyback"].as_bool().expect("flag"),
                simple_deployer: input["simple_deployer"].as_bool().expect("flag"),
                deploy_to_land: input["deploy_to_land"].as_bool().expect("flag"),
                destination_land_type: int(&input["destination_land_type"]) as u8,
                body_facing: 0,
            };
            let expected = row["output"]["frames"].as_array().expect("frames");
            let mut phase = STATE_TRANSLATE;
            let mut produced = 0;
            for _ in 0..int(&input["max_frames"]) {
                host.fractions.clear();
                host.events.clear();
                host.frame += 1;
                update_coordinates_and_altitude(
                    phase,
                    destination,
                    &params,
                    &mut flight,
                    &mut host,
                );
                if phase == STATE_TRANSLATE {
                    phase = state3_translate(destination, &params, &mut flight, &mut host);
                }
                let frame = json!({
                    "coord": host.location,
                    "current_speed": flight.current_speed_bits,
                    "target_speed": flight.target_speed_bits,
                    "target_height": flight.target_height,
                    "bob_phase": flight.bob_phase_bits,
                    "phase": phase,
                    "facing_current": flight.facing.current(host.frame),
                    "facing_destination": flight.facing.destination(),
                    "body_facing": host.body_facing,
                    "speed_fractions": host.fractions,
                    "events": host.events,
                });
                assert_eq!(
                    Some(&frame),
                    expected.get(produced),
                    "{name}: frame {produced} differs"
                );
                produced += 1;
                if phase != STATE_TRANSLATE {
                    break;
                }
            }
            assert_eq!(produced, expected.len(), "{name}: frame count");
        }
    }

    fn edge_host(
        trig: &'static TrigTable,
        atan: &'static AtanTable,
        tops: Vec<((i16, i16), i32)>,
        bridges: Vec<(i16, i16)>,
    ) -> FixtureHost<'static> {
        FixtureHost {
            frame: 10,
            trig,
            atan,
            kind: FlightOwnerKind::Unit,
            // Cell (3,3) centre, far from the terrain fixture's sloped cell.
            location: [3 * 256 + 128, 3 * 256 + 128, 500],
            balloon_hover: false,
            has_target: false,
            fractions: Vec::new(),
            events: Vec::new(),
            tops,
            bridges,
            piggyback: false,
            simple_deployer: false,
            deploy_to_land: false,
            destination_land_type: 0,
            body_facing: 0,
        }
    }

    fn eastbound_flight(speed: f64) -> JumpjetFlight {
        let mut flight = JumpjetFlight::linked(&JumpjetFlightParams::default());
        flight.facing.snap(0x4000, 0);
        flight.current_speed_bits = speed.to_bits();
        flight
    }

    /// `0x0054D820`: both bridge tests add the deck to the *current* cell's
    /// sample, so a bridge one cell ahead raises the current value, and the
    /// flat cell ahead then averages with it.
    #[test]
    fn a_bridge_ahead_raises_the_current_reference_sample() {
        let (trig, _) = required_math_tables();
        let host = edge_host(trig, required_atan_table(), Vec::new(), vec![(4, 3)]);
        let location = host.location;
        assert_eq!(
            reference_height(&eastbound_flight(4.0), &host, location),
            (0 + BRIDGE_DECK_LEPTONS) / 2
        );
        // Stationary: no look-ahead, and the current cell has no bridge.
        assert_eq!(reference_height(&eastbound_flight(0.0), &host, location), 0);
    }

    /// While moving, a higher cell one step ahead along the facing wins.
    #[test]
    fn a_higher_cell_ahead_sets_the_reference() {
        let (trig, _) = required_math_tables();
        let host = edge_host(trig, required_atan_table(), vec![((4, 3), 300)], Vec::new());
        let location = host.location;
        assert_eq!(
            reference_height(&eastbound_flight(4.0), &host, location),
            300
        );
        // A lower cell ahead averages instead: (100 + 300) / 2 from the current cell.
        let host = edge_host(
            trig,
            required_atan_table(),
            vec![((3, 3), 300), ((4, 3), 100)],
            Vec::new(),
        );
        assert_eq!(
            reference_height(&eastbound_flight(4.0), &host, location),
            200
        );
    }

    /// `CellClass @ 0x00485080`: a building's `Dimension2` height is used ahead
    /// of the 85-lepton lift for any other techno.
    #[test]
    fn cell_top_height_prefers_a_building_over_the_object_lift() {
        assert_eq!(cell_top_height(10, Some(208), true), 218);
        assert_eq!(cell_top_height(10, None, true), 95);
        assert_eq!(cell_top_height(10, None, false), 10);
    }

    /// State3's turn error rounds the unsigned 16-bit difference, so a quarter
    /// turn left reads 192 and a quarter turn right reads 64.
    #[test]
    fn a_left_turn_reads_as_a_larger_error_than_the_same_right_turn() {
        let error = |difference: u16| ((((u32::from(difference) >> 7) + 1) >> 1) & 0xFF) as i32;
        assert_eq!(error(0x4000), 64);
        assert_eq!(error(0xC000), 192);
        assert_eq!(error(0xFFFF), 0);
    }
}
