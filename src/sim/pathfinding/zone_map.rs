//! Zone-based connectivity map for hierarchical pathfinding.
//!
//! The map is partitioned into zones — connected regions of passable cells —
//! per `MovementZone`. This enables:
//! - **O(1) reachability checks**: two cells are mutually reachable iff they
//!   share the same zone ID (or zones are connected via the adjacency graph).
//! - **Hierarchical search**: Dijkstra on the zone graph finds a corridor of
//!   zones, then A* only explores cells within that corridor.
//!
//! Zones are computed via flood-fill at map load and rebuilt when terrain
//! changes (building placement/destruction, bridge destruction).
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/pathfinding, sim/terrain_cost, sim/locomotor.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::{BTreeMap, VecDeque};

use super::PathGrid;
use super::terrain_cost::TerrainCostGrid;
use super::zone_build;
use super::zone_hierarchy::{SuperZoneMap, ZoneHierarchy};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::{MovementZone, SpeedType};
use crate::sim::movement::locomotor::MovementLayer;

/// Zone ID: 0 = impassable/unassigned, 1+ = valid zone.
pub type ZoneId = u16;

/// Sentinel for impassable or unassigned cells.
pub const ZONE_INVALID: ZoneId = 0;

/// Per-zone metadata: centroid and cell count.
/// Used by the hierarchical zone Dijkstra to estimate inter-zone distances.
#[derive(Debug, Clone, Copy, Default)]
pub struct ZoneInfo {
    pub center: (u16, u16),
    pub cell_count: u32,
}

/// Per-movement-zone cell-to-zone lookup.
#[derive(Debug, Clone)]
pub struct ZoneMap {
    /// Zone ID per cell, indexed by `y * width + x`. ZONE_INVALID = impassable.
    ///
    /// TODO(RE): RA2/YR does not store zone IDs directly per cell. Each cell carries
    /// a nodeIndex, and each MovementZone has its own zoneIdByNodeIndex table.
    zone_ids: Vec<ZoneId>,
    /// Per-cell bridge redirect: for bridge cells, the ground endpoint cell
    /// whose zone ID should be returned for bridge-layer queries.
    /// None = no bridges on map. Mirrors gamemd.exe GetZoneID redirect (0x0056d230).
    bridge_redirect: Option<Vec<Option<(u16, u16)>>>,
    pub width: u16,
    pub height: u16,
    /// Highest assigned zone ID. Native-derived maps reserve label 1, so this
    /// can be greater than the number of publicly passable components.
    pub zone_count: u16,
    /// Per-zone centroid and cell count (index = zone_id - 1). Reserved labels
    /// retain default metadata.
    pub zone_info: Vec<ZoneInfo>,
}

impl ZoneMap {
    /// Construct a ZoneMap from pre-computed arrays.
    pub(crate) fn new(
        zone_ids: Vec<ZoneId>,
        bridge_redirect: Option<Vec<Option<(u16, u16)>>>,
        width: u16,
        height: u16,
        zone_count: u16,
        zone_info: Vec<ZoneInfo>,
    ) -> Self {
        Self {
            zone_ids,
            bridge_redirect,
            width,
            height,
            zone_count,
            zone_info,
        }
    }

    /// Look up the zone ID for a cell at the given layer.
    ///
    /// For bridge-layer queries on a structural cell, returns the ground zone
    /// selected by the matching high-bridge record. Nonstructural cells a high
    /// record reaches keep their own ground zone. Any cell the redirect table
    /// does not cover — and every cell when the map has no high bridge at all —
    /// has no bridge layer and is invalid. Answering such a query with the
    /// ground zone would report every cell as bridge-reachable.
    pub fn zone_at(&self, x: u16, y: u16, layer: MovementLayer) -> ZoneId {
        if x >= self.width || y >= self.height {
            return ZONE_INVALID;
        }
        let idx = y as usize * self.width as usize + x as usize;
        match layer {
            MovementLayer::Bridge => {
                let Some(redirect) = &self.bridge_redirect else {
                    return ZONE_INVALID;
                };
                let Some(Some((ex, ey))) = redirect.get(idx) else {
                    return ZONE_INVALID;
                };
                let e_idx = *ey as usize * self.width as usize + *ex as usize;
                self.zone_ids.get(e_idx).copied().unwrap_or(ZONE_INVALID)
            }
            _ => self.zone_ids[idx],
        }
    }

