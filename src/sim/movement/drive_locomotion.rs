//! DriveLocomotion runtime helpers.
//!
//! This module owns Drive-specific state updates that should not leak into the
//! generic `MovementTarget` path. Detailed DriveTrack consumption remains in
//! `drive_track`; this file handles the Drive-local speed fraction scaffold.

use std::collections::BTreeMap;

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::{LocomotorKind, SpeedType};
use crate::rules::object_type::{ObjectCategory, ObjectType};
use crate::rules::ruleset::RuleSet;
use crate::sim::components::{
    DriveCoord, DriveLocomotionRuntime, NavTargetRef, ShipLocomotionRuntime,
};
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::house_state::HouseState;
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::pathfinding::terrain_speed::TerrainSpeedConfig;
use crate::util::fixed_math::SimFixed;
use crate::util::native_x87::{NativeF32Bits, NativeF64Bits, X87Chop53, X87Ordering};

const DRIVE_DESTINATION_BRAKE_FLOOR: NativeF64Bits =
    NativeF64Bits::from_bits(0x3fd3_3333_4000_0000);
const DRIVE_ALTERNATE_BRAKE_RATE: NativeF64Bits =
    NativeF64Bits::from_bits(0x3f58_9374_c000_0000);
const DRIVE_ALTERNATE_BRAKE_FLOOR: NativeF64Bits =
    NativeF64Bits::from_bits(0x3fb9_9999_a000_0000);
