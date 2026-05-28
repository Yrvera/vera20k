//! Shared tiberium cell mutation logic.
//!
//! Owns the Rust equivalent of gamemd's `CellClass::Reduce_Tiberium` boundary.
//! Callers such as miners, crater smudges, and combat ore damage must use this
//! module instead of mutating `ResourceNode` or `OverlayGrid` independently.

use std::collections::BTreeMap;

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::tiberium_type::{TiberiumTypeId, TiberiumTypeRegistry};
use crate::sim::miner::{ResourceNode, ResourceType};
use crate::sim::ore_growth::OreGrowthState;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::rng::SimRng;

const ORE_STOCK_PER_DENSITY: u16 = 120;
const GEM_STOCK_PER_DENSITY: u16 = 180;

/// Mutable state needed to apply a shared tiberium reduction.
pub struct ReduceTiberiumContext<'a> {
    pub resource_nodes: &'a mut BTreeMap<(u16, u16), ResourceNode>,
    pub overlay_grid: Option<&'a mut OverlayGrid>,
    pub ore_growth_state: &'a mut OreGrowthState,
    pub overlay_registry: Option<&'a OverlayTypeRegistry>,
    pub tiberium_types: Option<&'a TiberiumTypeRegistry>,
    pub resolved_terrain: Option<&'a ResolvedTerrainGrid>,
    pub source_object_cells: Option<&'a std::collections::BTreeSet<(u16, u16)>>,
    pub rng: Option<&'a mut SimRng>,
    pub binary_frame: u32,
    pub spread_enabled: bool,
    pub radar_dirty_cells: Option<&'a mut Vec<(u16, u16)>>,
    pub radar_dirty_generation: Option<&'a mut u64>,
    pub tactical_dirty_cells: Option<&'a mut Vec<(u16, u16)>>,
}

/// Result of one `Reduce_Tiberium` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReduceTiberiumOutcome {
    pub removed_amount: u16,
    pub resource_type: Option<ResourceType>,
    pub fully_removed: bool,
}

impl ReduceTiberiumOutcome {
    fn none() -> Self {
        Self {
            removed_amount: 0,
            resource_type: None,
            fully_removed: false,
        }
    }
}

/// Apply gamemd-shaped tiberium reduction to one cell.
pub fn reduce_tiberium(
    ctx: &mut ReduceTiberiumContext<'_>,
    cell: (u16, u16),
    amount: u16,
) -> ReduceTiberiumOutcome {
    if amount == 0 {
        return ReduceTiberiumOutcome::none();
    }

    let Some(node) = ctx.resource_nodes.get(&cell).copied() else {
        return ReduceTiberiumOutcome::none();
    };

    let base = stock_per_density(node.resource_type);
    let overlay_density = ctx.overlay_grid.as_deref().and_then(|grid| {
        let overlay = grid.cell(cell.0, cell.1);
        overlay.overlay_id.map(|_| overlay.overlay_data as u16)
    });
    let removed_tiberium_type = current_tiberium_type(ctx, cell);
    let current_density = overlay_density.unwrap_or_else(|| node.remaining / base);
    if current_density == 0 {
        return ReduceTiberiumOutcome::none();
    }

    if amount < current_density {
        let remaining_density = current_density - amount;
        if let Some(grid) = ctx.overlay_grid.as_deref_mut() {
            grid.set_overlay_data(cell.0, cell.1, remaining_density.min(11) as u8);
        }
        if let Some(node) = ctx.resource_nodes.get_mut(&cell) {
            node.remaining = remaining_density.saturating_mul(base);
        }
        mark_dirty(ctx, cell);
        return ReduceTiberiumOutcome {
            removed_amount: amount,
            resource_type: Some(node.resource_type),
            fully_removed: false,
        };
    }

    if let Some(grid) = ctx.overlay_grid.as_deref_mut() {
        grid.clear_overlay(cell.0, cell.1);
    }
    ctx.resource_nodes.remove(&cell);
    if let (
        Some(removed_type),
        Some(grid),
        Some(registry),
        Some(types),
        Some(source_object_cells),
        Some(rng),
    ) = (
        removed_tiberium_type,
        ctx.overlay_grid.as_deref(),
        ctx.overlay_registry,
        ctx.tiberium_types,
        ctx.source_object_cells,
        ctx.rng.as_deref_mut(),
    ) {
        ctx.ore_growth_state
            .reseed_native_spread_neighbors_after_reduction(
                removed_type,
                grid,
                registry,
                types,
                ctx.resolved_terrain,
                source_object_cells,
                cell,
                ctx.binary_frame,
                ctx.spread_enabled,
                rng,
            );
    } else {
        ctx.ore_growth_state
            .reseed_spread_neighbors_after_reduction(node.resource_type, cell, ctx.resource_nodes);
    }
    mark_dirty(ctx, cell);

    ReduceTiberiumOutcome {
        removed_amount: current_density,
        resource_type: Some(node.resource_type),
        fully_removed: true,
    }
}