    /// Get the centroid and cell count for a zone.
    pub fn info_for(&self, zone_id: ZoneId) -> Option<&ZoneInfo> {
        if zone_id == ZONE_INVALID {
            return None;
        }
        self.zone_info.get(zone_id as usize - 1)
    }

    /// Check if two cells are in the same zone (same layer assumed).
    pub fn same_zone(&self, a: (u16, u16), b: (u16, u16), layer: MovementLayer) -> bool {
        let za = self.zone_at(a.0, a.1, layer);
        let zb = self.zone_at(b.0, b.1, layer);
        za != ZONE_INVALID && za == zb
    }

    /// Immutable access to the ground-layer zone ID array.
    pub(crate) fn zone_ids_slice(&self) -> &[ZoneId] {
        &self.zone_ids
    }

    /// Mutable access to the ground-layer zone ID array.
    pub(crate) fn zone_ids_mut(&mut self) -> &mut Vec<ZoneId> {
        &mut self.zone_ids
    }

    pub(crate) fn set_ground_zone_at_index(&mut self, index: usize, zone: ZoneId) {
        if let Some(slot) = self.zone_ids.get_mut(index) {
            *slot = zone;
        }
    }

    /// Replace the bridge redirect table (e.g. after incremental recomputation).
    pub(crate) fn set_bridge_redirect(&mut self, redirect: Option<Vec<Option<(u16, u16)>>>) {
        self.bridge_redirect = redirect;
    }

    /// Update zone_count (e.g. after incremental zone assignment).
    pub(crate) fn set_zone_count(&mut self, n: u16) {
        self.zone_count = n;
    }

    /// Replace zone_info (e.g. after incremental recomputation).
    pub(crate) fn set_zone_info(&mut self, info: Vec<ZoneInfo>) {
        self.zone_info = info;
    }
}

/// Zone adjacency graph — which zones border each other.
#[derive(Debug, Clone)]
pub struct ZoneAdjacency {
    /// For each zone ID (1-indexed), adjacent zone IDs in discovery order.
    pub neighbors: Vec<Vec<ZoneId>>,
}

impl ZoneAdjacency {
    /// Construct from a pre-built neighbor list.
    pub(crate) fn new(neighbors: Vec<Vec<ZoneId>>) -> Self {
        Self { neighbors }
    }

    /// Check if two zones are directly adjacent.
    pub fn are_adjacent(&self, a: ZoneId, b: ZoneId) -> bool {
        if a == ZONE_INVALID || b == ZONE_INVALID {
            return false;
        }
        let idx = a as usize;
        if idx >= self.neighbors.len() {
            return false;
        }
        self.neighbors[idx].contains(&b)
    }

    /// Get the neighbors of a zone.
    pub fn neighbors_of(&self, z: ZoneId) -> &[ZoneId] {
        if z == ZONE_INVALID || z as usize >= self.neighbors.len() {
            return &[];
        }
        &self.neighbors[z as usize]
    }
}

/// Complete zone system: zone maps + adjacency graphs for all movement zones.
#[derive(Debug, Clone)]
pub struct ZoneGrid {
    maps: BTreeMap<MovementZone, ZoneMap>,
    adjacency: BTreeMap<MovementZone, ZoneAdjacency>,
    /// Connected-component labels for O(1) reachability checks.
    super_zones: BTreeMap<MovementZone, SuperZoneMap>,
    /// One optional gamemd-style route-selection hierarchy shared by all rows.
    hierarchy: Option<ZoneHierarchy>,
    /// Cell-owned reduced classes, shared base clusters, and the retained raw
    /// per-row cluster mappings used by exact one-cell repair.
    base_topology: Option<zone_build::BaseZoneTopology>,
    /// Map-load bridge records paired with the hierarchy snapshot. These are
    /// consumed only by hierarchy-coordinate projection.
    bridge_records: Vec<crate::sim::bridge_state::BridgeEndpointRecord>,
    pub width: u16,
    pub height: u16,
}