const DRIVE_CRUSH_FRACTION: NativeF64Bits =
    NativeF64Bits::from_bits(0x3fc9_9999_9999_999a);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct VehicleRampFlags {
    /// UnitTypeClass `Passive +0xE0C`.
    pub(super) passive: bool,
    /// Owner `+0x3CD`, the alternate out-of-arrival-band brake gate.
    pub(super) alternate_brake: bool,
    /// UnitClass `CurrentlyCrushing +0x6B5`.
    pub(super) currently_crushing: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DriveProcessOutcome {
    NotDrive,
    Processed,
}

pub(crate) fn process_drive_locomotion_shell(entity: &GameEntity) -> DriveProcessOutcome {
    if entity.drive_locomotion.is_none() {
        return DriveProcessOutcome::NotDrive;
    }
    DriveProcessOutcome::Processed
}

pub(super) fn drive_requires_native_step(drive: &DriveLocomotionRuntime) -> bool {
    !drive.path.directions.is_empty() || drive.residual_budget != 0
}

/// `ILocomotion::Is_Moving` (slot 4) for the Drive locomotor.
///
/// gamemd reads the locomotor's OWN coordinates, not the owner's path queue: a
/// non-null destination is moving; otherwise a null head-to is not moving; a
/// head-to whose X and Y already equal the owner's exact lepton position is not
/// moving; anything else is. Z is deliberately not compared.
///
/// This is the predicate `Drive::Is_Ok_To_End` consults before a piggyback may
/// be unwound. It is a different function from `Is_Moving_Now` (slot 32), which
/// additionally folds in hull rotation and the live per-frame speed — a Drive
/// unit with a destination but zero speed is `Is_Moving`, not `Is_Moving_Now`.
pub(crate) fn drive_locomotor_is_moving(entity: &GameEntity) -> bool {
    let Some(drive) = entity.drive_locomotion.as_ref() else {
        return false;
    };
    if drive.destination.is_some() {
        return true;
    }
    let Some(head) = drive.head_to else {
        return false;
    };
    let owner_x = i32::from(entity.position.rx) * 256 + entity.position.sub_x.to_num::<i32>();
    let owner_y = i32::from(entity.position.ry) * 256 + entity.position.sub_y.to_num::<i32>();
    head.x != owner_x || head.y != owner_y
}

pub(super) fn refresh_drive_head_to_coord(entity: &mut GameEntity, coord: DriveCoord) -> bool {
    let Some(drive) = entity.drive_locomotion.as_mut() else {
        return false;
    };
    if drive.head_to == Some(coord) {
        return false;
    }
    drive.head_to = Some(coord);
    true
}

pub(super) fn drive_entity_nav_targets(entities: &EntityStore) -> Vec<(u64, NavTargetRef)> {
    entities
        .keys_sorted()
        .into_iter()
        .filter_map(|id| {
            let entity = entities.get(id)?;
            let target = entity.navigation.nav_com?;
            matches!(target, NavTargetRef::Entity { .. }).then_some((id, target))
        })
        .collect()
}

/// Compute the Drive-local target speed fraction from currently modeled runtime
/// modifiers. This is the `DriveLocomotion` owner value; raw `Speed=` remains a
/// separate top-speed input.
///
/// `below_condition_yellow` carries the owner's damaged state into the fraction:
/// gamemd reads the health ratio inside `Process_Movement` itself and multiplies
/// the fraction by 0.75 when it is at or below `[AudioVisual] ConditionYellow`.
#[allow(clippy::too_many_arguments)]
pub(super) fn compute_drive_target_speed_fraction(
    speed_type: SpeedType,
    locomotor_kind: LocomotorKind,
    current_cell: (u16, u16),
    next_cell: (u16, u16),
    current_world_xy: (i32, i32),
    on_bridge: bool,
    is_unit: bool,
    terrain: &ResolvedTerrainGrid,
    config: &TerrainSpeedConfig,
    below_condition_yellow_or_unordered: bool,
) -> NativeF64Bits {
    if !matches!(locomotor_kind, LocomotorKind::Drive | LocomotorKind::Ship) {
        return NativeF64Bits::ONE;
    }

    let current = terrain.cell(current_cell.0, current_cell.1);
    let candidate = terrain.cell(next_cell.0, next_cell.1);
    let reference_level = current
        .map(|cell| i32::from(i8::from_ne_bytes([cell.level])))
        .unwrap_or(0)
        .wrapping_add(if on_bridge { 4 } else { 0 });
    let candidate_level = candidate
        .map(|cell| i32::from(i8::from_ne_bytes([cell.level])))
        .unwrap_or(0);
    let use_road = reference_level.abs_diff(candidate_level) >= 2;
    let profile = if use_road {
        &config.road_speed_costs
    } else {
        candidate
            .map(|cell| &cell.speed_costs)
            .unwrap_or(&config.road_speed_costs)
    };
    let row_f32 = profile.native_multiplier_for(speed_type);
    let row = f32::from_bits(row_f32.bits()) as f64;
    let mut result = if row > 1.0 {
        NativeF64Bits::ONE
    } else {
        NativeF64Bits::from_bits(row.to_bits())
    };

    if is_unit {
        let current_ground = current.and_then(|cell| {
            crate::util::lepton::ground_height_leptons(
                cell.level,
                cell.slope_type,
                current_world_xy.0,
                current_world_xy.1,
            )
            .ok()
        });
        let candidate_x = i32::from(next_cell.0).wrapping_mul(256).wrapping_add(128);
        let candidate_y = i32::from(next_cell.1).wrapping_mul(256).wrapping_add(128);
        let candidate_ground = candidate.and_then(|cell| {
            crate::util::lepton::ground_height_leptons(
                cell.level,
                cell.slope_type,
                candidate_x,
                candidate_y,
            )
            .ok()
        });
        if let (Some(current_ground), Some(candidate_ground)) =
            (current_ground, candidate_ground)
        {
            let slope = if candidate_ground > current_ground {
                Some(if speed_type == SpeedType::Track {
                    config.tracked_uphill_native
                } else {
                    config.wheeled_uphill_native
                })
            } else if candidate_ground < current_ground {
                Some(if speed_type == SpeedType::Track {
                    config.tracked_downhill_native
                } else {
                    config.wheeled_downhill_native
                })
            } else {
                None
            };
            if let Some(slope) = slope {
                result = native_fraction_product(result, slope);
            }
        }
    }

    let combined = f64::from_bits(result.bits());
    if combined == 0.0 || combined.is_nan() {
        result = NativeF64Bits::HALF;
    }
    if below_condition_yellow_or_unordered {
        result = native_fraction_product(
            result,
            NativeF64Bits::from_bits(0x3fe8_0000_0000_0000),
        );
    }
    result
}

fn native_fraction_product(lhs: NativeF64Bits, rhs: NativeF64Bits) -> NativeF64Bits {
    match (X87Chop53::load_f64(lhs), X87Chop53::load_f64(rhs)) {
        (Ok(lhs), Ok(rhs)) => X87Chop53::store_f64(X87Chop53::mul(lhs, rhs))
            .unwrap_or(NativeF64Bits::POSITIVE_ZERO),
        _ => NativeF64Bits::from_bits(
            (f64::from_bits(lhs.bits()) * f64::from_bits(rhs.bits())).to_bits(),
        ),
    }
}

/// Caller-specific Drive/Ship stored-destination distance. Native evaluates
/// the squared deltas in z/y/x order, materializes a qword, then passes it
/// through the shared f32-table `Sqrt_Approx` and chops to i32. A structural
/// destination cell replaces the stored Z with exact ground height +416.
pub(super) fn stored_destination_distance(
    owner_xyz: DriveCoord,
    mut destination: DriveCoord,
    terrain: Option<&ResolvedTerrainGrid>,
) -> i32 {
    if let Some(terrain) = terrain {
        let cell_x = destination.x.div_euclid(256);
        let cell_y = destination.y.div_euclid(256);
        if let (Ok(cell_x), Ok(cell_y)) = (u16::try_from(cell_x), u16::try_from(cell_y))
            && let Some(cell) = terrain.cell(cell_x, cell_y)
            && cell
                .bridge_facts
                .has_flag(crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL)
            && let Ok(ground) = crate::util::lepton::ground_height_leptons(
                cell.level,
                cell.slope_type,
                destination.x,
                destination.y,
            )
        {
            destination.z = ground.wrapping_add(416);
        }
    }

    let dz = X87Chop53::load_i32(destination.z.wrapping_sub(owner_xyz.z));
    let dy = X87Chop53::load_i32(destination.y.wrapping_sub(owner_xyz.y));
    let dx = X87Chop53::load_i32(destination.x.wrapping_sub(owner_xyz.x));
    let zy = X87Chop53::add(X87Chop53::mul(dz, dz), X87Chop53::mul(dy, dy));
    let squared = X87Chop53::add(zy, X87Chop53::mul(dx, dx));
    let squared = X87Chop53::store_f64(squared).unwrap_or(NativeF64Bits::POSITIVE_ZERO);
    let squared = X87Chop53::load_f64(squared).expect("map squared distance is finite");
    let root = crate::util::native_x87::sqrt_approx_f32(squared)
        .expect("map squared distance fits Sqrt_Approx");
    let root = X87Chop53::load_f32(root).expect("Sqrt_Approx returns finite f32");
    X87Chop53::ftol_i64(root).unwrap_or(i64::MIN) as i32
}

/// Update Drive target/current speed fractions before budget consumption.
///
/// Gamemd keeps the target fraction on DriveLocomotion and the applied/current
/// fraction on the owner through `SetSpeedFraction`. Rust stores both in the
/// runtime for now, but `current_speed_fraction` is the movement authority.
pub(super) fn update_drive_speed_fraction(
    drive: &mut DriveLocomotionRuntime,
    owner_current_fraction: &mut NativeF64Bits,
    target_fraction: NativeF64Bits,
    accelerates: bool,
    native_type_speed: i32,
    accel_factor: NativeF64Bits,
    decel_factor: NativeF64Bits,
    slowdown_distance: i32,
    distance_to_goal: i32,
) {
    update_drive_speed_fraction_with_flags(
        drive,
        owner_current_fraction,
        target_fraction,
        accelerates,
        native_type_speed,
        accel_factor,
        decel_factor,
        slowdown_distance,
        distance_to_goal,
        VehicleRampFlags::default(),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_drive_speed_fraction_with_flags(
    drive: &mut DriveLocomotionRuntime,
    owner_current_fraction: &mut NativeF64Bits,
    target_fraction: NativeF64Bits,
    accelerates: bool,
    native_type_speed: i32,
    accel_factor: NativeF64Bits,
    decel_factor: NativeF64Bits,
    slowdown_distance: i32,
    distance_to_goal: i32,
    flags: VehicleRampFlags,
) {
    if drive.track_index >= 64 {
        // ProcessMovement's forced-selector arm bypasses the locomotor target
        // and conditionally writes the owner. An unordered comparison skips.
        if ordered_compare_bits(target_fraction, *owner_current_fraction)
            .is_some_and(|ordering| ordering != X87Ordering::Equal)
        {
            *owner_current_fraction = normalize_current_speed_fraction(target_fraction);
        }
        if accelerates {
            return;
        }
        // Nonaccelerated Track ignores the selector gate and applies the
        // already-installed locomotor target, not the bypass result.
        let installed_target = drive.target_speed_fraction;
        update_vehicle_speed_fraction(
            &mut drive.target_speed_fraction,
            owner_current_fraction,
            installed_target,
            false,
            native_type_speed,
            accel_factor,
            decel_factor,
            slowdown_distance,
            distance_to_goal,
            flags,
        );
        return;
    }
    update_vehicle_speed_fraction(
        &mut drive.target_speed_fraction,
        owner_current_fraction,
        target_fraction,
        accelerates,
        native_type_speed,
        accel_factor,
        decel_factor,
        slowdown_distance,
        distance_to_goal,
        flags,
    );
}

/// Update Ship's class-owned target fraction and owner-applied fraction.
///
/// The active Ship `Process_Drive_Track` body uses these same transitions
/// before calling the owner's `SetSpeedFraction` slot. Keeping both values on
/// Ship runtime gives `Is_Moving_Now` its native source without consulting the
/// path-execution adapter.
#[allow(clippy::too_many_arguments)]
pub(super) fn update_ship_speed_fraction(
    ship: &mut ShipLocomotionRuntime,
    owner_current_fraction: &mut NativeF64Bits,
    target_fraction: NativeF64Bits,
    accelerates: bool,
    native_type_speed: i32,
    accel_factor: NativeF64Bits,
    decel_factor: NativeF64Bits,
    slowdown_distance: i32,
    distance_to_goal: i32,
) {
    update_ship_speed_fraction_with_flags(
        ship,
        owner_current_fraction,
        target_fraction,
        accelerates,
        native_type_speed,
        accel_factor,
        decel_factor,
        slowdown_distance,
        distance_to_goal,
        VehicleRampFlags::default(),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_ship_speed_fraction_with_flags(
    ship: &mut ShipLocomotionRuntime,
    owner_current_fraction: &mut NativeF64Bits,
    target_fraction: NativeF64Bits,
    accelerates: bool,
    native_type_speed: i32,
    accel_factor: NativeF64Bits,
    decel_factor: NativeF64Bits,
    slowdown_distance: i32,
    distance_to_goal: i32,
    flags: VehicleRampFlags,
) {
    if ship.track_index >= 64 {
        if ordered_compare_bits(target_fraction, *owner_current_fraction)
            .is_some_and(|ordering| ordering != X87Ordering::Equal)
        {
            *owner_current_fraction = normalize_current_speed_fraction(target_fraction);
        }
        if accelerates {
            return;
        }
        let installed_target = ship.target_speed_fraction;
        update_vehicle_speed_fraction(
            &mut ship.target_speed_fraction,
            owner_current_fraction,
            installed_target,
            false,
            native_type_speed,
            accel_factor,
            decel_factor,
            slowdown_distance,
            distance_to_goal,
            flags,
        );
        return;
    }
    update_vehicle_speed_fraction(
        &mut ship.target_speed_fraction,
        owner_current_fraction,
        target_fraction,
        accelerates,
        native_type_speed,
        accel_factor,
        decel_factor,
        slowdown_distance,
        distance_to_goal,
        flags,
    );
}

/// Ship normally recomputes its requested fraction in `Process_Movement`.
/// `Stop_Moving` is the active exception: after it clears destination while a
/// committed head remains, `Process_Drive_Track` consumes the class-owned
/// clamped target without running a fresh terrain request over it.
pub(super) fn ship_process_target_speed_fraction(
    ship: &ShipLocomotionRuntime,
    movement_target_fraction: NativeF64Bits,
) -> NativeF64Bits {
    if ship.destination.is_none() && ship.head_to.is_some() {
        ship.target_speed_fraction
    } else {
        movement_target_fraction
    }
}

/// The `Accelerates=` ramp, `DriveLocomotionClass::Process_Drive_Track` @
/// `0x004B0F20`. Three of its arms are **not** modelled, recorded here:
///
/// - **The second brake band.** Outside `SlowdownDistance`, when `owner+0x3CD`
///   is set, native brakes by `rawSpeed × 0.0015` with a floor of `0.1`
///   (`0x004B10FF`) — much gentler, to a much lower floor, than the arrival
///   band. `+0x3CD` is written by `UnitClass::ReceiveDamage` `0x00737E51`,
///   `TemporalClass::AI` `0x00629C69`, the teleport post-warp validation and
///   the jumpjet touchdown, and `Process` @ `0x004B0500` also zeroes the target
///   speed on it — the two writes there are `[recv+0x4C]` and `[recv+0x50]` on
///   the **ILocomotion** receiver, i.e. complete-object `+0x50`/`+0x54`, the two
///   halves of one `double`. The integer movement residual at complete `+0x4C`
///   is untouched. So it is load-bearing in two places. Its identity is
///   UNCHECKED, which is why this carries no frequency clause yet: that is the
///   gap to close before ranking it.
/// - **The crush clamp.** While `owner+0x6B5` is set (raised at `0x004B1A2F`
///   when the mover drives over a crushable, cleared in
///   `UnitClass::PerCellProcess`), native replaces the whole ramp with
///   `min(target, 0.2)` and writes it back to `drive+0x50` (`0x004B1146`).
///   Trigger: a crusher mid-crush. Player effect: retail slows to a fifth speed
///   over the victim. Frequency: **common, not rare.** It lives inside the
///   `Accelerates` branch, and of the 29 `Crusher=yes` types in stock
///   `rulesmd.ini` only 14 carry `Accelerates=false` — the 15 that accelerate
///   include `[APOC]`, both MCVs, `[V3]`, every ore miner and the amphibious
///   transports. An Apocalypse driving over infantry is ordinary Soviet play.
///   Downstream risk: it writes the locomotor-owned slot, not just the owner's.
/// - **The `drive+0x58 >= 0x40` bypass.** Both `Process_Movement` @
///   `0x004B2630` and this function gate on `*(int*)(drive+0x58) < 0x40`; above
///   it native skips the `drive+0x50` write *and* the whole ramp, setting the
///   owner fraction directly. VERA always writes and always ramps. Trigger:
///   a live `Force_Track` curve. `Force_Track` @ `0x004B0C40` is the only writer
///   that can push the selector past 0x3F — both ordinary writers cap at 63 —
///   and the three call sites that pass one are the **Yuri Tank Bunker**
///   occupant lifecycle: the installer at `0x00458E50` (`0x43`..`0x46`, chosen
///   by facing at `0x00459132`-`0x0045915C`), `BuildingClass::UndockUnit` @
///   `0x004593A0` (`0x47`, pushed at `0x0045942C`) and
///   `BuildingClass::ReleaseDockedHarvester` @ `0x004595C0` (`0x47`, pushed at
///   `0x00459751`). The latter two early-return unless the building's `+0x2E4`
///   dock link is set, and the installer is its only setter, so the whole family
///   needs a garrisoned bunker. Stock `rulesmd.ini` has exactly one
///   `Bunker=yes`: `[NATBNK]`.
///
///   So the gate means: while on a bunker install or eject curve, skip the ramp
///   and drive at the 1.0 `Force_Track` installed. Player effect: VERA ramps
///   where native jumps straight to full speed. Frequency: **zero in any match
///   without a garrisoned Tank Bunker**, occasional in Yuri matchups — not the
///   factory door. Downstream risk: `ForcedDriveTrackState` already carries a
///   full-speed constant, so the forced arm is approximated; what is missing is
///   the same `< 0x40` gate inside `Process_Movement` @ `0x004B2630` (gates read
///   at `0x004B0FA8` and `0x004B3DFA`).
///
///   `Force_Track` has three further callers, all passing `-1`:
///   `TechnoClass::PerformDeploy` @ `0x007101B3`, `SuperClass::Launch` @
///   `0x006CCAA2`, and `0x0062AB24`.
/// - **The `Passive=` skip.** `UnitTypeClass+0xE0C` (stored at `0x0074783D`)
///   disables the entire ramp for a UnitClass mover. Trigger: a `Passive=yes`
///   type. Player effect: none observed. Frequency: zero in skirmish — stock
///   `Passive=yes` is civilian traffic. Downstream risk: one `if`.
#[allow(clippy::too_many_arguments)]
fn update_vehicle_speed_fraction(
    target_slot: &mut NativeF64Bits,
    current_slot: &mut NativeF64Bits,
    target_fraction: NativeF64Bits,
    accelerates: bool,
    native_type_speed: i32,
    accel_factor: NativeF64Bits,
    decel_factor: NativeF64Bits,
    slowdown_distance: i32,
    distance_to_goal: i32,
    flags: VehicleRampFlags,
) {
    // The locomotor-owned target fraction is **not** clamped in gamemd.
    // `Process_Movement` @ `0x004B2630` writes `drive+0x50` raw, so a healthy
    // tracked mover going downhill on a 100% land row legitimately carries 1.2
    // — the terrain chain's own tests assert the combined value exceeds 1.0.
    // The only native clamp is inside `TechnoClass::SetSpeedFraction` @
    // `0x004D3710`, on the owner's `+0x578`.
    //
    // That clamp is still reached: **every** arm of `Process_Drive_Track` @
    // `0x004B0F20` terminates in `SetSpeedFraction` through vtable `+0x544`, so
    // gamemd discards the above-1.0 portion one step later and the mover does
    // not actually go faster downhill. Keeping this slot unclamped matches
    // where the native clamp lives, and matters only to anything that reads the
    // *target* fraction rather than the owner's; it is not a speed change.
    *target_slot = target_fraction;
    if !accelerates {
        *current_slot = normalize_current_speed_fraction(target_fraction);
        return;
    }
    if flags.passive {
        return;
    }

    let brake = if distance_to_goal < slowdown_distance {
        let candidate = native_sub_scaled(*current_slot, native_type_speed, decel_factor);
        Some(ordered_max(candidate, DRIVE_DESTINATION_BRAKE_FLOOR))
    } else if flags.alternate_brake {
        let candidate = native_sub_scaled(
            *current_slot,
            native_type_speed,
            DRIVE_ALTERNATE_BRAKE_RATE,
        );
        Some(ordered_max(candidate, DRIVE_ALTERNATE_BRAKE_FLOOR))
    } else {
        None
    };

    if flags.currently_crushing {
        let crush_target = ordered_min_unordered_left(*target_slot, DRIVE_CRUSH_FRACTION);
        *target_slot = crush_target;
        *current_slot = normalize_current_speed_fraction(crush_target);
        return;
    }
    if let Some(brake) = brake {
        *current_slot = normalize_current_speed_fraction(brake);
        return;
    }

    let next = match ordered_compare_bits(*current_slot, *target_slot) {
        Some(X87Ordering::Equal) => return,
        Some(X87Ordering::Greater) => {
            let candidate = native_sub_scaled(*current_slot, native_type_speed, decel_factor);
            if ordered_compare_bits(*target_slot, candidate) == Some(X87Ordering::Greater) {
                *target_slot
            } else {
                candidate
            }
        }
        Some(X87Ordering::Less) | None => {
            let candidate = native_add_bits(*current_slot, accel_factor);
            match ordered_compare_bits(candidate, *target_slot) {
                Some(X87Ordering::Greater) | None => *target_slot,
                _ => candidate,
            }
        }
    };
    *current_slot = normalize_current_speed_fraction(next);
}

fn raw_is_nan(bits: NativeF64Bits) -> bool {
    let raw = bits.bits();
    raw & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000
        && raw & 0x000f_ffff_ffff_ffff != 0
}

fn ordered_compare_bits(lhs: NativeF64Bits, rhs: NativeF64Bits) -> Option<X87Ordering> {
    if raw_is_nan(lhs) || raw_is_nan(rhs) {
        return None;
    }
    match (X87Chop53::load_f64(lhs), X87Chop53::load_f64(rhs)) {
        (Ok(lhs), Ok(rhs)) => Some(X87Chop53::compare(lhs, rhs)),
        _ => f64::from_bits(lhs.bits())
            .partial_cmp(&f64::from_bits(rhs.bits()))
            .map(|ordering| match ordering {
                std::cmp::Ordering::Less => X87Ordering::Less,
                std::cmp::Ordering::Equal => X87Ordering::Equal,
                std::cmp::Ordering::Greater => X87Ordering::Greater,
            }),
    }
}

fn native_add_bits(lhs: NativeF64Bits, rhs: NativeF64Bits) -> NativeF64Bits {
    match (X87Chop53::load_f64(lhs), X87Chop53::load_f64(rhs)) {
        (Ok(lhs), Ok(rhs)) => X87Chop53::store_f64(X87Chop53::add(lhs, rhs))
            .unwrap_or(NativeF64Bits::POSITIVE_ZERO),
        _ => NativeF64Bits::from_bits(
            (f64::from_bits(lhs.bits()) + f64::from_bits(rhs.bits())).to_bits(),
        ),
    }
}

fn native_sub_scaled(
    current: NativeF64Bits,
    native_type_speed: i32,
    rate: NativeF64Bits,
) -> NativeF64Bits {
    match (X87Chop53::load_f64(current), X87Chop53::load_f64(rate)) {
        (Ok(current), Ok(rate)) => {
            let delta = X87Chop53::mul(X87Chop53::load_i32(native_type_speed), rate);
            X87Chop53::store_f64(X87Chop53::sub(current, delta))
                .unwrap_or(NativeF64Bits::POSITIVE_ZERO)
        }
        _ => NativeF64Bits::from_bits(
            (f64::from_bits(current.bits())
                - f64::from(native_type_speed) * f64::from_bits(rate.bits()))
            .to_bits(),
        ),
    }
}

/// Ordered max whose unordered arm preserves the left/candidate operand.
fn ordered_max(candidate: NativeF64Bits, floor: NativeF64Bits) -> NativeF64Bits {
    if ordered_compare_bits(candidate, floor) == Some(X87Ordering::Less) {
        floor
    } else {
        candidate
    }
}

/// Ordered min whose unordered arm selects the native left/target operand.
fn ordered_min_unordered_left(target: NativeF64Bits, cap: NativeF64Bits) -> NativeF64Bits {
    if ordered_compare_bits(target, cap) == Some(X87Ordering::Greater) {
        cap
    } else {
        target
    }
}

/// Reproduce the positive/zero value returned by owner slot `+0x538` for the
/// active stock vehicle speed path.
///
/// `FootClass::GetCurrentSpeed` first truncates the type/owner-adjusted raw
/// speed, then multiplies by owner `+0x578` and truncates again. Rust keeps the
/// adjusted speed in leptons/second, so dividing by the 15-Hz native baseline
/// recovers the first integer before applying the locomotor-owned fraction.
///
/// Two native terms are **not** modelled, recorded rather than guessed:
/// - the elite multiply, `HasWeaponAbility(0)` -> `x Rules+0x678`, which sits
///   *between* the two truncations (`0x004DB1E8`-`0x004DB205`). Trigger: an
///   elite unit. Player effect: elites move at their veteran speed instead of
///   their elite one. Frequency: every elite vehicle, which an active player
///   accumulates over a long match. Downstream risk: none - one factor in the
///   middle of a chain VERA already reproduces exactly.
/// - the halving at `0x004DB226`: RTTI 1 (UnitClass) with `owner+0x6CC != -1`
///   -> `speed / 2` via `CDQ/SUB/SAR 1`. `+0x6CC` is UNCHECKED, so this
///   carries no frequency clause - naming that field is the prerequisite, and
///   a factor of two on a vehicle's per-frame speed is first-order if it
///   fires.
pub(crate) fn native_fraction_from_sim(value: SimFixed) -> NativeF64Bits {
    NativeF64Bits::from_bits(value.to_num::<f64>().to_bits())
}

/// Legacy adapter retained only for callers being migrated in this slice.
/// Production Drive/Ship/Walk/Hover paths no longer use it.
pub(crate) fn owner_current_speed_from_fraction(
    adjusted_speed_per_second: SimFixed,
    current_speed_fraction: SimFixed,
) -> i32 {
    let adjusted_type_speed =
        (adjusted_speed_per_second / SimFixed::from_num(15)).to_num::<i32>();
    foot_current_speed(
        adjusted_type_speed,
        NativeF32Bits::ONE,
        NativeF64Bits::ONE,
        false,
        NativeF64Bits::ONE,
        normalize_current_speed_fraction(native_fraction_from_sim(current_speed_fraction)),
    )
}

/// Native `TechnoClass::SetSpeedFraction @ 0x004D3710` selection contract.
/// Ordered interior values preserve every input bit; infinities and NaNs take
/// their native compare arms rather than Rust `clamp` semantics.
pub(crate) fn normalize_current_speed_fraction(input: NativeF64Bits) -> NativeF64Bits {
    let raw = input.bits();
    let sign = raw >> 63 != 0;
    let exponent = (raw >> 52) & 0x7ff;
    let fraction = raw & 0x000f_ffff_ffff_ffff;
    if exponent == 0x7ff {
        if fraction != 0 || sign {
            return NativeF64Bits::POSITIVE_ZERO;
        }
        return NativeF64Bits::ONE;
    }
    let value = X87Chop53::load_f64(input).expect("finite fraction including subnormal");
    let one = X87Chop53::load_f64(NativeF64Bits::ONE).expect("one");
    if matches!(
        X87Chop53::compare(value, one),
        X87Ordering::Equal | X87Ordering::Greater
    ) {
        return NativeF64Bits::ONE;
    }
    let zero = X87Chop53::load_f64(NativeF64Bits::POSITIVE_ZERO).expect("zero");
    if matches!(
        X87Chop53::compare(value, zero),
        X87Ordering::Equal | X87Ordering::Less
    ) {
        return NativeF64Bits::POSITIVE_ZERO;
    }
    input
}

pub(crate) fn set_entity_current_speed_fraction(
    entity: &mut GameEntity,
    input: NativeF64Bits,
) -> NativeF64Bits {
    let selected = normalize_current_speed_fraction(input);
    entity.current_speed_fraction = selected;
    selected
}

fn ftol_low_i32(value: crate::util::native_x87::X87Value) -> i32 {
    X87Chop53::ftol_i64(value).unwrap_or(i64::MIN) as i32
}

/// Exact `FootClass::GetCurrentSpeed @ 0x004DB1A0` three-stage consumer.
/// House and crate multipliers share stage one; VeteranSpeed and current
/// fraction each have their own mandatory integer boundary.
pub(crate) fn foot_current_speed(
    native_type_speed: i32,
    house_speed_bonus: NativeF32Bits,
    crate_speed: NativeF64Bits,
    has_faster_ability: bool,
    veteran_speed: NativeF64Bits,
    current_fraction: NativeF64Bits,
) -> i32 {
    let Ok(house) = X87Chop53::load_f32(house_speed_bonus) else {
        return 0;
    };
    let Ok(crate_mult) = X87Chop53::load_f64(crate_speed) else {
        return 0;
    };
    let stage1 = X87Chop53::mul(X87Chop53::load_i32(native_type_speed), house);
    let stage1 = ftol_low_i32(X87Chop53::mul(stage1, crate_mult));
    let stage2 = if has_faster_ability {
        let Ok(veteran) = X87Chop53::load_f64(veteran_speed) else {
            return 0;
        };
        ftol_low_i32(X87Chop53::mul(X87Chop53::load_i32(stage1), veteran))
    } else {
        stage1
    };
    let Ok(fraction) = X87Chop53::load_f64(current_fraction) else {
        return 0;
    };
    ftol_low_i32(X87Chop53::mul(
        X87Chop53::load_i32(stage2),
        fraction,
    ))
}

/// Infantry's sole owner-vslot override, applied after the common Foot query.
pub(crate) fn infantry_current_speed(common: i32, is_prone: bool, crawls: bool) -> i32 {
    if !is_prone {
        return common;
    }
    if crawls {
        common.wrapping_sub(common / 3)
    } else {
        common.wrapping_add(common / 2)
    }
}

/// Resolve every live HouseType/category/crate/rank operand for the native
/// owner query. Jumpjet and Teleport deliberately do not call this from their
/// displacement paths; Drive, Ship, Walk, and Hover do.
pub(crate) fn entity_current_speed(
    entity: &GameEntity,
    object: &ObjectType,
    rules: &RuleSet,
    houses: &BTreeMap<InternedId, HouseState>,
    interner: &StringInterner,
) -> i32 {
    let country_name = houses
        .get(&entity.owner)
        .and_then(|house| house.country)
        .map(|country| interner.resolve(country))
        .unwrap_or_else(|| interner.resolve(entity.owner));
    let house_bonus = rules.country_speed_bonus(country_name, object.category);
    let has_faster = if entity.veterancy
        >= crate::sim::combat::veterancy::RANK_ELITE_U16
    {
        object.veteran_faster || object.elite_faster
    } else {
        entity.veterancy >= crate::sim::combat::veterancy::RANK_VETERAN_U16
            && object.veteran_faster
    };
    let common = foot_current_speed(
        object.native_speed,
        house_bonus,
        entity.speed_crate_multiplier,
        has_faster,
        rules.general.veteran_speed,
        entity.current_speed_fraction,
    );
    if object.category == ObjectCategory::Infantry {
        infantry_current_speed(
            common,
            crate::sim::infantry::is_prone_for_damage(entity),
            object.crawls,
        )
    } else {
        common
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::util::fixed_math::{SIM_HALF, SIM_ONE};

    fn native_f64(value: f64) -> NativeF64Bits {
        NativeF64Bits::from_bits(value.to_bits())
    }

    fn native_value(value: NativeF64Bits) -> f64 {
        f64::from_bits(value.bits())
    }

    #[test]
    fn gsi_13_06_ship_speed_fraction_uses_locomotor_owned_state() {
        let mut ship = ShipLocomotionRuntime::default();
        let mut current = NativeF64Bits::POSITIVE_ZERO;

        update_ship_speed_fraction(
            &mut ship,
            &mut current,
            native_f64(0.5),
            false,
            10,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1000,
        );
        assert_eq!(ship.target_speed_fraction, native_f64(0.5));
        assert_eq!(current, native_f64(0.5));

        current = NativeF64Bits::POSITIVE_ZERO;
        update_ship_speed_fraction(
            &mut ship,
            &mut current,
            NativeF64Bits::ONE,
            true,
            10,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1000,
        );
        assert_eq!(ship.target_speed_fraction, NativeF64Bits::ONE);
        assert_eq!(current, native_f64(0.03));
    }

    #[test]
    fn gsi_13_06_stock_shp_current_speed_preserves_native_truncation() {
        use crate::util::fixed_math::ra2_speed_to_leptons_per_second;

        let stock_ship_speed = ra2_speed_to_leptons_per_second(8);
        assert_eq!(
            owner_current_speed_from_fraction(stock_ship_speed, SimFixed::lit("0.03")),
            0,
            "DLPH/SQD first acceleration step truncates to zero"
        );
        assert_eq!(
            owner_current_speed_from_fraction(stock_ship_speed, SimFixed::lit("0.06")),
            1
        );
        let stock_dron_speed = ra2_speed_to_leptons_per_second(10);
        assert_eq!(
            owner_current_speed_from_fraction(stock_dron_speed, SimFixed::lit("0.03")),
            0,
            "DRON's raw stage 25 still truncates 0.75 to zero"
        );
        assert_eq!(
            owner_current_speed_from_fraction(stock_dron_speed, SIM_ONE),
            25,
            "Accelerates=false DRON uses its full converted type speed"
        );
    }

    #[test]
    fn gsi_13_06_nonterminal_ship_post_stop_process_preserves_clamped_target() {
        use crate::util::fixed_math::ra2_speed_to_leptons_per_second;

        let speed = ra2_speed_to_leptons_per_second(8);
        let native_speed = (speed / SimFixed::from_num(15)).to_num::<i32>();
        let target = DRIVE_DESTINATION_BRAKE_FLOOR;
        let mut current = native_f64(0.5);
        let mut ship = ShipLocomotionRuntime {
            destination: None,
            head_to: Some(DriveCoord::cell(4, 3, 0)),
            target_speed_fraction: target,
            owner_current_speed: 10,
            ..Default::default()
        };

        for _ in 0..10 {
            let requested = ship_process_target_speed_fraction(&ship, NativeF64Bits::ONE);
            assert_eq!(requested, target);
            update_ship_speed_fraction(
                &mut ship,
                &mut current,
                requested,
                true,
                native_speed,
                native_f64(0.03),
                native_f64(0.002),
                0,
                256,
            );
            ship.owner_current_speed = foot_current_speed(
                native_speed,
                NativeF32Bits::ONE,
                NativeF64Bits::ONE,
                false,
                NativeF64Bits::ONE,
                current,
            );
            assert_eq!(ship.target_speed_fraction, target);
            assert!(ship.owner_current_speed > 0);
        }

        assert_eq!(ship.destination, None);
        assert_eq!(ship.head_to, Some(DriveCoord::cell(4, 3, 0)));
        assert_eq!(current, target);
        assert_eq!(ship.owner_current_speed, 6);
    }

    fn terrain_cell(rx: u16, ry: u16, mut speed_costs: SpeedCostProfile) -> ResolvedTerrainCell {
        let native = |value: Option<u8>| {
            NativeF32Bits::from_bits((f32::from(value.unwrap_or(100)) / 100.0).to_bits())
        };
        if !speed_costs.native_row_present {
            speed_costs.native_row_present = true;
            speed_costs.native_speed_bits = [
                native(speed_costs.foot),
                native(speed_costs.track),
                native(speed_costs.wheel),
                native(speed_costs.hover),
                NativeF32Bits::ONE,
                native(speed_costs.float),
                native(speed_costs.amphibious),
                native(speed_costs.float_beach),
            ];
        }
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: false,
            tileset_index: Some(0),
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs,
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: false,
            height_in_pixels: 0,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            terrain_object_occupation: None,
            overlay_blocks: false,
            overlay_zone_type: None,
            outside_playfield: false,
            zone_type: 0,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: Default::default(),
            base_speed_costs: Default::default(),
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn sim_fraction(value: NativeF64Bits) -> SimFixed {
        SimFixed::from_num(f64::from_bits(value.bits()))
    }

    #[test]
    fn drive_target_speed_fraction_uses_terrain_modifier() {
        let terrain = ResolvedTerrainGrid::from_cells(
            2,
            1,
            vec![
                terrain_cell(0, 0, SpeedCostProfile::default()),
                terrain_cell(
                    1,
                    0,
                    SpeedCostProfile {
                        track: Some(50),
                        ..Default::default()
                    },
                ),
            ],
        );

        let fraction = sim_fraction(compute_drive_target_speed_fraction(
            SpeedType::Track,
            LocomotorKind::Drive,
            (0, 0),
            (1, 0),
            (128, 128),
            false,
            true,
            &terrain,
            &TerrainSpeedConfig::default(),
            false,
        ));

        assert_eq!(fraction, SIM_HALF);
    }

    /// Two flat `[Clear]` cells, `Track=100%`, so the terrain × slope product is
    /// exactly 1.0 and the only thing under test is the damaged-mover factor.
    fn flat_clear_pair() -> ResolvedTerrainGrid {
        let clear = SpeedCostProfile {
            foot: Some(100),
            track: Some(100),
            wheel: Some(100),
            ..Default::default()
        };
        ResolvedTerrainGrid::from_cells(
            2,
            1,
            vec![terrain_cell(0, 0, clear), terrain_cell(1, 0, clear)],
        )
    }

    fn fraction_for(kind: LocomotorKind, damaged: bool) -> SimFixed {
        sim_fraction(compute_drive_target_speed_fraction(
            SpeedType::Track,
            kind,
            (0, 0),
            (1, 0),
            (128, 128),
            false,
            true,
            &flat_clear_pair(),
            &TerrainSpeedConfig::default(),
            damaged,
        ))
    }

    /// GSI-06.04 G1: gamemd multiplies the per-cell speed fraction by 0.75 when
    /// the owner's health ratio is at or below `[AudioVisual] ConditionYellow`.
    /// A healthy mover on the same cells keeps the full fraction.
    #[test]
    fn gsi_06_04_damaged_drive_mover_slows_to_three_quarters() {
        assert_eq!(fraction_for(LocomotorKind::Drive, false), SIM_ONE);
        assert_eq!(
            fraction_for(LocomotorKind::Drive, true),
            SimFixed::lit("0.75")
        );
    }

    /// The same factor is read by Ship `Process_Movement`; a damaged destroyer
    /// slows exactly like a damaged tank.
    #[test]
    fn gsi_06_04_damaged_ship_mover_slows_to_three_quarters() {
        assert_eq!(fraction_for(LocomotorKind::Ship, false), SIM_ONE);
        assert_eq!(
            fraction_for(LocomotorKind::Ship, true),
            SimFixed::lit("0.75")
        );
    }

    /// The damaged factor is NOT read by any other locomotor — a wounded GI, a
    /// wounded Robot Tank and a wounded Rocketeer all keep full speed. Guards
    /// against over-applying the factor once it exists.
    #[test]
    fn gsi_06_04_damaged_non_drive_movers_keep_full_speed() {
        for kind in [
            LocomotorKind::Walk,
            LocomotorKind::Hover,
            LocomotorKind::Jumpjet,
            LocomotorKind::Mech,
            LocomotorKind::Teleport,
            LocomotorKind::Tunnel,
            LocomotorKind::Fly,
        ] {
            assert_eq!(fraction_for(kind, true), SIM_ONE, "damaged {kind:?}");
            assert_eq!(fraction_for(kind, false), SIM_ONE, "healthy {kind:?}");
        }
    }

    /// GSI-06.04 G2: gamemd routes the land-type table through Drive and Ship
    /// only. A hover mover standing on `[Clear] Hover=50%` therefore runs at full
    /// speed on land — its locomotor is a pure throttle that never reads the
    /// table — while a Drive mover on the same cell is halved.
    #[test]
    fn gsi_06_04_land_type_table_reaches_drive_and_ship_only() {
        let terrain = ResolvedTerrainGrid::from_cells(
            2,
            1,
            vec![
                terrain_cell(0, 0, SpeedCostProfile::default()),
                terrain_cell(
                    1,
                    0,
                    SpeedCostProfile {
                        hover: Some(50),
                        track: Some(50),
                        ..Default::default()
                    },
                ),
            ],
        );
        let sample = |kind: LocomotorKind, st: SpeedType| {
            sim_fraction(compute_drive_target_speed_fraction(
                st,
                kind,
                (0, 0),
                (1, 0),
                (128, 128),
                false,
                true,
                &terrain,
                &TerrainSpeedConfig::default(),
                false,
            ))
        };
        assert_eq!(sample(LocomotorKind::Drive, SpeedType::Track), SIM_HALF);
        assert_eq!(sample(LocomotorKind::Ship, SpeedType::Track), SIM_HALF);
        assert_eq!(sample(LocomotorKind::Hover, SpeedType::Hover), SIM_ONE);
        assert_eq!(sample(LocomotorKind::Walk, SpeedType::Foot), SIM_ONE);
    }

    /// GSI-06.04 G2 (slope half): the four `[General]` slope coefficients are
    /// applied at the same two sites as the table, so infantry get no downhill
    /// boost on a ramp.
    #[test]
    fn gsi_06_04_slope_coefficients_reach_drive_and_ship_only() {
        let flat = SpeedCostProfile {
            foot: Some(100),
            track: Some(100),
            ..Default::default()
        };
        let mut high = terrain_cell(0, 0, flat);
        high.level = 2;
        let terrain = ResolvedTerrainGrid::from_cells(2, 1, vec![high, terrain_cell(1, 0, flat)]);
        let sample = |kind: LocomotorKind, st: SpeedType| {
            sim_fraction(compute_drive_target_speed_fraction(
                st,
                kind,
                (0, 0),
                (1, 0),
                (128, 128),
                false,
                true,
                &terrain,
                &TerrainSpeedConfig::default(),
                false,
            ))
        };
        // Downhill: the Drive mover takes the 1.2x coefficient...
        assert_eq!(
            sample(LocomotorKind::Drive, SpeedType::Track),
            SimFixed::lit("1.2")
        );
        // ...and the walking infantryman does not.
        assert_eq!(sample(LocomotorKind::Walk, SpeedType::Foot), SIM_ONE);
    }

    fn drive_entity_at(rx: u16, ry: u16) -> GameEntity {
        let mut entity = GameEntity::test_default(1, "HARV", "Americans", rx, ry);
        entity.drive_locomotion = Some(DriveLocomotionRuntime::default());
        entity
    }

    /// GSI-06.12 GAP 3: `Drive::Is_Ok_To_End` reads ILocomotion slot 4
    /// (`Is_Moving`) on the ACTIVE locomotor, and that predicate looks at the
    /// Drive locomotor's own destination and head-to coords — not at the
    /// owner's path queue. A non-null destination alone means "moving".
    #[test]
    fn gsi_06_12_drive_is_moving_reads_its_own_destination() {
        let mut entity = drive_entity_at(3, 3);
        assert!(!drive_locomotor_is_moving(&entity));

        entity.drive_locomotion.as_mut().expect("drive").destination =
            Some(DriveCoord::cell(9, 3, 0));
        assert!(drive_locomotor_is_moving(&entity));
    }

    /// With no destination, a null head-to is not moving and a head-to that
    /// already equals the owner's exact lepton X/Y is not moving either. Z is
    /// deliberately not part of the comparison.
    #[test]
    fn gsi_06_12_drive_is_moving_compares_head_to_against_the_owner_position() {
        let mut entity = drive_entity_at(3, 3);
        entity.position.sub_x = SimFixed::from_num(128);
        entity.position.sub_y = SimFixed::from_num(128);

        let drive = entity.drive_locomotion.as_mut().expect("drive");
        drive.head_to = Some(DriveCoord::cell(3, 3, 0));
        assert!(
            !drive_locomotor_is_moving(&entity),
            "head-to at the owner's own cell centre is not moving"
        );

        let drive = entity.drive_locomotion.as_mut().expect("drive");
        drive.head_to = Some(DriveCoord::cell(3, 3, 7));
        assert!(
            !drive_locomotor_is_moving(&entity),
            "a Z-only difference does not make it moving"
        );

        let drive = entity.drive_locomotion.as_mut().expect("drive");
        drive.head_to = Some(DriveCoord::cell(4, 3, 0));
        assert!(drive_locomotor_is_moving(&entity));
    }

    #[test]
    fn a_downhill_target_stays_above_one_but_the_owner_fraction_does_not() {
        // `Process_Movement` @ 0x004B2630 writes `drive+0x50` unclamped, so a
        // 1.2 downhill product survives on the locomotor-owned slot; the only
        // native clamp is `TechnoClass::SetSpeedFraction` @ 0x004D3710, which
        // every arm of `Process_Drive_Track` reaches, so the owner's fraction
        // never exceeds 1.
        let mut drive = DriveLocomotionRuntime::default();
        let mut current = NativeF64Bits::POSITIVE_ZERO;

        update_drive_speed_fraction(
            &mut drive,
            &mut current,
            native_f64(1.2),
            false,
            10,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1000,
        );

        assert_eq!(drive.target_speed_fraction, native_f64(1.2));
        assert_eq!(current, NativeF64Bits::ONE);
    }

    #[test]
    fn accelerates_false_assigns_current_fraction_directly() {
        let mut drive = DriveLocomotionRuntime::default();
        let mut current = NativeF64Bits::POSITIVE_ZERO;

        update_drive_speed_fraction(
            &mut drive,
            &mut current,
            native_f64(0.5),
            false,
            10,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1000,
        );

        assert_eq!(drive.target_speed_fraction, native_f64(0.5));
        assert_eq!(current, native_f64(0.5));
    }

    #[test]
    fn accelerates_true_ramps_current_fraction_upward() {
        let mut drive = DriveLocomotionRuntime::default();
        let mut current = NativeF64Bits::POSITIVE_ZERO;

        update_drive_speed_fraction(
            &mut drive,
            &mut current,
            NativeF64Bits::ONE,
            true,
            10,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1000,
        );

        assert_eq!(drive.target_speed_fraction, NativeF64Bits::ONE);
        assert_eq!(current, native_f64(0.03));
    }

    #[test]
    fn accelerates_true_brakes_by_raw_speed_scaled_decel_with_floor() {
        let mut drive = DriveLocomotionRuntime::default();
        let mut current = native_f64(0.5);

        update_drive_speed_fraction(
            &mut drive,
            &mut current,
            NativeF64Bits::ONE,
            true,
            10,
            native_f64(0.03),
            native_f64(0.002),
            500,
            499,
        );

        assert_eq!(native_value(current), 0.5 - 10.0 * 0.002);
    }

    #[test]
    fn accelerates_true_braking_uses_strict_slowdown_distance() {
        let mut drive = DriveLocomotionRuntime::default();
        let mut current = native_f64(0.5);

        update_drive_speed_fraction(
            &mut drive,
            &mut current,
            NativeF64Bits::ONE,
            true,
            10,
            native_f64(0.03),
            native_f64(0.002),
            500,
            500,
        );

        assert_eq!(current, NativeF64Bits::from_bits(0x3fe0_f5c2_8f5c_28f5));
    }

    #[test]
    fn active_speed_consumer_keeps_all_three_integer_boundaries() {
        let stock_veteran = NativeF64Bits::from_bits(0x3ff3_3333_4000_0000);
        let stock_crate = NativeF64Bits::from_bits(0x3ff3_3333_3333_3333);
        assert_eq!(
            foot_current_speed(
                17,
                NativeF32Bits::ONE,
                NativeF64Bits::ONE,
                false,
                stock_veteran,
                NativeF64Bits::ONE,
            ),
            17
        );
        assert_eq!(
            foot_current_speed(
                17,
                NativeF32Bits::ONE,
                NativeF64Bits::ONE,
                true,
                stock_veteran,
                NativeF64Bits::ONE,
            ),
            20
        );
        assert_eq!(
            foot_current_speed(
                17,
                NativeF32Bits::ONE,
                stock_crate,
                true,
                stock_veteran,
                NativeF64Bits::ONE,
            ),
            24
        );
        assert_eq!(
            foot_current_speed(
                21,
                NativeF32Bits::ONE,
                NativeF64Bits::from_bits(0.9f64.to_bits()),
                true,
                stock_veteran,
                NativeF64Bits::ONE,
            ),
            21,
            "18.9 truncates to 18 before VeteranSpeed, so the result is 21 rather than fused 22"
        );
    }

    #[test]
    fn active_infantry_wrapper_is_signed_and_wrapping() {
        assert_eq!(infantry_current_speed(-17, true, true), -12);
        assert_eq!(infantry_current_speed(-17, true, false), -25);
        assert_eq!(infantry_current_speed(i32::MAX, true, true), 1_431_655_765);
        assert_eq!(infantry_current_speed(i32::MAX, true, false), -1_073_741_826);
        assert_eq!(infantry_current_speed(i32::MIN, true, true), -1_431_655_766);
        assert_eq!(infantry_current_speed(i32::MIN, true, false), 1_073_741_824);
    }

    #[test]
    fn active_fraction_setter_preserves_only_strict_ordered_interior_bits() {
        for (input, expected) in [
            (0x0000_0000_0000_0000, 0x0000_0000_0000_0000),
            (0x8000_0000_0000_0000, 0x0000_0000_0000_0000),
            (0x3ff0_0000_0000_0000, 0x3ff0_0000_0000_0000),
            (0x3ff0_0000_0000_0001, 0x3ff0_0000_0000_0000),
            (0x0000_0000_0000_0001, 0x0000_0000_0000_0001),
            (0x8000_0000_0000_0001, 0x0000_0000_0000_0000),
            (0x7ff0_0000_0000_0000, 0x3ff0_0000_0000_0000),
            (0xfff0_0000_0000_0000, 0x0000_0000_0000_0000),
            (0x7ff8_0000_0000_0001, 0x0000_0000_0000_0000),
            (0xfff0_0000_0000_0001, 0x0000_0000_0000_0000),
            (0x3fd8_0000_0000_0001, 0x3fd8_0000_0000_0001),
        ] {
            assert_eq!(
                normalize_current_speed_fraction(NativeF64Bits::from_bits(input)).bits(),
                expected,
                "input {input:#018x}"
            );
        }
    }

    #[test]
    fn active_target_producer_widens_f32_and_applies_zero_unordered_negative_and_damage() {
        let row = |track: NativeF32Bits| {
            SpeedCostProfile::default().with_native_values([
                NativeF32Bits::ONE,
                track,
                NativeF32Bits::ONE,
                NativeF32Bits::ONE,
                NativeF32Bits::ONE,
                NativeF32Bits::ONE,
                NativeF32Bits::ONE,
                NativeF32Bits::ONE,
            ])
        };
        let sample = |track: NativeF32Bits, damaged: bool| {
            let terrain = ResolvedTerrainGrid::from_cells(
                2,
                1,
                vec![terrain_cell(0, 0, row(NativeF32Bits::ONE)), terrain_cell(1, 0, row(track))],
            );
            compute_drive_target_speed_fraction(
                SpeedType::Track,
                LocomotorKind::Drive,
                (0, 0),
                (1, 0),
                (128, 128),
                false,
                true,
                &terrain,
                &TerrainSpeedConfig::default(),
                damaged,
            )
        };
        let seventy = sample(NativeF32Bits::from_bits(0.7f32.to_bits()), false);
        assert_eq!(seventy.bits(), 0x3fe6_6666_6000_0000);
        assert_eq!(sample(NativeF32Bits::POSITIVE_ZERO, false), NativeF64Bits::HALF);
        assert_eq!(
            sample(NativeF32Bits::from_bits(0x7fc0_1234), false),
            NativeF64Bits::HALF
        );
        assert_eq!(
            sample(NativeF32Bits::from_bits((-0.25f32).to_bits()), false).bits(),
            (-0.25f64).to_bits()
        );
        assert_eq!(
            sample(NativeF32Bits::from_bits(0.7f32.to_bits()), true),
            native_fraction_product(
                seventy,
                NativeF64Bits::from_bits(0.75f64.to_bits())
            )
        );
    }

    #[test]
    fn active_target_producer_uses_signed_level_road_override_and_on_bridge_plus_four() {
        let row = |track: f32| {
            SpeedCostProfile::default().with_native_values([
                NativeF32Bits::ONE,
                NativeF32Bits::from_bits(track.to_bits()),
                NativeF32Bits::ONE,
                NativeF32Bits::ONE,
                NativeF32Bits::ONE,
                NativeF32Bits::ONE,
                NativeF32Bits::ONE,
                NativeF32Bits::ONE,
            ])
        };
        let mut config = TerrainSpeedConfig::default();
        config.road_speed_costs = row(0.4);
        config.tracked_uphill_native = NativeF64Bits::ONE;
        config.tracked_downhill_native = NativeF64Bits::ONE;
        let mut current = terrain_cell(0, 0, row(1.0));
        current.level = 0xff;
        let mut next = terrain_cell(1, 0, row(0.7));
        next.level = 1;
        let terrain = ResolvedTerrainGrid::from_cells(2, 1, vec![current, next]);
        let signed = compute_drive_target_speed_fraction(
            SpeedType::Track,
            LocomotorKind::Drive,
            (0, 0),
            (1, 0),
            (128, 128),
            false,
            true,
            &terrain,
            &config,
            false,
        );
        assert_eq!(signed.bits(), (0.4f32 as f64).to_bits());

        let terrain = ResolvedTerrainGrid::from_cells(
            2,
            1,
            vec![terrain_cell(0, 0, row(1.0)), terrain_cell(1, 0, row(0.7))],
        );
        let bridged = compute_drive_target_speed_fraction(
            SpeedType::Track,
            LocomotorKind::Drive,
            (0, 0),
            (1, 0),
            (128, 128),
            true,
            true,
            &terrain,
            &config,
            false,
        );
        assert_eq!(bridged.bits(), (0.4f32 as f64).to_bits());
    }

    #[test]
    fn forced_selector_preserves_target_and_routes_process_result_to_owner_only() {
        let original_target = native_f64(0.375);
        let mut drive = DriveLocomotionRuntime {
            track_index: 67,
            target_speed_fraction: original_target,
            ..Default::default()
        };
        let mut current = native_f64(0.25);
        update_drive_speed_fraction(
            &mut drive,
            &mut current,
            native_f64(0.7),
            true,
            17,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1000,
        );
        assert_eq!(drive.target_speed_fraction, original_target);
        assert_eq!(current, native_f64(0.7));

        update_drive_speed_fraction(
            &mut drive,
            &mut current,
            NativeF64Bits::from_bits(0x7ff8_0000_0000_1234),
            true,
            17,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1000,
        );
        assert_eq!(drive.target_speed_fraction, original_target);
        assert_eq!(current, native_f64(0.7), "unordered compare skips owner write");
    }

    #[test]
    fn active_ramp_closes_passive_alternate_crush_and_unordered_control_edges() {
        let mut drive = DriveLocomotionRuntime::default();
        let mut current = native_f64(0.4);
        update_drive_speed_fraction_with_flags(
            &mut drive,
            &mut current,
            native_f64(0.8),
            true,
            20,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1_000,
            VehicleRampFlags {
                passive: true,
                ..Default::default()
            },
        );
        assert_eq!(drive.target_speed_fraction, native_f64(0.8));
        assert_eq!(current, native_f64(0.4), "Passive skips the owner setter");

        update_drive_speed_fraction_with_flags(
            &mut drive,
            &mut current,
            native_f64(0.8),
            true,
            20,
            native_f64(0.03),
            native_f64(0.002),
            500,
            500,
            VehicleRampFlags {
                alternate_brake: true,
                ..Default::default()
            },
        );
        assert_eq!(
            current,
            native_sub_scaled(native_f64(0.4), 20, DRIVE_ALTERNATE_BRAKE_RATE),
            "the 500 boundary excludes the arrival band and selects +0x3CD braking"
        );

        current = native_f64(0.11);
        update_drive_speed_fraction_with_flags(
            &mut drive,
            &mut current,
            native_f64(0.8),
            true,
            20,
            native_f64(0.03),
            native_f64(0.002),
            500,
            500,
            VehicleRampFlags {
                alternate_brake: true,
                ..Default::default()
            },
        );
        assert_eq!(current, DRIVE_ALTERNATE_BRAKE_FLOOR);

        current = native_f64(0.7);
        update_drive_speed_fraction_with_flags(
            &mut drive,
            &mut current,
            native_f64(0.8),
            true,
            20,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1_000,
            VehicleRampFlags {
                currently_crushing: true,
                ..Default::default()
            },
        );
        assert_eq!(drive.target_speed_fraction, DRIVE_CRUSH_FRACTION);
        assert_eq!(current, DRIVE_CRUSH_FRACTION);

        current = NativeF64Bits::from_bits(0x7ff8_0000_0000_4321);
        update_drive_speed_fraction(
            &mut drive,
            &mut current,
            native_f64(0.6),
            true,
            20,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1_000,
        );
        assert_eq!(current, native_f64(0.6), "unordered acceleration caps to T");

        current = native_f64(0.4);
        update_drive_speed_fraction(
            &mut drive,
            &mut current,
            NativeF64Bits::from_bits(0x7ff8_0000_0000_5678),
            true,
            20,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1_000,
        );
        assert_eq!(current, NativeF64Bits::POSITIVE_ZERO);
    }

    #[test]
    fn active_ship_forced_selector_matches_drive_owner_only_routing() {
        let original_target = native_f64(0.375);
        let mut ship = ShipLocomotionRuntime {
            track_index: 71,
            target_speed_fraction: original_target,
            ..Default::default()
        };
        let mut current = native_f64(0.25);
        update_ship_speed_fraction(
            &mut ship,
            &mut current,
            native_f64(0.7),
            true,
            17,
            native_f64(0.03),
            native_f64(0.002),
            500,
            1_000,
        );
        assert_eq!(ship.target_speed_fraction, original_target);
        assert_eq!(current, native_f64(0.7));
    }

    #[test]
    fn active_stored_destination_distance_uses_structural_bridge_z_and_zyx_order() {
        let profile = SpeedCostProfile::default().with_native_values([NativeF32Bits::ONE; 8]);
        let flat = terrain_cell(0, 0, profile.clone());
        let mut bridge = terrain_cell(1, 0, profile);
        bridge.bridge_facts.raw_flags = crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
        let terrain = ResolvedTerrainGrid::from_cells(2, 1, vec![flat, bridge]);
        let owner = DriveCoord { x: 284, y: 128, z: 0 };
        let destination = DriveCoord { x: 384, y: 128, z: 0 };

        assert_eq!(
            stored_destination_distance(owner, destination, Some(&terrain)),
            427,
            "sqrt_approx(chop((416^2 + 0^2) + 100^2))"
        );
        assert_eq!(stored_destination_distance(owner, destination, None), 100);
    }
}
