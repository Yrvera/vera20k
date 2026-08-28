//! Synchronous CrateClass pickup prefix, selection, and guard transaction.

use crate::map::entities::EntityCategory;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::crate_rules::{CrateEffect, POWERUP_COUNT};
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_rules::LandType;
use crate::sim::intern::InternedId;
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::Simulation;
use crate::util::native_x87::NativeF64Bits;

use super::effects;

/// Literal native control return. Zero is not synonymous with consumption:
/// Event-49 death and successful Unit placement both return it for different
/// reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePickupReturn {
    Zero,
    One,
}

/// Borrowed static/context inputs for one synchronous pickup. Persistent
/// authority remains on Simulation and RuleSet; this struct owns no state.
pub(crate) struct CratePickupInputs<'a> {
    pub(crate) rules: &'a RuleSet,
    pub(crate) overlays: &'a OverlayTypeRegistry,
    pub(crate) path_grid: Option<&'a PathGrid>,
    /// Installed by the ordered Tag runtime slice. It may mutate or uninit the
    /// collector; the transaction always re-fetches by stable ID afterwards.
    pub(crate) event_49: Option<&'a mut dyn CrateEvent49Dispatch>,
}

pub(crate) trait CrateEvent49Dispatch {
    fn raise(&mut self, sim: &mut Simulation, collector_id: u64, tag_id: InternedId);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SelectedPowerup {
    pub(super) effect: CrateEffect,
    pub(super) data: NativeF64Bits,
}

#[derive(Debug, Clone, Copy)]
struct CollectorSnapshot {
    owner: InternedId,
    attached_tag_id: Option<InternedId>,
    category: EntityCategory,
    armor_bits: u64,
    speed_bits: u64,
    firepower_bits: u64,
    veterancy: u16,
    cloak_modifier: bool,
    firepower_capable: bool,
    trainable: bool,
}

impl Simulation {
    /// `CrateClass__PickupDispatch @ 0x00481A00`.
    pub(crate) fn pickup_crate_at(
        &mut self,
        cell: (u16, u16),
        collector_id: u64,
        mut inputs: CratePickupInputs<'_>,
    ) -> NativePickupReturn {
        let Some((overlay_id, overlay_data, flags)) = self
            .overlay_grid
            .as_ref()
            .filter(|grid| cell.0 < grid.width() && cell.1 < grid.height())
            .and_then(|grid| {
                let cell = grid.cell(cell.0, cell.1);
                let id = cell.overlay_id?;
                Some((id, cell.overlay_data, inputs.overlays.flags(id)?))
            })
        else {
            return NativePickupReturn::One;
        };
        if !flags.crate_type {
            return NativePickupReturn::One;
        }

        let Some(mut collector) = collector_snapshot(self, collector_id, inputs.rules) else {
            return NativePickupReturn::One;
        };
        if self.session.game_mode_nonzero
            && self
                .houses
                .get(&collector.owner)
                .is_some_and(|house| house.multiplay_passive)
        {
            return NativePickupReturn::One;
        }

        if flags.crate_trigger {
            if let (Some(tag_id), Some(dispatch)) =
                (collector.attached_tag_id, inputs.event_49.as_deref_mut())
            {
                dispatch.raise(self, collector_id, tag_id);
            }
            let Some(refetched) = collector_snapshot(self, collector_id, inputs.rules) else {
                return NativePickupReturn::Zero;
            };
            if !self.entities().get(collector_id).is_some_and(|entity| {
                entity.lifecycle.object_alive && !entity.dying
            }) {
                return NativePickupReturn::Zero;
            }
            collector = refetched;
            self.crate_authority.pickup_any_latch = true;
        }

        let selected = select_powerup(
            self,
            inputs.rules,
            inputs.overlays,
            overlay_id,
            overlay_data,
        );
        let selected = if self.session.game_mode_nonzero {
            apply_multiplayer_guards(self, inputs.rules, cell, collector, selected)
        } else {
            selected
        };

        // Modes 0/5 own no WOL counter. Raw mode 4 is rejected before
        // Simulation construction by the NativeGameMode boundary slice.
        let _ = self.remove_crate_at_cell(inputs.rules, inputs.overlays, cell);
        if self.session.game_mode_nonzero && self.session.game_options.crates {
            let _ = self.place_random_crate(inputs.rules, inputs.overlays, inputs.path_grid);
        }

        let selected = if selected.effect == CrateEffect::Squad {
            SelectedPowerup {
                effect: CrateEffect::Money,
                data: inputs.rules.crate_rules.powerups[CrateEffect::Money as usize].data,
            }
        } else {
            selected
        };
        effects::execute(self, cell, collector_id, selected, inputs.rules, inputs.overlays, inputs.path_grid)
    }