impl ZoneGrid {
    /// Build zone maps for all non-trivial categories from terrain data.
    pub fn build(
        path_grid: &PathGrid,
        terrain_costs: &BTreeMap<SpeedType, TerrainCostGrid>,
        width: u16,
        height: u16,
    ) -> Self {
        Self::build_with_terrain(path_grid, terrain_costs, None, &[], width, height)
    }

    /// Build zone maps using resolved terrain passability when available.
    /// Bridge endpoint records inject cross-bridge adjacency edges for
    /// ground-capable movement zones.
    pub fn build_with_terrain(
        path_grid: &PathGrid,
        terrain_costs: &BTreeMap<SpeedType, TerrainCostGrid>,
        resolved_terrain: Option<&ResolvedTerrainGrid>,
        bridge_records: &[crate::sim::bridge_state::BridgeEndpointRecord],
        width: u16,
        height: u16,
    ) -> Self {
        let mut maps = BTreeMap::new();
        let mut adjacency = BTreeMap::new();
        let mut super_zones = BTreeMap::new();
        let base_topology = resolved_terrain.map(|terrain| {
            zone_build::build_base_zone_topology(path_grid, terrain, bridge_records, width, height)
        });
        let hierarchy = base_topology.as_ref().map(|base| {
            zone_build::build_zone_hierarchy(
                base,
                path_grid,
                resolved_terrain,
                bridge_records,
                width,
                height,
            )
        });

        for &mz in MovementZone::all_ground() {
            let speed_type = mz.speed_type();
            let cost_grid = terrain_costs.get(&speed_type);

            let (mut zone_map, mut adj) = if let Some(base) = &base_topology {
                zone_build::build_zone_map_from_base_topology(base, mz, width, height)
            } else {
                zone_build::build_zone_map_with_terrain(
                    path_grid, cost_grid, None, mz, width, height,
                )
            };

            if mz.can_use_bridges() {
                if base_topology.is_none() {
                    zone_build::inject_bridge_adjacency(
                        &mut adj,
                        zone_map.zone_ids_slice(),
                        bridge_records,
                        width,
                        zone_build::BridgeRecordFilter::AllActive,
                    );
                }
                zone_map.set_bridge_redirect(zone_build::build_bridge_redirect(
                    path_grid,
                    resolved_terrain,
                    bridge_records,
                    width,
                    height,
                ));
            }

            let sz = SuperZoneMap::from_adjacency(&adj, zone_map.zone_count);
            super_zones.insert(mz, sz);
            maps.insert(mz, zone_map);
            adjacency.insert(mz, adj);
        }

        ZoneGrid {
            maps,
            adjacency,
            super_zones,
            hierarchy,
            base_topology,
            bridge_records: bridge_records.to_vec(),
            width,
            height,
        }
    }

    /// Get the zone map for a movement zone.
    pub fn map_for(&self, mz: MovementZone) -> Option<&ZoneMap> {
        self.maps.get(&mz)
    }

