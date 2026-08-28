//! Eight positive-weight retail crate effects and their presentation tails.

use crate::map::entities::EntityCategory;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::crate_rules::CrateEffect;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::{EntityDamageEvent, RAD_NO_ATTACKER, ReceiverCallFlags};
use crate::sim::components::AnimClassSpawnDescriptor;
use crate::sim::find_nearby_cell::{
    NearbyAnchorGate, NearbyFootprint, NearbyQuery, PassabilityArgs,
    find_nearby_passable_cell, map_owned_radius_cap,
};
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::{
    CrateUpgradeEvaKind, PlacementEvidence, RevealOutcome, RevealPosition, RevealRequest,
    SimSoundEvent, Simulation,
};
use crate::util::fixed_math::SimFixed;
use crate::util::lepton::CELL_CENTER_LEPTON;
use crate::util::native_x87::{NativeF64Bits, X87Chop53, X87Ordering, distance_3d_leptons};

use super::pickup::{NativePickupReturn, SelectedPowerup};

pub(super) fn execute(
    sim: &mut Simulation,
    cell: (u16, u16),
    collector_id: u64,
    selected: SelectedPowerup,
    rules: &RuleSet,
    overlays: &OverlayTypeRegistry,
    path_grid: Option<&PathGrid>,
) -> NativePickupReturn {
    let (result, common_effect) = match selected.effect {
        CrateEffect::Money => (
            money(sim, cell, collector_id, selected.data, rules),
            selected.effect,
        ),
        CrateEffect::Unit => {
            unit(sim, cell, collector_id, selected.data, rules, overlays, path_grid)
        }
        CrateEffect::HealBase => (
            heal_base(sim, cell, collector_id, rules, overlays),
            selected.effect,
        ),
        CrateEffect::Reveal => (
            reveal(sim, cell, collector_id, rules),
            selected.effect,
        ),
        CrateEffect::Armor => (
            radius_multiplier(
                sim,
                cell,
                collector_id,
                selected.data,
                rules,
                MultiplierKind::Armor,
            ),
            selected.effect,
        ),
        CrateEffect::Speed => (
            radius_multiplier(
                sim,
                cell,
                collector_id,
                selected.data,
                rules,
                MultiplierKind::Speed,
            ),
            selected.effect,
        ),
        CrateEffect::Firepower => (
            radius_multiplier(
                sim,
                cell,
                collector_id,
                selected.data,
                rules,
                MultiplierKind::Firepower,
            ),
            selected.effect,
        ),
        CrateEffect::Veteran => (
            veteran(sim, cell, collector_id, selected.data, rules),
            selected.effect,
        ),
        // Installed retail assigns zero weight to these eleven slots. They
        // remain typed/selectable for map/action fidelity, but are outside the
        // active-retail effect set closed by this Phase-3 prerequisite.
        _ => (NativePickupReturn::One, selected.effect),
    };
    if !(selected.effect == CrateEffect::Unit && result == NativePickupReturn::Zero) {
        common_animation(sim, cell, common_effect, rules);
    }
    result
}

fn money(
    sim: &mut Simulation,
    cell: (u16, u16),
    collector_id: u64,
    data: NativeF64Bits,
    rules: &RuleSet,
) -> NativePickupReturn {
    let Some(picker_owner) = sim.entities().get(collector_id).map(|entity| entity.owner) else {
        return NativePickupReturn::One;
    };
    let amount = if sim.session.game_mode_nonzero {
        let base = ftol_low_i32(data);
        sim.scenario_rng
            .next_range_i32_inclusive(base, base.wrapping_add(900))
    } else {
        rules.crate_rules.solo_money
    };
    // Offline Campaign has one local player pointer. Keep the native special
    // case explicit instead of changing the ordinary picker-owner path.
    let credit_owner = if !sim.session.game_mode_nonzero
        && owner_is_local_player(sim, picker_owner)
    {
        sim.session
            .house_order
            .iter()
            .copied()
            .find(|owner| owner_is_local_player(sim, *owner))
            .unwrap_or(picker_owner)
    } else {
        picker_owner
    };
    if let Some(house) = sim.houses.get_mut(&credit_owner) {
        house.credits = house.credits.wrapping_add(amount);
    }
    emit_picker_sound(
        sim,
        picker_owner,
        cell,
        rules.crate_rules.crate_money_sound.as_deref(),
    );
    NativePickupReturn::One
}