    /// Release a movement leaf's entity borrow, perform the one synchronous
    /// crate transaction, then re-fetch the collector/tombstone and execute the
    /// verified caller-specific tail.
    pub(crate) fn pickup_crate_from_movement_probe(
        &mut self,
        collector_id: u64,
        probe: crate::sim::movement::crate_callers::MovementCrateProbe,
        inputs: CratePickupInputs<'_>,
    ) -> NativePickupReturn {
        let Some(cell) = probe.cell() else {
            return NativePickupReturn::One;
        };
        let result = self.pickup_crate_at(cell, collector_id, inputs);
        if let Some(entity) = self.substrate.entities.get_mut(collector_id) {
            let _ = crate::sim::movement::crate_callers::continue_after_pickup(
                entity,
                probe,
                result,
            );
        }
        result
    }

    /// Crate-runtime prerequisite for `ParasiteClass::Attach @ 0x0062A980`.
    ///
    /// The later Parasite owner performs admission and calls this only for an
    /// accepted SQD/victim pair. This transaction installs Ship ForceTrack,
    /// releases every entity borrow for synchronous pickup/effects, re-fetches
    /// the deferred-lifetime collector, completes ForceTrack, and finally
    /// installs the two reciprocal Attach links in native order.
    pub(crate) fn pickup_crate_from_sqd_attach(
        &mut self,
        attacker_id: u64,
        victim_id: u64,
        inputs: CratePickupInputs<'_>,
    ) -> Option<NativePickupReturn> {
        let probe = crate::sim::parasite_attachment::begin_sqd_ship_force_track(
            &mut self.substrate.entities,
            attacker_id,
            victim_id,
        )?;
        let cell = probe.cell()?;
        let result = self.pickup_crate_at(cell, attacker_id, inputs);
        crate::sim::parasite_attachment::finish_sqd_attach_after_ship_force_track(
            &mut self.substrate.entities,
            attacker_id,
            victim_id,
            probe,
            result,
        );
        Some(result)
    }
}

fn collector_snapshot(
    sim: &Simulation,
    collector_id: u64,
    rules: &RuleSet,
) -> Option<CollectorSnapshot> {
    let entity = sim.entities().get(collector_id)?;
    let object = rules.object(sim.resolve(entity.type_ref));
    Some(CollectorSnapshot {
        owner: entity.owner,
        attached_tag_id: entity.attached_tag_id,
        category: entity.category,
        armor_bits: entity.armor_multiplier.bits(),
        speed_bits: entity.speed_crate_multiplier.bits(),
        firepower_bits: entity.firepower_crate_multiplier.bits(),
        veterancy: entity.veterancy,
        cloak_modifier: entity.cloak_crate_applied,
        firepower_capable: object.is_some_and(|object| {
            object.primary.is_some() || object.secondary.is_some()
        }),
        trainable: object.is_some_and(|object| object.trainable),
    })
}

fn select_powerup(
    sim: &mut Simulation,
    rules: &RuleSet,
    overlays: &OverlayTypeRegistry,
    overlay_id: u8,
    overlay_data: u8,
) -> SelectedPowerup {
    let effect = if usize::from(overlay_data) < POWERUP_COUNT {
        CrateEffect::ALL[usize::from(overlay_data)]
    } else {
        let total = rules
            .crate_rules
            .powerups
            .iter()
            .fold(0_i32, |sum, entry| sum.wrapping_add(entry.weight));
        let roll = sim
            .scenario_rng
            .next_range_u32_inclusive(1, total as u32) as i32;
        let mut cumulative = 0_i32;
        CrateEffect::ALL
            .into_iter()
            .find(|effect| {
                cumulative = cumulative.wrapping_add(
                    rules.crate_rules.powerups[*effect as usize].weight,
                );
                cumulative >= roll
            })
            .unwrap_or(CrateEffect::Money)
    };

    let effect = if !sim.session.game_mode_nonzero && overlay_data == 0 {
        let crate_id = rules
            .crate_rules
            .crate_img
            .as_deref()
            .and_then(|name| overlays.id_for_name(name));
        let wood_id = rules
            .crate_rules
            .wood_crate_img
            .as_deref()
            .and_then(|name| overlays.id_for_name(name));
        let water_id = rules
            .crate_rules
            .water_crate_img
            .as_deref()
            .and_then(|name| overlays.id_for_name(name));
        let mut mapped = effect;
        if crate_id == Some(overlay_id) {
            mapped = rules.crate_rules.silver_crate;
        }
        if wood_id == Some(overlay_id) {
            mapped = rules.crate_rules.wood_crate;
        }
        if water_id == Some(overlay_id) {
            mapped = rules.crate_rules.water_crate;
        }
        mapped
    } else {
        effect
    };
    SelectedPowerup {
        effect,
        data: rules.crate_rules.powerups[effect as usize].data,
    }
}

fn apply_multiplayer_guards(
    sim: &Simulation,
    rules: &RuleSet,
    cell: (u16, u16),
    collector: CollectorSnapshot,
    mut selected: SelectedPowerup,
) -> SelectedPowerup {
    let Some(house) = sim.houses.get(&collector.owner) else {
        return selected;
    };
    let base_unit = side_base_unit(rules, house.side_index);
    let owns_base_unit = base_unit.is_some_and(|base_type| {
        sim.entities().values().any(|entity| {
            entity.owner == collector.owner
                && entity.category == EntityCategory::Unit
                && entity.lifecycle.object_alive
                && !entity.dying
                && sim.resolve(entity.type_ref).eq_ignore_ascii_case(base_type)
        })
    });
    if house.owned_building_count == 0
        && house.credits > 1500
        && !owns_base_unit
        && sim.session.game_options.bases
    {
        selected.effect = CrateEffect::Unit;
        selected.data = rules.crate_rules.powerups[CrateEffect::Unit as usize].data;
    }

    let owned_units = sim
        .entities()
        .values()
        .filter(|entity| {
            entity.owner == collector.owner
                && entity.category == EntityCategory::Unit
                && entity.lifecycle.object_alive
                && !entity.dying
        })
        .count();
    let owned_infantry = sim
        .entities()
        .values()
        .filter(|entity| {
            entity.owner == collector.owner
                && entity.category == EntityCategory::Infantry
                && entity.lifecycle.object_alive
                && !entity.dying
        })
        .count();
    let remap_money = match selected.effect {
        CrateEffect::Unit => owned_units > 50,
        CrateEffect::Squad => owned_infantry > 100,
        CrateEffect::Cloak => collector.cloak_modifier,
        CrateEffect::Armor => collector.armor_bits != NativeF64Bits::ONE.bits(),
        CrateEffect::Speed => {
            collector.category == EntityCategory::Aircraft
                || collector.speed_bits != NativeF64Bits::ONE.bits()
        }
        CrateEffect::Firepower => {
            !collector.firepower_capable
                || collector.firepower_bits != NativeF64Bits::ONE.bits()
        }
        CrateEffect::Veteran => !collector.trainable || collector.veterancy >= 200,
        _ => false,
    } || (matches!(selected.effect, CrateEffect::Unit | CrateEffect::Squad)
        && is_water_or_beach(sim, cell));

    if remap_money
        || (is_water(sim, cell)
            && !rules.crate_rules.powerups[selected.effect as usize].water_allowed)
    {
        selected.effect = CrateEffect::Money;
        selected.data = rules.crate_rules.powerups[CrateEffect::Money as usize].data;
    }
    selected
}

fn side_base_unit(rules: &RuleSet, side_index: u8) -> Option<&str> {
    rules
        .general
        .base_unit_types
        .iter()
        .find(|type_id| {
            rules.object(type_id).is_some_and(|object| {
                object.owner.iter().any(|owner| {
                    rules.side_index(owner).is_some_and(|side| side.0 == side_index)
                        || rules
                            .country_side_index(owner)
                            .is_some_and(|side| side.0 == side_index)
                })
            })
        })
        .or_else(|| rules.general.base_unit_types.first())
        .map(String::as_str)
}

fn land_type_at(sim: &Simulation, cell: (u16, u16)) -> Option<u8> {
    sim.resolved_terrain
        .as_ref()?
        .cell(cell.0, cell.1)
        .map(|cell| cell.yr_cell_land_type)
}

fn is_water(sim: &Simulation, cell: (u16, u16)) -> bool {
    land_type_at(sim, cell) == Some(LandType::Water.as_index())
}

fn is_water_or_beach(sim: &Simulation, cell: (u16, u16)) -> bool {
    matches!(
        land_type_at(sim, cell),
        Some(value) if value == LandType::Water.as_index() || value == LandType::Beach.as_index()
    )
}