fn current_tiberium_type(
    ctx: &ReduceTiberiumContext<'_>,
    cell: (u16, u16),
) -> Option<TiberiumTypeId> {
    let grid = ctx.overlay_grid.as_deref()?;
    let registry = ctx.overlay_registry?;
    let types = ctx.tiberium_types?;
    let overlay_id = grid.cell(cell.0, cell.1).overlay_id?;
    registry
        .tiberium_overlay_mapping(types, overlay_id)
        .map(|mapping| mapping.tiberium_type)
}

fn stock_per_density(resource_type: ResourceType) -> u16 {
    match resource_type {
        ResourceType::Ore => ORE_STOCK_PER_DENSITY,
        ResourceType::Gem => GEM_STOCK_PER_DENSITY,
    }
}

fn mark_dirty(ctx: &mut ReduceTiberiumContext<'_>, cell: (u16, u16)) {
    if let Some(cells) = ctx.radar_dirty_cells.as_deref_mut()
        && !cells.contains(&cell)
    {
        cells.push(cell);
        if let Some(generation) = ctx.radar_dirty_generation.as_deref_mut() {
            *generation = (*generation).wrapping_add(1);
        }
    }
    if let Some(cells) = ctx.tactical_dirty_cells.as_deref_mut()
        && !cells.contains(&cell)
    {
        cells.push(cell);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    use crate::map::overlay_types::OverlayTypeRegistry;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::tiberium_type::TiberiumTypeRegistry;
    use crate::sim::rng::SimRng;

    fn ore_node(density: u16) -> ResourceNode {
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: density * ORE_STOCK_PER_DENSITY,
        }
    }

    fn gem_node(density: u16) -> ResourceNode {
        ResourceNode {
            resource_type: ResourceType::Gem,
            remaining: density * GEM_STOCK_PER_DENSITY,
        }
    }

    fn native_tiberium_fixture() -> (OverlayTypeRegistry, TiberiumTypeRegistry) {
        let mut ini_text = String::from(
            "\
[Tiberiums]
0=Riparius

[Riparius]
Image=1
Growth=2200
GrowthPercentage=.06
Spread=2200
SpreadPercentage=.06

[OverlayTypes]
",
        );
        for i in 1..=12 {
            ini_text.push_str(&format!("{}=TIB{:02}\n", i - 1, i));
        }
        for i in 1..=12 {
            ini_text.push_str(&format!("[TIB{:02}]\nTiberium=yes\n", i));
        }
        let ini = IniFile::from_str(&ini_text);
        (
            OverlayTypeRegistry::from_ini(&ini, None),
            TiberiumTypeRegistry::from_ini(&ini),
        )
    }

    #[test]
    fn partial_reduction_updates_overlay_node_and_dirty_lists() {
        let mut nodes = BTreeMap::new();
        nodes.insert((5, 5), ore_node(8));
        let mut overlay = OverlayGrid::new(10, 10);
        overlay.place_overlay(5, 5, 1, 8);
        let mut growth = OreGrowthState::new(10, 10);
        let mut radar_dirty = Vec::new();
        let mut radar_generation = 0;
        let mut tactical_dirty = Vec::new();

        let mut ctx = ReduceTiberiumContext {
            resource_nodes: &mut nodes,
            overlay_grid: Some(&mut overlay),
            ore_growth_state: &mut growth,
            overlay_registry: None,
            tiberium_types: None,
            resolved_terrain: None,
            source_object_cells: None,
            rng: None,
            binary_frame: 0,
            spread_enabled: false,
            radar_dirty_cells: Some(&mut radar_dirty),
            radar_dirty_generation: Some(&mut radar_generation),
            tactical_dirty_cells: Some(&mut tactical_dirty),
        };

        let outcome = reduce_tiberium(&mut ctx, (5, 5), 2);

        assert_eq!(outcome.removed_amount, 2);
        assert_eq!(outcome.resource_type, Some(ResourceType::Ore));
        assert!(!outcome.fully_removed);
        assert_eq!(overlay.cell(5, 5).overlay_data, 6);
        assert_eq!(
            nodes.get(&(5, 5)).unwrap().remaining,
            6 * ORE_STOCK_PER_DENSITY
        );
        assert_eq!(radar_dirty, vec![(5, 5)]);
        assert_eq!(radar_generation, 1);
        assert_eq!(tactical_dirty, vec![(5, 5)]);
    }

    #[test]
    fn full_reduction_uses_overlay_density_caps_harvest_and_clears_overlay() {
        let mut nodes = BTreeMap::new();
        nodes.insert((5, 5), ore_node(12));
        let mut overlay = OverlayGrid::new(10, 10);
        overlay.place_overlay(5, 5, 1, 11);
        let mut growth = OreGrowthState::new(10, 10);
        let mut radar_dirty = Vec::new();
        let mut radar_generation = 0;
        let mut tactical_dirty = Vec::new();

        let mut ctx = ReduceTiberiumContext {
            resource_nodes: &mut nodes,
            overlay_grid: Some(&mut overlay),
            ore_growth_state: &mut growth,
            overlay_registry: None,
            tiberium_types: None,
            resolved_terrain: None,
            source_object_cells: None,
            rng: None,
            binary_frame: 0,
            spread_enabled: false,
            radar_dirty_cells: Some(&mut radar_dirty),
            radar_dirty_generation: Some(&mut radar_generation),
            tactical_dirty_cells: Some(&mut tactical_dirty),
        };

        let outcome = reduce_tiberium(&mut ctx, (5, 5), 12);

        assert_eq!(outcome.removed_amount, 11);
        assert_eq!(outcome.resource_type, Some(ResourceType::Ore));
        assert!(outcome.fully_removed);
        assert!(nodes.get(&(5, 5)).is_none());
        assert_eq!(overlay.cell(5, 5).overlay_id, None);
        assert_eq!(radar_dirty, vec![(5, 5)]);
        assert_eq!(radar_generation, 1);
        assert_eq!(tactical_dirty, vec![(5, 5)]);
    }

    #[test]
    fn full_reduction_reseeds_same_type_spread_neighbors() {
        let mut nodes = BTreeMap::new();
        nodes.insert((5, 5), ore_node(3));
        nodes.insert((6, 5), ore_node(4));
        nodes.insert((5, 6), ore_node(4));
        nodes.insert((4, 5), gem_node(4));
        let mut overlay = OverlayGrid::new(10, 10);
        overlay.place_overlay(5, 5, 1, 3);
        let mut growth = OreGrowthState::new(10, 10);
        let mut radar_dirty = Vec::new();
        let mut radar_generation = 0;
        let mut tactical_dirty = Vec::new();

        let mut ctx = ReduceTiberiumContext {
            resource_nodes: &mut nodes,
            overlay_grid: Some(&mut overlay),
            ore_growth_state: &mut growth,
            overlay_registry: None,
            tiberium_types: None,
            resolved_terrain: None,
            source_object_cells: None,
            rng: None,
            binary_frame: 0,
            spread_enabled: false,
            radar_dirty_cells: Some(&mut radar_dirty),
            radar_dirty_generation: Some(&mut radar_generation),
            tactical_dirty_cells: Some(&mut tactical_dirty),
        };

        let outcome = reduce_tiberium(&mut ctx, (5, 5), 3);

        assert!(outcome.fully_removed);
        let queued: Vec<_> = growth
            .spread_queue_entries()
            .iter()
            .map(|entry| (entry.resource_type, entry.rx, entry.ry))
            .collect();
        assert_eq!(
            queued,
            vec![(ResourceType::Ore, 6, 5), (ResourceType::Ore, 5, 6)]
        );
    }

    #[test]
    fn full_reduction_reseeds_native_spread_queue_when_type_context_is_available() {
        let (overlay_registry, tiberium_types) = native_tiberium_fixture();
        let tib01 = overlay_registry.id_for_name("TIB01").expect("TIB01");
        let mut nodes = BTreeMap::new();
        nodes.insert((5, 5), ore_node(3));
        nodes.insert((6, 5), ore_node(4));
        nodes.insert((5, 6), ore_node(4));
        let mut overlay = OverlayGrid::new(10, 10);
        overlay.place_overlay(5, 5, tib01, 3);
        overlay.place_overlay(6, 5, tib01, 4);
        overlay.place_overlay(5, 6, tib01, 4);
        let mut growth = OreGrowthState::new(10, 10);
        growth.reset_native_tiberium_classes(tiberium_types.len(), 100);
        let mut rng = SimRng::new(3);
        growth.add_native_spread_queue_cell(
            &overlay,
            &overlay_registry,
            &tiberium_types,
            None,
            &BTreeSet::new(),
            5,
            5,
            100,
            true,
            &mut rng,
        );
        let before_reduce_rng = rng.state();
        let source_object_cells = BTreeSet::new();

        let mut ctx = ReduceTiberiumContext {
            resource_nodes: &mut nodes,
            overlay_grid: Some(&mut overlay),
            ore_growth_state: &mut growth,
            overlay_registry: Some(&overlay_registry),
            tiberium_types: Some(&tiberium_types),
            resolved_terrain: None,
            source_object_cells: Some(&source_object_cells),
            rng: Some(&mut rng),
            binary_frame: 200,
            spread_enabled: true,
            radar_dirty_cells: None,
            radar_dirty_generation: None,
            tactical_dirty_cells: None,
        };

        let outcome = reduce_tiberium(&mut ctx, (5, 5), 3);

        assert!(outcome.fully_removed);
        assert!(growth.spread_queue_entries().is_empty());
        let class = &growth.native_tiberium_state().classes[0];
        assert!(
            !class.spread_bitmap.contains(&(5, 5)),
            "full removal clears the removed cell's native spread bitmap bit"
        );
        assert!(class.spread_bitmap.contains(&(6, 5)));
        assert!(class.spread_bitmap.contains(&(5, 6)));
        assert_eq!(
            class
                .spread_heap
                .iter()
                .filter(|entry| (entry.rx, entry.ry) == (6, 5) || (entry.rx, entry.ry) == (5, 6))
                .count(),
            2
        );
        assert_ne!(
            rng.state(),
            before_reduce_rng,
            "native AddToSpreadQueue reseed consumes RNG for accepted neighbors"
        );
    }

    #[test]
    fn gem_partial_reduction_uses_gem_density_base_without_overlay() {
        let mut nodes = BTreeMap::new();
        nodes.insert((5, 5), gem_node(4));
        let mut growth = OreGrowthState::new(10, 10);

        let mut ctx = ReduceTiberiumContext {
            resource_nodes: &mut nodes,
            overlay_grid: None,
            ore_growth_state: &mut growth,
            overlay_registry: None,
            tiberium_types: None,
            resolved_terrain: None,
            source_object_cells: None,
            rng: None,
            binary_frame: 0,
            spread_enabled: false,
            radar_dirty_cells: None,
            radar_dirty_generation: None,
            tactical_dirty_cells: None,
        };

        let outcome = reduce_tiberium(&mut ctx, (5, 5), 2);

        assert_eq!(outcome.removed_amount, 2);
        assert_eq!(outcome.resource_type, Some(ResourceType::Gem));
        assert_eq!(
            nodes.get(&(5, 5)).unwrap().remaining,
            2 * GEM_STOCK_PER_DENSITY
        );
    }
}