fn unit(
    sim: &mut Simulation,
    cell: (u16, u16),
    collector_id: u64,
    unit_data: NativeF64Bits,
    rules: &RuleSet,
    _overlays: &OverlayTypeRegistry,
    _path_grid: Option<&PathGrid>,
) -> (NativePickupReturn, CrateEffect) {
    let Some(owner) = sim.entities().get(collector_id).map(|entity| entity.owner) else {
        return (NativePickupReturn::One, CrateEffect::Unit);
    };
    let owner_name = sim.resolve(owner).to_owned();
    let free_mcv = sim.houses.get(&owner).is_some_and(|house| {
        house.owned_building_count == 0
            && house.credits > 1500
            && sim.session.game_options.bases
    });
    let chosen = if let Some(forced) = rules.crate_rules.unit_crate_type.as_deref() {
        rules.object(forced).map(|object| object.id.clone())
    } else {
        let units = rules.unit_types_in_order().collect::<Vec<_>>();
        if units.is_empty() {
            None
        } else {
            loop {
                let index = sim
                    .scenario_rng
                    .next_range_u32_inclusive(0, (units.len() - 1) as u32)
                    as usize;
                let object = units[index];
                let is_base = rules
                    .general
                    .base_unit_types
                    .iter()
                    .any(|type_id| object.id.eq_ignore_ascii_case(type_id));
                let human = owner_is_human_player(sim, owner);
                if object.crate_goodie
                    && (!is_base || (sim.session.game_options.bases && (human || free_mcv)))
                {
                    break Some(object.id.clone());
                }
            }
        }
    };
    let Some(chosen) = chosen else {
        return (NativePickupReturn::One, CrateEffect::Unit);
    };
    let Some(chosen_type) = rules.object(&chosen) else {
        return (NativePickupReturn::One, CrateEffect::Unit);
    };
    let z = cell_ground_height(sim, cell).div_euclid(104).clamp(0, 255) as u8;
    let Some(child_id) = sim.spawn_object_limbo_at_height(
        &chosen,
        &owner_name,
        cell.0,
        cell.1,
        0,
        z,
        rules,
    ) else {
        return (NativePickupReturn::One, CrateEffect::Unit);
    };
    if let Some(child) = sim.entities_mut().get_mut(child_id) {
        child.position.sub_x = SimFixed::from_num(CELL_CENTER_LEPTON);
        child.position.sub_y = SimFixed::from_num(CELL_CENTER_LEPTON);
    }

    let exact_success = try_unlimbo_crate_unit(sim, child_id, cell);
    let nearby_success = if exact_success {
        false
    } else {
        let query = NearbyQuery {
            passability: PassabilityArgs {
                speed_type: chosen_type.speed_type,
                required_zone_id: None,
                movement_zone: chosen_type.movement_zone,
                bridge_aware_zone: false,
            },
            footprint: NearbyFootprint::SINGLE,
            anchor_gate: NearbyAnchorGate::NativeHeightAware,
            allow_bridge_cells: true,
            check_height: false,
            check_occupancy: true,
            radius_cap: map_owned_radius_cap(
                i32::from(sim.session.map_width),
                i32::from(sim.session.map_height),
            ),
            target_cell: None,
            path_grid: _path_grid,
            resolved_terrain: sim.resolved_terrain.as_ref(),
            overlay_grid: sim.overlay_grid.as_ref(),
            occupancy: Some(&sim.substrate.occupancy),
            entities: Some(&sim.substrate.entities),
            zone_grid: sim.zone_grid.as_ref(),
            playfield_bounds: sim.playfield_bounds,
        };
        let nearby = find_nearby_passable_cell(
            (i32::from(cell.0), i32::from(cell.1)),
            &query,
            sim.session.binary_frame,
        );
        nearby.is_some_and(|nearby| try_unlimbo_crate_unit(sim, child_id, nearby))
    };
    if exact_success || nearby_success {
        emit_picker_sound(
            sim,
            owner,
            cell,
            rules.crate_rules.crate_unit_sound.as_deref(),
        );
        (NativePickupReturn::Zero, CrateEffect::Unit)
    } else {
        // A constructed child whose exact and one-nearby Unlimbo both fail
        // falls back through Money using Unit's already-loaded data.
        sim.uninit_with_rules(child_id, rules);
        (
            money(sim, cell, collector_id, unit_data, rules),
            CrateEffect::Money,
        )
    }
}