    /// Exact non-bridge `MapClass::GetZoneID` raw-row lookup.
    ///
    /// Native `MapClass::GetZoneID @ 0x0056D230` packs both coordinate
    /// components to signed 16-bit, indexes an `(W + 1) * (W + 1)` square,
    /// clamps only the resulting signed linear index, and projects the base
    /// cluster through the requested raw movement-zone row. The extra final
    /// row and column are zero-initialized base cluster 0; raw labels `1` and
    /// `0xffff` are returned unchanged.
    ///
    /// This seam deliberately refuses compatibility-only or malformed Rust
    /// topology. Native permits an unchecked movement-row access, but Rust has
    /// no sound equivalent for that undefined read, so a non-concrete row or a
    /// missing cluster entry fails explicitly.
    pub(crate) fn get_zone_id_nonbridge_native(
        &self,
        coord: (i32, i32),
        movement_zone: MovementZone,
    ) -> Option<ZoneId> {
        let base = self.base_topology.as_ref()?;
        if self.width != self.height {
            return None;
        }

        let width = usize::from(self.width);
        let cell_count = width.checked_mul(width)?;
        if base.movement_classes.len() != cell_count || base.zone_ids.len() != cell_count {
            return None;
        }

        let row = movement_zone.matrix_row()?;
        let raw_row = base.raw_zone_ids_by_row.get(row)?;
        let side = i32::from(self.width).checked_add(1)?;
        let padded_count = side.checked_mul(side)?;
        let x = i32::from(coord.0 as i16);
        let y = i32::from(coord.1 as i16);
        let linear = side.wrapping_mul(y).wrapping_add(x);
        let clamped = linear.clamp(0, padded_count - 1);
        let padded_x = clamped % side;
        let padded_y = clamped / side;
        let cluster = if padded_x < i32::from(self.width) && padded_y < i32::from(self.height) {
            let index = padded_y as usize * width + padded_x as usize;
            *base.zone_ids.get(index)?
        } else {
            ZONE_INVALID
        };
        raw_row.get(cluster as usize).copied()
    }

    /// Get the adjacency graph for a movement zone.
    pub fn adjacency_for(&self, mz: MovementZone) -> Option<&ZoneAdjacency> {
        self.adjacency.get(&mz)
    }

    /// Get the shared route-selection hierarchy when this movement row exists.
    pub(crate) fn hierarchy_for(&self, mz: MovementZone) -> Option<&ZoneHierarchy> {
        if !self.maps.contains_key(&mz) {
            return None;
        }
        self.hierarchy.as_ref()
    }

    pub(crate) fn bridge_records(&self) -> &[crate::sim::bridge_state::BridgeEndpointRecord] {
        &self.bridge_records
    }

    pub(crate) fn movement_classes_match(&self, terrain: &ResolvedTerrainGrid) -> bool {
        let Some(base) = &self.base_topology else {
            return false;
        };
        base.movement_classes.len() == self.width as usize * self.height as usize
            && (0..self.height).all(|y| {
                (0..self.width).all(|x| {
                    let index = y as usize * self.width as usize + x as usize;
                    base.movement_classes[index]
                        == zone_build::movement_class_for_cell(terrain, x, y)
                })
            })
    }