fn try_unlimbo_crate_unit(sim: &mut Simulation, child_id: u64, cell: (u16, u16)) -> bool {
    let admitted = sim
        .substrate
        .occupancy
        .get(cell.0, cell.1)
        .is_none_or(|occupancy| !occupancy.has_blockers_on(MovementLayer::Ground));
    let z = sim
        .resolved_terrain
        .as_ref()
        .and_then(|terrain| terrain.cell(cell.0, cell.1))
        .map_or(0, |terrain| terrain.level as u8);
    matches!(
        sim.try_reveal_entity(
            child_id,
            RevealRequest {
                position: RevealPosition {
                    rx: cell.0,
                    ry: cell.1,
                    z,
                    sub_x: SimFixed::from_num(CELL_CENTER_LEPTON),
                    sub_y: SimFixed::from_num(CELL_CENTER_LEPTON),
                },
                placement: if admitted {
                    PlacementEvidence::EvaluateMark
                } else {
                    PlacementEvidence::MarkFailed
                },
                logic_eligible: true,
            }
        ),
        RevealOutcome::Revealed { .. }
    )
}

fn heal_base(
    sim: &mut Simulation,
    cell: (u16, u16),
    collector_id: u64,
    rules: &RuleSet,
    overlays: &OverlayTypeRegistry,
) -> NativePickupReturn {
    let Some(owner) = sim.entities().get(collector_id).map(|entity| entity.owner) else {
        return NativePickupReturn::One;
    };
    emit_picker_sound(
        sim,
        owner,
        cell,
        rules.crate_rules.heal_crate_sound.as_deref(),
    );
    let warhead = sim.rule_handles().c4;
    let mut index = 0usize;
    while index < sim.tactical_registration_order().len() {
        let stable_id = sim.tactical_registration_order()[index];
        let event = sim.entities().get(stable_id).and_then(|entity| {
            (entity.owner == owner).then(|| {
                EntityDamageEvent::direct_receiver(
                    stable_id,
                    i32::from(entity.health.current).wrapping_sub(i32::from(entity.health.max)),
                    0,
                    RAD_NO_ATTACKER,
                    None,
                    warhead,
                    ReceiverCallFlags {
                        ignore_defenses: true,
                        arg6: true,
                    },
                )
            })
        });
        if let Some(event) = event {
            sim.commit_noncombat_aoe_hits(rules, Some(overlays), &[event]);
        }
        index += 1;
    }
    NativePickupReturn::One
}

fn reveal(
    sim: &mut Simulation,
    cell: (u16, u16),
    collector_id: u64,
    rules: &RuleSet,
) -> NativePickupReturn {
    let Some(owner) = sim.entities().get(collector_id).map(|entity| entity.owner) else {
        return NativePickupReturn::One;
    };
    let local = owner_is_local_player(sim, owner);
    if let Some(house) = sim.houses.get_mut(&owner) {
        house.map_is_clear = true;
    } else {
        return NativePickupReturn::One;
    }
    let already = local && sim.houses.get(&owner).is_some_and(|house| house.visionary);
    if local && !already {
        // Rust recomputes every Techno sight contribution each master frame;
        // that is the persistent counterpart of native's bracketing Paranoid
        // reveal/unreveal passes. The direct CellClass write remains a distinct
        // same-stack operation here.
        if let Some(house) = sim.houses.get_mut(&owner) {
            house.visionary = true;
        }
        let _ = sim.fog.blackout_shroud_for_owner(
            owner,
            sim.session.map_width,
            sim.session.map_height,
        );
        sim.radar_terrain_dirty_generation = sim.radar_terrain_dirty_generation.wrapping_add(1);
        sim.playfield_revision = sim.playfield_revision.wrapping_add(1);
    }
    if let Some(sound_id) = rules.crate_rules.crate_reveal_sound.as_deref() {
        sim.sound_events.push(SimSoundEvent::CrateEffect {
            sound_id: sound_id.to_owned(),
            owner,
            rx: cell.0,
            ry: cell.1,
        });
    }
    NativePickupReturn::One
}

#[derive(Clone, Copy)]
enum MultiplierKind {
    Armor,
    Speed,
    Firepower,
}

fn radius_multiplier(
    sim: &mut Simulation,
    cell: (u16, u16),
    collector_id: u64,
    data: NativeF64Bits,
    rules: &RuleSet,
    kind: MultiplierKind,
) -> NativePickupReturn {
    let Some(owner) = sim.entities().get(collector_id).map(|entity| entity.owner) else {
        return NativePickupReturn::One;
    };
    let origin = crate_center_world(sim, cell);
    let mut index = 0usize;
    let mut eva_needed = false;
    while index < sim.tactical_registration_order().len() {
        let stable_id = sim.tactical_registration_order()[index];
        let admitted = sim.entities().get(stable_id).is_some_and(|entity| {
            let candidate = entity_world(sim, entity);
            distance_3d_leptons(origin, candidate) < rules.crate_rules.radius_leptons
                && match kind {
                    MultiplierKind::Armor => {
                        entity.armor_multiplier.bits() == NativeF64Bits::ONE.bits()
                    }
                    MultiplierKind::Speed => {
                        entity.category != EntityCategory::Aircraft
                            && entity.locomotor.is_some()
                            && entity.speed_crate_multiplier.bits() == NativeF64Bits::ONE.bits()
                    }
                    MultiplierKind::Firepower => {
                        entity.firepower_crate_multiplier.bits() == NativeF64Bits::ONE.bits()
                    }
                }
        });
        if admitted {
            let affected_owner = sim.entities().get(stable_id).map(|entity| entity.owner);
            eva_needed |= affected_owner
                .and_then(|owner| sim.houses.get(&owner))
                .is_some_and(|house| match kind {
                    MultiplierKind::Speed => house.player_control,
                    MultiplierKind::Armor | MultiplierKind::Firepower => {
                        house.is_controlled_by_human(sim.session.game_mode_nonzero)
                    }
                });
        }
        if admitted && let Some(entity) = sim.entities_mut().get_mut(stable_id) {
            match kind {
                MultiplierKind::Armor => {
                    entity.armor_multiplier = x87_mul_bits(entity.armor_multiplier, data)
                }
                MultiplierKind::Speed => {
                    entity.speed_crate_multiplier =
                        x87_mul_bits(entity.speed_crate_multiplier, data)
                }
                MultiplierKind::Firepower => {
                    entity.firepower_crate_multiplier =
                        x87_mul_bits(entity.firepower_crate_multiplier, data)
                }
            }
        }
        index += 1;
    }
    if eva_needed {
        sim.sound_events.push(SimSoundEvent::CrateUpgradeEva {
            kind: match kind {
                MultiplierKind::Armor => CrateUpgradeEvaKind::Armor,
                MultiplierKind::Speed => CrateUpgradeEvaKind::Speed,
                MultiplierKind::Firepower => CrateUpgradeEvaKind::Firepower,
            },
        });
    }
    let sound = match kind {
        MultiplierKind::Armor => rules.crate_rules.crate_armour_sound.as_deref(),
        MultiplierKind::Speed => rules.crate_rules.crate_speed_sound.as_deref(),
        MultiplierKind::Firepower => rules.crate_rules.crate_fire_sound.as_deref(),
    };
    emit_picker_sound(sim, owner, cell, sound);
    NativePickupReturn::One
}

fn veteran(
    sim: &mut Simulation,
    cell: (u16, u16),
    collector_id: u64,
    data: NativeF64Bits,
    rules: &RuleSet,
) -> NativePickupReturn {
    let Some(owner) = sim.entities().get(collector_id).map(|entity| entity.owner) else {
        return NativePickupReturn::One;
    };
    let origin = crate_center_world(sim, cell);
    let mut index = 0usize;
    while index < sim.tactical_registration_order().len() {
        let stable_id = sim.tactical_registration_order()[index];
        let admitted = sim.entities().get(stable_id).is_some_and(|entity| {
            entity.lifecycle.object_alive
                && entity.lifecycle.cell_marked
                && !entity.dying
                && x87_bits_positive(data)
                && distance_3d_leptons(origin, entity_world(sim, entity))
                    < rules.crate_rules.radius_leptons
                && rules
                    .object(sim.resolve(entity.type_ref))
                    .is_some_and(|object| object.trainable)
        });
        if admitted && let Some(entity) = sim.entities_mut().get_mut(stable_id) {
            let mut iteration = 0_i32;
            while x87_int_less_than_bits(iteration, data) {
                let raw = f32::from_bits(entity.veterancy_raw.bits());
                entity.veterancy_raw = if (1.0..2.0).contains(&raw) {
                    crate::util::native_x87::NativeF32Bits::from_bits(2.0_f32.to_bits())
                } else if (0.0..1.0).contains(&raw) {
                    crate::util::native_x87::NativeF32Bits::from_bits(1.0_f32.to_bits())
                } else if raw < 0.0 {
                    crate::util::native_x87::NativeF32Bits::POSITIVE_ZERO
                } else {
                    entity.veterancy_raw
                };
                entity.veterancy =
                    crate::sim::combat::veterancy::rank_u16(entity.veterancy_raw);
                iteration = iteration.wrapping_add(1);
            }
        }
        index += 1;
    }
    emit_picker_sound(
        sim,
        owner,
        cell,
        rules.crate_rules.crate_promote_sound.as_deref(),
    );
    NativePickupReturn::One
}