    /// Project one RecalcAttributes cell into the retained base topology
    /// without assigning a zone or rebuilding hierarchy. Mutation owners that
    /// have a later native repair callback use this to make the new class
    /// visible to earlier ordered neighbor repairs.
    pub(crate) fn refresh_base_movement_class_at(
        &mut self,
        terrain: &ResolvedTerrainGrid,
        x: u16,
        y: u16,
    ) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let index = y as usize * self.width as usize + x as usize;
        let Some(base) = self.base_topology.as_mut() else {
            return false;
        };
        let Some(slot) = base.movement_classes.get_mut(index) else {
            return false;
        };
        *slot = zone_build::movement_class_for_cell(terrain, x, y);
        true
    }

    #[cfg(test)]
    pub(crate) fn base_movement_class_at(&self, x: u16, y: u16) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.base_topology
            .as_ref()?
            .movement_classes
            .get(y as usize * self.width as usize + x as usize)
            .copied()
    }

    pub(crate) fn base_topology_mut(&mut self) -> Option<&mut zone_build::BaseZoneTopology> {
        self.base_topology.as_mut()
    }

    pub(crate) fn base_and_hierarchy_mut(
        &mut self,
    ) -> Option<(&zone_build::BaseZoneTopology, &mut ZoneHierarchy)> {
        Some((self.base_topology.as_ref()?, self.hierarchy.as_mut()?))
    }

    /// Project one adopted base cluster through the retained raw 13-row maps.
    /// No topology, count, adjacency, or unrelated cell is rewritten.
    pub(crate) fn project_adopted_base_cell(&mut self, cell_index: usize) {
        let Some(base) = &self.base_topology else {
            return;
        };
        let Some(&cluster) = base.zone_ids.get(cell_index) else {
            return;
        };
        let (width, height) = (self.width, self.height);
        for &movement_zone in MovementZone::all_ground() {
            let row = movement_zone.matrix_row().expect("concrete movement row");
            let raw = base.raw_zone_ids_by_row[row]
                .get(cluster as usize)
                .copied()
                .unwrap_or(u16::MAX);
            let projected = (raw > 1 && raw != u16::MAX)
                .then_some(raw)
                .unwrap_or(ZONE_INVALID);
            if let Some(map) = self.maps.get_mut(&movement_zone) {
                map.set_ground_zone_at_index(cell_index, projected);
                let zone_info = zone_build::compute_zone_info(
                    map.zone_ids_slice(),
                    width,
                    height,
                    map.zone_count,
                );
                map.set_zone_info(zone_info);
            }
        }
    }

    pub(crate) fn replace_hierarchy(&mut self, hierarchy: ZoneHierarchy) {
        self.hierarchy = Some(hierarchy);
    }

    /// Rebuild the products owned by the base connectivity pass while retaining
    /// the hierarchy's append-only identifiers for the following local patch.
    pub(crate) fn rebuild_base_connectivity_preserving_hierarchy(
        &mut self,
        path_grid: &PathGrid,
        resolved_terrain: &ResolvedTerrainGrid,
        bridge_records: &[crate::sim::bridge_state::BridgeEndpointRecord],
    ) {
        let base_topology = zone_build::build_base_zone_topology(
            path_grid,
            resolved_terrain,
            bridge_records,
            self.width,
            self.height,
        );
        let mut maps = BTreeMap::new();
        let mut adjacency = BTreeMap::new();
        let mut super_zones = BTreeMap::new();

        for &movement_zone in MovementZone::all_ground() {
            let (mut zone_map, graph) = zone_build::build_zone_map_from_base_topology(
                &base_topology,
                movement_zone,
                self.width,
                self.height,
            );
            if movement_zone.can_use_bridges() {
                zone_map.set_bridge_redirect(zone_build::build_bridge_redirect(
                    path_grid,
                    Some(resolved_terrain),
                    bridge_records,
                    self.width,
                    self.height,
                ));
            }
            super_zones.insert(
                movement_zone,
                SuperZoneMap::from_adjacency(&graph, zone_map.zone_count),
            );
            maps.insert(movement_zone, zone_map);
            adjacency.insert(movement_zone, graph);
        }

        self.maps = maps;
        self.adjacency = adjacency;
        self.super_zones = super_zones;
        self.base_topology = Some(base_topology);
        self.bridge_records = bridge_records.to_vec();
    }

    /// Mutable access to the zone map for a movement zone (for incremental updates).
    pub(crate) fn map_mut(&mut self, mz: MovementZone) -> Option<&mut ZoneMap> {
        self.hierarchy = None;
        self.maps.get_mut(&mz)
    }

    /// Mutable access to the adjacency graph for a movement zone (for incremental updates).
    pub(crate) fn adjacency_mut(&mut self, mz: MovementZone) -> Option<&mut ZoneAdjacency> {
        self.hierarchy = None;
        self.adjacency.get_mut(&mz)
    }

    /// Replace the super-zone map for a movement zone (after incremental adjacency update).
    pub(crate) fn set_super_zone(&mut self, mz: MovementZone, sz: SuperZoneMap) {
        self.hierarchy = None;
        self.super_zones.insert(mz, sz);
    }

    /// Replace the one shared route-selection hierarchy (test fixtures only).
    #[allow(dead_code)]
    pub(crate) fn set_hierarchy(&mut self, hierarchy: ZoneHierarchy) {
        self.hierarchy = Some(hierarchy);
    }

    /// O(1) reachability check: can a unit with this movement zone reach `to`
    /// from `from`?
    ///
    /// `MapClass::Can_Reach_Zone` @ `0x0056D100` is a **pure equality compare**
    /// of the two `GetZoneID` results — it consults no adjacency graph and no
    /// connected-components structure.
    ///
    /// **VERA-internal widening, gamemd has no equivalent:** the two arms below
    /// that accept distinct zone IDs when the super-zone labels or the adjacency
    /// graph connect them. Trigger: any pair of distinct zone IDs joined by an
    /// adjacency edge. Player effect: VERA accepts a move order gamemd's
    /// reachability test refuses. Frequency: **zero on the production path
    /// today** — `build_zone_map_from_base_topology` deliberately returns an
    /// empty adjacency, so `are_connected` is false for distinct IDs and this
    /// collapses to the native equality. It becomes live the moment the legacy
    /// incremental path (`zone_incremental`, which repopulates adjacency and
    /// replaces the super-zone map) owns a production rebuild, and then it fires
    /// on every ground move order. Downstream risk: high — it is the difference
    /// between "one zone per reachable region" and "zones plus a graph", and the
    /// two designs cannot both be right.
    ///
    /// Also not modelled, recorded: the native opens with `if (speed_type == -1)
    /// return true`, a sentinel arm no caller can reach through
    /// `AStar_pathfind_search` (it resolves `-1` to `TechnoType+0x5B4` first) but
    /// which a caller passing a raw speed type would. Trigger and therefore
    /// frequency: UNCHECKED — no VERA call site passes a sentinel today. Player
    /// effect if it ever fires: gamemd waves the order through; VERA runs the
    /// zone test. Downstream risk: none, it is the first line of the function.
    ///
    /// And the native's two off-playfield
    /// short-circuits, which return `true` early when the source cell fails
    /// `Is_Cell_In_Playfield(cell, 1)` but lies inside the isometric diamond,
    /// and when the caller's flag is set with the source inside and the
    /// destination outside-but-in-diamond. VERA returns `false` for either,
    /// because an off-playfield cell yields `ZONE_INVALID`. Trigger: an order
    /// whose endpoint is in the map border outside the playfield rect. Player
    /// effect: the unit refuses an order retail accepts. Frequency: map-edge
    /// clicks only. Downstream risk: none; both are head-of-function predicates.
    pub fn can_reach(
        &self,
        mz: MovementZone,
        from: (u16, u16),
        from_layer: MovementLayer,
        to: (u16, u16),
        to_layer: MovementLayer,
    ) -> bool {
        let Some(zone_map) = self.maps.get(&mz) else {
            return true; // No zone data — assume reachable (conservative)
        };
        let za = zone_map.zone_at(from.0, from.1, from_layer);
        let zb = zone_map.zone_at(to.0, to.1, to_layer);
        if za == ZONE_INVALID || zb == ZONE_INVALID {
            return false;
        }
        if za == zb {
            return true;
        }
        // Different zones — O(1) super-zone check (union-find connected components).
        if let Some(sz) = self.super_zones.get(&mz) {
            return sz.are_connected(za, zb);
        }
        // Fallback to BFS if super-zones not available (should not happen).
        let Some(adj) = self.adjacency.get(&mz) else {
            return false;
        };
        zone_graph_connected(adj, za, zb, zone_map.zone_count)
    }
}

/// BFS on the zone adjacency graph to check connectivity.
pub(crate) fn zone_graph_connected(
    adj: &ZoneAdjacency,
    start: ZoneId,
    goal: ZoneId,
    max_zones: u16,
) -> bool {
    if start == goal {
        return true;
    }
    let mut visited = vec![false; max_zones as usize + 1];
    let mut queue = VecDeque::new();
    visited[start as usize] = true;
    queue.push_back(start);

    while let Some(z) = queue.pop_front() {
        for &neighbor in adj.neighbors_of(z) {
            if neighbor == goal {
                return true;
            }
            if !visited[neighbor as usize] {
                visited[neighbor as usize] = true;
                queue.push_back(neighbor);
            }
        }
    }
    false
}

// Tests are declared in zone/mod.rs (zone_map_tests.rs).