fn common_animation(sim: &mut Simulation, cell: (u16, u16), effect: CrateEffect, rules: &RuleSet) {
    let Some(animation) = rules.crate_rules.powerups[effect as usize].animation.as_deref() else {
        return;
    };
    let type_name = sim.interner.intern(animation);
    let mut descriptor = AnimClassSpawnDescriptor::new(
        type_name,
        cell.0,
        cell.1,
        CELL_CENTER_LEPTON,
        CELL_CENTER_LEPTON,
        0,
    );
    descriptor.draw_flags = 0x600;
    let center = crate_center_world(sim, cell);
    let _ = sim.spawn_anim_at_world(
        rules,
        descriptor,
        crate::sim::anim_class::AnimWorldCoord {
            x: center[0],
            y: center[1],
            z: center[2].wrapping_add(200),
        },
    );
}

fn emit_picker_sound(
    sim: &mut Simulation,
    owner: crate::sim::intern::InternedId,
    cell: (u16, u16),
    sound: Option<&str>,
) {
    if owner_is_human_player(sim, owner) && let Some(sound_id) = sound {
        sim.sound_events.push(SimSoundEvent::CrateEffect {
            sound_id: sound_id.to_owned(),
            owner,
            rx: cell.0,
            ry: cell.1,
        });
    }
}

/// `HouseClass::IsHumanPlayer` is mode-aware: Campaign uses the local
/// PlayerControl byte while multiplayer uses the session human-seat byte.
fn owner_is_human_player(sim: &Simulation, owner: crate::sim::intern::InternedId) -> bool {
    sim.houses
        .get(&owner)
        .is_some_and(|house| house.is_controlled_by_human(sim.session.game_mode_nonzero))
}

/// Exact `g_PlayerPtr` identity predicate. Reveal's remote early return and
/// mode-zero Money redirect read this pointer, not `IsHumanPlayer`.
fn owner_is_local_player(sim: &Simulation, owner: crate::sim::intern::InternedId) -> bool {
    sim.houses
        .get(&owner)
        .is_some_and(|house| house.player_control)
}

fn crate_center_world(sim: &Simulation, cell: (u16, u16)) -> [i32; 3] {
    [
        i32::from(cell.0).wrapping_mul(256).wrapping_add(128),
        i32::from(cell.1).wrapping_mul(256).wrapping_add(128),
        cell_ground_height(sim, cell),
    ]
}

fn cell_ground_height(sim: &Simulation, cell: (u16, u16)) -> i32 {
    let x = i32::from(cell.0).wrapping_mul(256).wrapping_add(128);
    let y = i32::from(cell.1).wrapping_mul(256).wrapping_add(128);
    sim.resolved_terrain
        .as_ref()
        .and_then(|terrain| terrain.cell(cell.0, cell.1))
        .and_then(|terrain| {
            crate::util::lepton::ground_height_leptons(terrain.level, terrain.slope_type, x, y).ok()
        })
        .unwrap_or(0)
}

fn entity_world(sim: &Simulation, entity: &crate::sim::game_entity::GameEntity) -> [i32; 3] {
    [
        i32::from(entity.position.rx)
            .wrapping_mul(256)
            .wrapping_add(entity.position.sub_x.to_num::<i32>()),
        i32::from(entity.position.ry)
            .wrapping_mul(256)
            .wrapping_add(entity.position.sub_y.to_num::<i32>()),
        crate::sim::combat::object_world_z_leptons(entity, sim.resolved_terrain.as_ref()),
    ]
}

fn ftol_low_i32(bits: NativeF64Bits) -> i32 {
    let value = X87Chop53::load_f64(bits).expect("typed Powerups data is finite binary64");
    X87Chop53::ftol_i64(value).unwrap_or(i64::MIN) as i32
}

fn x87_mul_bits(lhs: NativeF64Bits, rhs: NativeF64Bits) -> NativeF64Bits {
    let lhs = X87Chop53::load_f64(lhs).expect("crate multiplier is finite");
    let rhs = X87Chop53::load_f64(rhs).expect("typed Powerups data is finite");
    X87Chop53::store_f64(X87Chop53::mul(lhs, rhs)).expect("crate product fits binary64")
}

fn x87_int_less_than_bits(lhs: i32, rhs: NativeF64Bits) -> bool {
    let rhs = X87Chop53::load_f64(rhs).expect("typed Powerups data is finite");
    X87Chop53::compare(X87Chop53::load_i32(lhs), rhs) == X87Ordering::Less
}

fn x87_bits_positive(value: NativeF64Bits) -> bool {
    let value = X87Chop53::load_f64(value).expect("typed Powerups data is finite");
    X87Chop53::compare(value, X87Chop53::load_i32(0)) == X87Ordering::Greater
}
