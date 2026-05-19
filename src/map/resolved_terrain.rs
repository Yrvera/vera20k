//! Resolved terrain/topology stage built from raw map cells plus theater/TMP metadata.
//!
//! This module sits between `MapFile` parsing and downstream consumers such as
//! rendering, pathfinding, and building placement. It preserves raw IsoMapPack5
//! data while attaching resolved per-cell metadata such as final LAT-adjusted
//! tile choice, land/slope bytes from TMP, and coarse blocking/buildability flags.

/// Zone classification constants matching gamemd.exe RecalcZoneType output.
/// These index columns of `MOVEMENT_CLASS_PASSABILITY` in zone_build.rs.
pub mod zone_class {
    pub const GROUND: u8 = 0;
    pub const ROAD: u8 = 1;
    pub const WALL: u8 = 2;
    pub const BEACH: u8 = 3;
    pub const WATER: u8 = 4;
    pub const BUILDING: u8 = 5;
    pub const IMPASSABLE: u8 = 6;
    pub const OUTSIDE: u8 = 7;
}

use crate::assets::tmp_file::{TmpFile, TmpTile};
use crate::map::bridge_facts::BridgeCellFacts;
use crate::map::lat;
use crate::map::map_file::{MapCell, MapFile};
use crate::map::overlay::OverlayEntry;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::theater::{self, TheaterData, TileKey};
use crate::map::tube_facts::{TubeFact, TubeId};
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass, TerrainRules};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const YR_CELL_LAND_TUNNEL: u8 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Canonical ramp direction from TS++ TIBSUN_DEFINES.H (slope types 1-4).
/// These are the four basic full-edge ramps where two adjacent corners are raised.
///
/// Names are in **map coordinates** (as defined by TS++). In the isometric view,
/// map-North appears as screen upper-right. The actual tilt angles used for VXL
/// rendering come from the slope_type number (1-16) indexed into a pre-computed
/// matrix table — they don't depend on these labels.
pub enum RampDirection {
    West,
    North,
    East,
    South,
}

/// Bridge direction as expressed by the map overlay class. Do not derive high
/// bridge SHP body frames directly from these labels; rendering follows the
/// runtime bridge state-byte family (`Axis::NS => 0..=8`, `Axis::EW => 9..=17`).
/// Low bridges have no height offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeDirection {
    /// BRIDGE1, BRIDGEB1 — EW direction. Height offset = CellHeight + 1 = 16px.
    EastWest,
    /// BRIDGE2, BRIDGEB2 — NS direction. Height offset = CellHeight * 2 + 1 = 31px.
    NorthSouth,
    /// LOBRDG*, LOBRDB* — ground-level bridge. No height offset.
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeLayer {
    pub overlay_id: u8,
    pub overlay_name: String,
    /// Bridge deck height level (ground level + offset).
    pub deck_level: u8,
    /// Bridge direction — determines height offset and rendering.
    pub direction: BridgeDirection,
}

#[derive(Debug, Clone)]
pub struct ResolvedTerrainCell {
    pub rx: u16,
    pub ry: u16,
    pub source_tile_index: i32,
    pub source_sub_tile: u8,
    pub final_tile_index: i32,
    pub final_sub_tile: u8,
    /// True when `final_tile_index` falls in the first 16 tiles of the
    /// theater's `WoodBridgeSet`. This is the CellClass+0x38 predicate used
    /// by Engineer CABHUT bridge-repair dispatch.
    pub is_wood_bridge_repair_tile: bool,
    pub level: u8,
    pub filled_clear: bool,
    pub tileset_index: Option<u16>,
    pub land_type: u8,
    /// Final CellClass LandType value for binary predicates that need the
    /// original gamemd.exe value. Do not confuse this with `land_type`, which
    /// is the compressed passability-matrix column.
    pub yr_cell_land_type: u8,
    pub slope_type: u8,
    pub template_height: u8,
    pub render_offset_x: i32,
    pub render_offset_y: i32,
    pub terrain_class: TerrainClass,
    pub speed_costs: SpeedCostProfile,
    pub is_water: bool,
    pub is_cliff_like: bool,
    pub is_rough: bool,
    pub is_road: bool,
    /// True when this cell's tileset has `Morphable=yes`. Smudge placement
    /// requires this gate (matches gamemd IsoTileTypeClass+0x2E0).
    pub accepts_smudge: bool,
    /// FinalAlert2-style cliff redraw flag. When true, this cell's terrain tile
    /// is drawn a second time AFTER entities so cliff faces occlude units behind
    /// them. Computed from height differences with back-left neighbor cells
    /// (height diff >= 4). See MapData.cpp:3362-3377 in the EA FA2 source.
    pub is_cliff_redraw: bool,
    /// Tile visual variant index (FA2 bRNDImage): 0 = main tile, 1-4 = replacement a-d.
    pub variant: u8,
    pub has_ramp: bool,
    pub canonical_ramp: Option<RampDirection>,
    pub ground_walk_blocked: bool,
    pub terrain_object_blocks: bool,
    pub overlay_blocks: bool,
    /// Cached zone classification (0-7) matching gamemd.exe RecalcZoneType (0x483C80).
    /// Indexes columns of `MOVEMENT_CLASS_PASSABILITY` in zone_build.rs.
    ///
    /// 0=Ground, 1=Road(crate), 2=Wall, 3=Beach, 4=Water,
    /// 5=Building/TerrainObject, 6=Impassable, 7=Outside.
    ///
    /// Does NOT include building footprints (those are entity-based, checked via
    /// PathGrid at zone-build time). Updated by `recalc_overlay_passability` on
    /// overlay mutation.
    pub zone_type: u8,
    /// Terrain-only walk block flag — true when the base terrain (rock, cliff) is
    /// impassable, EXCLUDING overlay and terrain-object contributions.
    /// Needed by `recalc_overlay_passability` to re-derive zone_type after overlay
    /// removal without the conflated `ground_walk_blocked` field.
    pub base_ground_walk_blocked: bool,
    pub base_build_blocked: bool,
    pub build_blocked: bool,
    pub has_bridge_deck: bool,
    pub bridge_walkable: bool,
    pub bridge_transition: bool,
    pub bridge_deck_level: u8,
    pub bridge_layer: Option<BridgeLayer>,
    pub bridge_facts: BridgeCellFacts,
    /// CellClass+0x116 equivalent: index into `ResolvedTerrainGrid::tube_facts`.
    pub tube_index: Option<TubeId>,
    /// Per-tile radar minimap color (left half of isometric diamond), from TMP header.
    pub radar_left: [u8; 3],
    /// Per-tile radar minimap color (right half of isometric diamond), from TMP header.
    pub radar_right: [u8; 3],
    /// True if this cell's underlying TMP sub-tile carries a baked damaged-variant
    /// pixel set. Drives the kickoff gate of the bridge damage flood-fill (only
    /// cells with baked damage art may initiate propagation) and the render-side
    /// substitution that swaps in variant=1 when the bridge sim flags the cell.
    pub has_damaged_data: bool,
    /// Author-damaged anchor pre-classification: `Some(class)` if this
    /// cell's `final_tile_index` matches one of the 8 bridgehead anchor
    /// variant tile_ids in the current theater's BridgeAnchorVariantTable.
    /// `None` when not a variant tile (the common case for both
    /// non-bridge cells and pristine anchor cells).
    ///
    /// Sim's `BridgeRuntimeState::from_resolved_terrain` reads this to
    /// initialize `BridgeRuntimeCell.bridgehead_anchor_class` instead of
    /// the unconditional Variant0 default. None defaults to Variant0
    /// sim-side.
    pub bridgehead_anchor_class_at_load: Option<crate::sim::bridge_state::BridgeheadAnchorClass>,
}

impl ResolvedTerrainCell {
    pub fn is_walkable(&self) -> bool {
        !self.ground_walk_blocked
    }

    pub fn is_bridge_transition_cell(&self) -> bool {
        self.bridge_transition
    }

    pub fn is_elevated_bridge_cell(&self) -> bool {
        self.bridge_walkable && self.bridge_deck_level > self.level
    }

    pub fn bridge_deck_level_if_any(&self) -> Option<u8> {
        self.has_bridge_deck.then_some(self.bridge_deck_level)
    }

    pub fn bridge_flags(&self) -> u32 {
        self.bridge_facts.raw_flags
    }

    pub fn is_low_bridge_tube_cell(&self) -> bool {
        self.tube_index.is_some() && self.yr_cell_land_type == YR_CELL_LAND_TUNNEL
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedTerrainGrid {
    width: u16,
    height: u16,
    pub cells: Vec<ResolvedTerrainCell>,
    tube_facts: Vec<TubeFact>,
}

impl ResolvedTerrainGrid {
    pub fn from_cells(width: u16, height: u16, cells: Vec<ResolvedTerrainCell>) -> Self {
        Self::from_cells_with_tubes(width, height, cells, Vec::new())
    }

    pub fn from_cells_with_tubes(
        width: u16,
        height: u16,
        cells: Vec<ResolvedTerrainCell>,
        tube_facts: Vec<TubeFact>,
    ) -> Self {
        Self {
            width,
            height,
            cells,
            tube_facts,
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn index(&self, rx: u16, ry: u16) -> Option<usize> {
        if rx < self.width && ry < self.height {
            Some(ry as usize * self.width as usize + rx as usize)
        } else {
            None
        }
    }

    pub fn cell(&self, rx: u16, ry: u16) -> Option<&ResolvedTerrainCell> {
        self.index(rx, ry).and_then(|i| self.cells.get(i))
    }

    /// Mutable access to a cell by map coordinates.
    pub fn cell_mut(&mut self, rx: u16, ry: u16) -> Option<&mut ResolvedTerrainCell> {
        let idx = self.index(rx, ry)?;
        self.cells.get_mut(idx)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ResolvedTerrainCell> {
        self.cells.iter()
    }

    pub fn tube_facts(&self) -> &[TubeFact] {
        &self.tube_facts
    }

    pub fn tube(&self, tube_id: TubeId) -> Option<&TubeFact> {
        self.tube_facts.get(tube_id.as_usize())
    }

    pub fn tube_at_cell(&self, rx: u16, ry: u16) -> Option<&TubeFact> {
        let tube_id = self.cell(rx, ry)?.tube_index?;
        self.tube(tube_id)
    }

    pub fn step_coord_by_direction(&self, coord: (u16, u16), direction: u8) -> Option<(u16, u16)> {
        if direction == 8 {
            return Some(
                self.tube_at_cell(coord.0, coord.1)
                    .map_or((0, 0), |tube| tube.exit),
            );
        }
        let (dx, dy) = direction_offset(direction)?;
        let nx = coord.0 as i32 + dx;
        let ny = coord.1 as i32 + dy;
        if nx < 0 || ny < 0 || nx >= self.width as i32 || ny >= self.height as i32 {
            return None;
        }
        Some((nx as u16, ny as u16))
    }

    pub fn walk_directions_from(&self, start: (u16, u16), directions: &[u8]) -> Option<(u16, u16)> {
        let mut coord = start;
        for &direction in directions {
            coord = self.step_coord_by_direction(coord, direction)?;
        }
        Some(coord)
    }

    pub fn build(
        map: &MapFile,
        theater_data: Option<&TheaterData>,
        asset_manager: Option<&crate::assets::asset_manager::AssetManager>,
        terrain_rules: Option<&TerrainRules>,
        overlay_registry: Option<&OverlayTypeRegistry>,
        lat_enabled: bool,
        cliff_back_impassability: u8,
    ) -> Self {
        let (width, height) = grid_dimensions(&map.cells);
        if width == 0 || height == 0 {
            return Self {
                width: 0,
                height: 0,
                cells: Vec::new(),
                tube_facts: Vec::new(),
            };
        }

        let mut final_cells: Vec<MapCell> = map.cells.clone();
        if lat_enabled {
            if let Some(td) = theater_data {
                let lat_config = lat::parse_lat_config(&td.ini_data, &td.lookup);
                if !lat_config.grounds.is_empty() {
                    lat::apply_lat(&mut final_cells, &lat_config, &td.lookup);
                }
            }
        }

        let raw_lookup: HashMap<(u16, u16), &MapCell> =
            map.cells.iter().map(|c| ((c.rx, c.ry), c)).collect();
        let final_lookup: HashMap<(u16, u16), &MapCell> =
            final_cells.iter().map(|c| ((c.rx, c.ry), c)).collect();

        let terrain_objects: HashSet<(u16, u16)> = map
            .terrain_objects
            .iter()
            .map(|obj| (obj.rx, obj.ry))
            .collect();

        let mut overlays_by_cell: HashMap<(u16, u16), Vec<&OverlayEntry>> = HashMap::new();
        for overlay in &map.overlays {
            overlays_by_cell
                .entry((overlay.rx, overlay.ry))
                .or_default()
                .push(overlay);
        }

        let mut metadata_cache: HashMap<TileKey, TileMetadata> = HashMap::new();
        let mut warned_unknown_land_types: HashSet<u8> = HashSet::new();
        let mut cells: Vec<ResolvedTerrainCell> =
            Vec::with_capacity(width as usize * height as usize);

        for ry in 0..height {
            for rx in 0..width {
                let raw = raw_lookup.get(&(rx, ry)).copied();
                let final_cell = final_lookup.get(&(rx, ry)).copied();
                let (final_tile_index, final_sub_tile, level) = final_cell
                    .map(|cell| (cell.tile_index, cell.sub_tile, cell.z))
                    .unwrap_or((0, 0, 0));
                let is_wood_bridge_repair_tile =
                    is_wood_bridge_repair_tile(theater_data, final_tile_index);
                let tile_key = TileKey {
                    tile_id: normalize_tile_id(final_tile_index),
                    sub_tile: final_sub_tile,
                    variant: 0,
                };
                let mut metadata = if let Some(metadata) = metadata_cache.get(&tile_key) {
                    metadata.clone()
                } else {
                    let metadata = load_tile_metadata(
                        theater_data,
                        asset_manager,
                        terrain_rules,
                        tile_key,
                        &mut warned_unknown_land_types,
                    );
                    metadata_cache.insert(tile_key, metadata.clone());
                    metadata
                };
                let terrain_object_blocks = terrain_objects.contains(&(rx, ry));
                let overlay_effects = classify_overlay_effects(
                    overlays_by_cell.get(&(rx, ry)),
                    overlay_registry,
                    level,
                );
                let canonical_ramp = canonical_ramp_from_slope_type(metadata.slope_type);
                // Low bridge overlays are visual/damage facts. Movement is
                // driven by the final YR cell land type plus TubeClass facts,
                // not by forcing the surface terrain into ordinary road.
                // Road/pavement overlays override underlying terrain to Road.
                // Matches original engine: RecalcLandType sets LandType=Road(1)
                // when overlay.Wall is true.
                if overlay_effects.is_road && !overlay_effects.is_low_bridge {
                    metadata.is_road = true;
                    metadata.terrain_class = TerrainClass::Road;
                    metadata.land_type =
                        crate::sim::pathfinding::passability::LandType::Road.as_index();
                    metadata.yr_cell_land_type = metadata.land_type;
                }
                // Tiberium/ore overlays change the effective terrain type for passability.
                // Matches original engine: RecalcLandType sets cell+0xEC when tiberium present.
                // Also update speed_costs from [Tiberium] INI section so the terrain
                // cost grid uses correct speed modifiers (Foot=90%, Track=70%, etc.).
                if overlay_effects.has_tiberium {
                    metadata.land_type =
                        crate::sim::pathfinding::passability::LandType::Tiberium.as_index();
                    metadata.yr_cell_land_type = metadata.land_type;
                    metadata.terrain_class = TerrainClass::Tiberium;
                    if let Some(tib_semantics) =
                        terrain_rules.and_then(|tr| tr.semantics_by_name("Tiberium"))
                    {
                        metadata.speed_costs = tib_semantics.speed_costs;
                    }
                }
                let base_ground_walk_blocked = canonical_ramp.is_none() && metadata.ground_blocked;
                let is_cliff_like = metadata.is_cliff_like;
                // Compute zone_type matching RecalcZoneType (0x483C80) priority chain.
                // Must be computed BEFORE ground_walk_blocked is OR'd with overlay/terrain
                // object flags, since we need the base terrain passability.
                let zone_type = if overlay_effects.is_crate {
                    zone_class::ROAD
                } else if overlay_effects.is_wall {
                    zone_class::WALL
                } else if overlay_effects.has_tiberium {
                    zone_class::IMPASSABLE
                } else if overlay_effects.is_gate {
                    zone_class::IMPASSABLE
                } else if metadata.is_water {
                    zone_class::WATER
                } else if metadata.land_type
                    == crate::sim::pathfinding::passability::LandType::Beach.as_index()
                {
                    zone_class::BEACH
                } else if base_ground_walk_blocked {
                    zone_class::IMPASSABLE
                } else if terrain_object_blocks {
                    zone_class::BUILDING
                } else {
                    zone_class::GROUND
                };
                let ground_walk_blocked = base_ground_walk_blocked
                    || terrain_object_blocks
                    || overlay_effects.overlay_blocks;
                let base_build_blocked = metadata.build_blocked
                    || terrain_object_blocks
                    || overlay_effects.overlay_blocks
                    || canonical_ramp.is_some();
                let bridge_walkable = overlay_effects.has_bridge_deck
                    && !overlay_effects.is_low_bridge
                    && !terrain_object_blocks
                    && !overlay_effects.overlay_blocks;
                // Smudges (craters, scorches) only place on tiles whose tileset has
                // Morphable=yes. Cells with no resolved tile (filled_clear) default
                // to false. Computed once at resolve time so the smudge dispatcher
                // reads a single bool.
                let accepts_smudge = if raw.is_none() {
                    false
                } else {
                    theater_data
                        .map(|td| td.lookup.is_morphable(tile_key.tile_id))
                        .unwrap_or(false)
                };
                // Allow layer transitions on any bridge deck cell. High bridges over
                // water have ground_walk_blocked=true, but units still need to transition
                // from Ground→Bridge at the ramp/entry cells.
                // Only bridgehead ramp cells (detected below) allow layer
                // transitions. Deck cells must NOT be transitions — otherwise
                // the A* can switch Bridge→Ground mid-span and units clip
                // through the bridge.
                let bridge_transition = false;
                let build_blocked = base_build_blocked || overlay_effects.has_bridge_deck;
                cells.push(ResolvedTerrainCell {
                    rx,
                    ry,
                    source_tile_index: raw.map(|c| c.tile_index).unwrap_or(theater::NO_TILE),
                    source_sub_tile: raw.map(|c| c.sub_tile).unwrap_or(0),
                    final_tile_index,
                    final_sub_tile,
                    is_wood_bridge_repair_tile,
                    level,
                    filled_clear: raw.is_none(),
                    tileset_index: metadata.tileset_index,
                    land_type: metadata.land_type,
                    yr_cell_land_type: metadata.yr_cell_land_type,
                    slope_type: metadata.slope_type,
                    template_height: metadata.template_height,
                    render_offset_x: metadata.render_offset_x,
                    render_offset_y: metadata.render_offset_y,
                    terrain_class: metadata.terrain_class,
                    speed_costs: metadata.speed_costs,
                    is_water: metadata.is_water,
                    is_cliff_like,
                    is_rough: metadata.is_rough,
                    is_road: metadata.is_road,
                    accepts_smudge,
                    is_cliff_redraw: false,
                    variant: 0,
                    has_ramp: metadata.has_ramp,
                    canonical_ramp,
                    ground_walk_blocked,
                    terrain_object_blocks,
                    overlay_blocks: overlay_effects.overlay_blocks,
                    zone_type,
                    base_ground_walk_blocked,
                    base_build_blocked,
                    build_blocked,
                    has_bridge_deck: overlay_effects.has_bridge_deck,
                    bridge_walkable,
                    bridge_transition,
                    bridge_deck_level: overlay_effects
                        .bridge_layer
                        .as_ref()
                        .map(|layer| layer.deck_level)
                        .unwrap_or(level),
                    bridge_layer: overlay_effects.bridge_layer,
                    bridge_facts: BridgeCellFacts::default(),
                    tube_index: None,
                    radar_left: metadata.radar_left,
                    radar_right: metadata.radar_right,
                    has_damaged_data: metadata.has_damaged_data,
                    bridgehead_anchor_class_at_load: None,
                });
            }
        }

        let mut bridge_facts = vec![BridgeCellFacts::default(); cells.len()];
        for overlay in &map.overlays {
            if overlay.rx < width && overlay.ry < height {
                let idx = overlay.ry as usize * width as usize + overlay.rx as usize;
                bridge_facts[idx].overlay_id = Some(overlay.overlay_id);
            }
            if let Some((family, direction)) =
                crate::map::bridge_facts::high_bridge_stamp_for_overlay(overlay.overlay_id)
            {
                crate::map::bridge_facts::stamp_set_bridge_direction(
                    &mut bridge_facts,
                    width,
                    height,
                    (overlay.rx, overlay.ry),
                    family,
                    direction,
                    true,
                );
            }
        }

        if map.has_overlay_data_pack() {
            for (idx, cell) in cells.iter().enumerate() {
                bridge_facts[idx].state_byte = map.overlay_data_at(cell.rx, cell.ry);
            }
        }

        for (cell, facts) in cells.iter_mut().zip(bridge_facts) {
            cell.bridge_facts = facts;

            if facts.has_structural_bridge() {
                cell.has_bridge_deck = true;
                cell.bridge_walkable = !cell.terrain_object_blocks && !cell.overlay_blocks;
                cell.bridge_deck_level = cell.level.saturating_add(4);
                cell.build_blocked = cell.base_build_blocked || cell.bridge_walkable;
            } else if facts.family != crate::map::bridge_facts::BridgeStampFamily::None
                && cell
                    .bridge_layer
                    .as_ref()
                    .is_some_and(|bl| bl.direction != BridgeDirection::Low)
            {
                cell.has_bridge_deck = false;
                cell.bridge_walkable = false;
                cell.bridge_deck_level = cell.level;
                cell.build_blocked = cell.base_build_blocked;
            }

            if facts.has_transition_flag() {
                cell.bridge_transition = true;
            }
        }

        if let Some(td) = theater_data {
            if let (Some(bs_idx), Some(ramp_table)) = (
                td.bridge_set,
                crate::map::theater::BridgeRampTileTable::from_theater(td),
            ) {
                if let Some(bridge_set_bounds) = td.lookup.bounds().get(bs_idx as usize) {
                    let bridge_set_start = bridge_set_bounds.start;
                    let mut ramp_count = 0usize;
                    for cell in &mut cells {
                        if cell.final_tile_index < 0 {
                            continue;
                        }
                        let tile_id = normalize_tile_id(cell.final_tile_index);
                        let Some(ramp_tile) = ramp_table.match_tile_id(
                            tile_id,
                            bridge_set_start,
                            bridge_set_bounds.count,
                            cell.template_height,
                        ) else {
                            continue;
                        };
                        cell.bridge_facts.ramp_tile = Some(ramp_tile);
                        ramp_count += 1;
                    }
                    if ramp_count > 0 {
                        log::info!(
                            "ResolvedTerrain: {} exact high bridge ramp cells detected",
                            ramp_count,
                        );
                    }
                }
            }
        }

        {
            let mut high_deck: Vec<(u16, u16, u8, u32)> = cells
                .iter()
                .filter(|c| c.bridge_facts.has_structural_bridge())
                .map(|c| (c.rx, c.ry, c.bridge_deck_level, c.bridge_facts.raw_flags))
                .collect();
            high_deck.sort_by_key(|(rx, ry, _, _)| (*rx, *ry));
            if !high_deck.is_empty() {
                log::debug!(
                    "High bridge stamped structural cells ({} total):",
                    high_deck.len(),
                );
                for (rx, ry, dl, flags) in &high_deck {
                    log::debug!("  ({}, {}) deck_level={} flags=0x{:X}", rx, ry, dl, flags);
                }
            }
        }

        // Log bridge cell statistics for diagnostics.
        let bridge_cell_count: usize = cells.iter().filter(|c| c.has_bridge_deck).count();
        let low_bridge_count: usize = cells
            .iter()
            .filter(|c| {
                c.bridge_layer
                    .as_ref()
                    .map(|bl| bl.direction == BridgeDirection::Low)
                    .unwrap_or(false)
            })
            .count();
        let high_bridge_count: usize = bridge_cell_count - low_bridge_count;
        if bridge_cell_count > 0 {
            log::info!(
                "ResolvedTerrain: {} bridge deck cells ({} high, {} low)",
                bridge_cell_count,
                high_bridge_count,
                low_bridge_count,
            );
        }

        // FinalAlert2-style cliff redraw detection (MapData.cpp:3362-3377).
        // For each cell, check the 2x2 block of neighbors at offsets (-2..-1, -2..-1)
        // in isometric (rx, ry) space. If any neighbor is >= 4 levels lower than this
        // cell, mark it for second-pass terrain redraw so cliff faces occlude entities.
        const CLIFF_HEIGHT_THRESHOLD: u8 = 4;
        let mut cliff_redraw_count: usize = 0;
        for idx in 0..cells.len() {
            let rx = cells[idx].rx as i32;
            let ry = cells[idx].ry as i32;
            let h = cells[idx].level;
            if h < CLIFF_HEIGHT_THRESHOLD {
                continue;
            }
            let mut redraw = false;
            'outer: for dy in [-2i32, -1] {
                for dx in [-2i32, -1] {
                    let nx = rx + dx;
                    let ny = ry + dy;
                    if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                        let nidx = ny as usize * width as usize + nx as usize;
                        if nidx < cells.len()
                            && h.saturating_sub(cells[nidx].level) >= CLIFF_HEIGHT_THRESHOLD
                        {
                            redraw = true;
                            break 'outer;
                        }
                    }
                }
            }
            if redraw {
                cells[idx].is_cliff_redraw = true;
                cliff_redraw_count += 1;
            }
        }
        if cliff_redraw_count > 0 {
            log::info!(
                "ResolvedTerrain: {} cells flagged for cliff redraw",
                cliff_redraw_count,
            );
        }

        // CliffBackImpassability: mark cells at the base of ≥4-level cliffs as
        // impassable. Matches gamemd.exe CellClass::RecalcAttributes (0x0047d2b0).
        // When value == 2 (YR default), cells where ANY of 6 isometric neighbors
        // is ≥4 levels above get land_type=Rock and ground_walk_blocked=true.
        // Only overrides Clear(0), Water(4), Beach(3) land types.
        if cliff_back_impassability == 2 {
            const CLIFF_BACK_HEIGHT_DIFF: u8 = 4;
            // 6 neighbor offsets in (dx, dy) matching gamemd.exe RecalcAttributes:
            // (X, Y-1), (X-1, Y), (X+2, Y+2), (X+1, Y+1), (X-1, Y+1), (X+1, Y-1)
            const NEIGHBOR_OFFSETS: [(i32, i32); 6] =
                [(0, -1), (-1, 0), (2, 2), (1, 1), (-1, 1), (1, -1)];
            let rock_lt = crate::sim::pathfinding::passability::LandType::Rock.as_index();
            let clear_lt = crate::sim::pathfinding::passability::LandType::Clear.as_index();
            let water_lt = crate::sim::pathfinding::passability::LandType::Water.as_index();
            let beach_lt = crate::sim::pathfinding::passability::LandType::Beach.as_index();

            let mut cliff_back_count: usize = 0;
            for idx in 0..cells.len() {
                let lt = cells[idx].land_type;
                if lt != clear_lt && lt != water_lt && lt != beach_lt {
                    continue;
                }
                let cell_level = cells[idx].level;
                let rx = cells[idx].rx as i32;
                let ry = cells[idx].ry as i32;

                let mut behind_cliff = false;
                for &(dx, dy) in &NEIGHBOR_OFFSETS {
                    let nx = rx + dx;
                    let ny = ry + dy;
                    if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                        let nidx = ny as usize * width as usize + nx as usize;
                        if nidx < cells.len()
                            && cells[nidx].level >= cell_level + CLIFF_BACK_HEIGHT_DIFF
                        {
                            behind_cliff = true;
                            break;
                        }
                    }
                }
                if behind_cliff {
                    cells[idx].land_type = rock_lt;
                    cells[idx].ground_walk_blocked = true;
                    cells[idx].is_cliff_like = true;
                    cliff_back_count += 1;
                }
            }
            if cliff_back_count > 0 {
                log::info!(
                    "ResolvedTerrain: {} cells marked impassable by CliffBackImpassability",
                    cliff_back_count,
                );
            }
        }

        // Assign random tile visual variants (FA2 bRNDImage, MapData.cpp:3292-3306).
        // Uses deterministic hash of (rx, ry) for reproducibility across sessions.
        // Tiles with HasDamagedData (bridges) use variants for damage states, not
        // visual diversity — those are excluded.
        if let Some(td) = theater_data {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut variant_total: usize = 0;
            for cell in &mut cells {
                let tile_id = normalize_tile_id(cell.final_tile_index);
                let vc = td.lookup.variant_count(tile_id);
                if vc == 0 {
                    continue;
                }
                // Bridges with baked damaged variants reserve cell.variant for
                // the per-frame damaged_variant pick (sim-driven), so the
                // map-load PRNG must leave it at 0.
                if cell.has_damaged_data {
                    continue;
                }
                let mut hasher = DefaultHasher::new();
                (cell.rx, cell.ry).hash(&mut hasher);
                let hash = hasher.finish();
                // 0 = main tile, 1..=vc = replacement. Matches FA2's
                // rand() * (1 + count) / RAND_MAX distribution.
                cell.variant = (hash % (vc as u64 + 1)) as u8;
                if cell.variant > 0 {
                    variant_total += 1;
                }
            }
            if variant_total > 0 {
                log::info!(
                    "ResolvedTerrain: {} cells assigned tile variants",
                    variant_total,
                );
            }
        }

        // Pre-classify author-damaged anchor placements: cells whose
        // tileset is BridgeSet AND whose final_tile_index matches one of
        // the 4 NS or 4 EW variant tile_ids get a non-None
        // bridgehead_anchor_class_at_load. Sim's bridge-state init reads
        // this so maps that author pre-damaged anchors render correctly
        // from frame 1.
        if let Some(td) = theater_data {
            if let Some(table) = crate::map::theater::BridgeAnchorVariantTable::from_theater(td) {
                if let Some(bs_idx) = td.bridge_set {
                    for cell in cells.iter_mut() {
                        if cell.tileset_index != Some(bs_idx) {
                            continue;
                        }
                        if cell.final_tile_index < 0 {
                            continue;
                        }
                        let tid = if cell.final_tile_index == 0xFFFF {
                            0
                        } else {
                            cell.final_tile_index as u16
                        };
                        if let Some((_axis, class)) = table.match_tile_id(tid) {
                            cell.bridgehead_anchor_class_at_load = Some(class);
                        }
                    }
                }
            }
        }

        let mut tube_facts =
            seed_explicit_map_tubes(&mut cells, width, height, &map.explicit_tubes);
        build_auto_low_bridge_tubes(&mut cells, width, height, theater_data, &mut tube_facts);

        Self {
            width,
            height,
            cells,
            tube_facts,
        }
    }

    pub fn build_height_map(&self) -> BTreeMap<(u16, u16), u8> {
        self.cells
            .iter()
            .map(|cell| ((cell.rx, cell.ry), cell.level))
            .collect()
    }

    /// Build a bridge deck height map — only HIGH bridge cells are included.
    /// Low bridges (LOBRDG/LOBRDB) are at ground level and don't need height
    /// correction for click resolution or debug overlays.
    pub fn build_bridge_height_map(&self) -> BTreeMap<(u16, u16), u8> {
        self.cells
            .iter()
            .filter(|cell| {
                cell.has_bridge_deck
                    && !cell
                        .bridge_layer
                        .as_ref()
                        .is_some_and(|bl| bl.direction == BridgeDirection::Low)
            })
            .map(|cell| ((cell.rx, cell.ry), cell.bridge_deck_level))
            .collect()
    }
}

#[derive(Debug, Clone)]
struct TileMetadata {
    tileset_index: Option<u16>,
    has_tmp_metadata: bool,
    /// Mapped land type (0-7) for passability matrix lookups.
    land_type: u8,
    /// Final gamemd CellClass LandType value where Rust needs the binary
    /// predicate. This is not the TMP terrain_type byte.
    yr_cell_land_type: u8,
    /// Raw TMP terrain_type byte (0-15) for rules.ini semantic lookups.
    raw_land_type: u8,
    slope_type: u8,
    template_height: u8,
    render_offset_x: i32,
    render_offset_y: i32,
    terrain_class: TerrainClass,
    speed_costs: SpeedCostProfile,
    is_water: bool,
    is_cliff_like: bool,
    is_rough: bool,
    is_road: bool,
    has_ramp: bool,
    ground_blocked: bool,
    build_blocked: bool,
    /// Per-tile radar minimap color (left half of isometric diamond), from TMP header.
    radar_left: [u8; 3],
    /// Per-tile radar minimap color (right half of isometric diamond), from TMP header.
    radar_right: [u8; 3],
    /// Mirrors `TmpTile.has_damaged_data` — set when the TMP sub-tile flag DWORD
    /// declares a baked damaged-variant pixel set.
    has_damaged_data: bool,
}

impl Default for TileMetadata {
    fn default() -> Self {
        Self {
            tileset_index: None,
            has_tmp_metadata: false,
            land_type: 0,
            yr_cell_land_type: 0,
            raw_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Unknown,
            speed_costs: SpeedCostProfile::default(),
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            has_ramp: false,
            ground_blocked: false,
            build_blocked: false,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct OverlayEffects {
    overlay_blocks: bool,
    /// Overlay is a crate (Crate=yes). RecalcZoneType → ZoneType 1 (Road).
    is_crate: bool,
    /// Overlay is a gate (Gate=yes). RecalcZoneType → ZoneType 6 (Impassable).
    is_gate: bool,
    /// Overlay is a wall (Wall=yes, NOT Land=Road). RecalcZoneType → ZoneType 2 (Wall).
    is_wall: bool,
    has_bridge_deck: bool,
    bridge_layer: Option<BridgeLayer>,
    /// Low bridges override terrain to Road (NoUseTileLandType=true, Land=Road).
    /// When set, overrides the cell's is_water/is_road/ground_walk_blocked flags.
    is_low_bridge: bool,
    /// Cell has a Tiberium/ore overlay — changes effective land_type to Tiberium (5).
    has_tiberium: bool,
    /// Cell has a road/pavement overlay (Land=Road in rules.ini).
    /// Original engine: RecalcLandType sets LandType=Road(1) when overlay.Wall is true.
    is_road: bool,
}

fn grid_dimensions(cells: &[MapCell]) -> (u16, u16) {
    let mut max_rx: u16 = 0;
    let mut max_ry: u16 = 0;
    let mut found = false;
    for cell in cells {
        found = true;
        max_rx = max_rx.max(cell.rx);
        max_ry = max_ry.max(cell.ry);
    }
    if found {
        (max_rx.saturating_add(1), max_ry.saturating_add(1))
    } else {
        (0, 0)
    }
}

fn normalize_tile_id(tile_index: i32) -> u16 {
    if tile_index == 0xFFFF || tile_index < 0 {
        0
    } else {
        tile_index as u16
    }
}

fn is_wood_bridge_repair_tile(theater_data: Option<&TheaterData>, final_tile_index: i32) -> bool {
    if final_tile_index < 0 {
        return false;
    }
    let Some(td) = theater_data else {
        return false;
    };
    let Some(wood_bridge_set) = td.wood_bridge_set else {
        return false;
    };
    let Some(bounds) = td.lookup.bounds().get(wood_bridge_set as usize) else {
        return false;
    };
    let tile_id = normalize_tile_id(final_tile_index) as u32;
    let start = bounds.start as u32;
    tile_id >= start && tile_id < start + 16
}

const AUTO_TUBE_DIRECTIONS: [u8; 4] = [2, 4, 6, 0];

fn build_auto_low_bridge_tubes(
    cells: &mut [ResolvedTerrainCell],
    width: u16,
    height: u16,
    theater_data: Option<&TheaterData>,
    tubes: &mut Vec<TubeFact>,
) {
    for cell in cells.iter_mut() {
        if cell.yr_cell_land_type != YR_CELL_LAND_TUNNEL || cell.tube_index.is_some() {
            continue;
        }
        let Some(direction) = auto_tube_direction_for_tile(cell.final_tile_index, theater_data)
        else {
            continue;
        };
        let Some(_idx) = (cell.rx < width && cell.ry < height).then_some(()) else {
            continue;
        };
        let Ok(raw_id) = u16::try_from(tubes.len()) else {
            log::warn!(
                "ResolvedTerrain: tube registry exceeded u16::MAX; skipping tube at ({}, {})",
                cell.rx,
                cell.ry
            );
            continue;
        };
        let tube_id = TubeId(raw_id);
        tubes.push(TubeFact::auto_low_bridge((cell.rx, cell.ry), direction));
        cell.tube_index = Some(tube_id);
    }
}

fn seed_explicit_map_tubes(
    cells: &mut [ResolvedTerrainCell],
    width: u16,
    height: u16,
    explicit_tubes: &[TubeFact],
) -> Vec<TubeFact> {
    let mut tubes = Vec::with_capacity(explicit_tubes.len());
    for tube in explicit_tubes {
        let Ok(raw_id) = u16::try_from(tubes.len()) else {
            log::warn!(
                "ResolvedTerrain: explicit [Tubes] registry exceeded u16::MAX; skipping remaining tubes"
            );
            break;
        };
        let tube_id = TubeId(raw_id);
        tubes.push(tube.clone());
        let (rx, ry) = tube.entry;
        if rx >= width || ry >= height {
            log::warn!(
                "ResolvedTerrain: explicit [Tubes] entry cell ({}, {}) outside resolved grid",
                rx,
                ry
            );
            continue;
        }
        let idx = ry as usize * width as usize + rx as usize;
        if let Some(cell) = cells.get_mut(idx) {
            cell.tube_index = Some(tube_id);
        }
    }
    tubes
}

fn auto_tube_direction_for_tile(
    final_tile_index: i32,
    theater_data: Option<&TheaterData>,
) -> Option<u8> {
    let tile_id = normalize_tile_id(final_tile_index);
    let td = theater_data?;
    for tileset_index in [
        td.tunnels,
        td.track_tunnels,
        td.dirt_tunnels,
        td.dirt_track_tunnels,
    ]
    .into_iter()
    .flatten()
    {
        let Some(bounds) = td.lookup.bounds().get(tileset_index as usize) else {
            continue;
        };
        let Some(offset) = tile_id.checked_sub(bounds.start) else {
            continue;
        };
        if offset < 4 {
            return AUTO_TUBE_DIRECTIONS.get(offset as usize).copied();
        }
    }
    None
}

fn direction_offset(direction: u8) -> Option<(i32, i32)> {
    match direction & 7 {
        0 => Some((0, -1)),
        1 => Some((1, -1)),
        2 => Some((1, 0)),
        3 => Some((1, 1)),
        4 => Some((0, 1)),
        5 => Some((-1, 1)),
        6 => Some((-1, 0)),
        7 => Some((-1, -1)),
        _ => None,
    }
}

fn load_tile_metadata(
    theater_data: Option<&TheaterData>,
    asset_manager: Option<&crate::assets::asset_manager::AssetManager>,
    terrain_rules: Option<&TerrainRules>,
    key: TileKey,
    warned_unknown_land_types: &mut HashSet<u8>,
) -> TileMetadata {
    let Some(td) = theater_data else {
        return TileMetadata::default();
    };
    let Some(asset_manager) = asset_manager else {
        return metadata_from_set_name(
            td.lookup
                .tileset_index(key.tile_id)
                .and_then(|idx| td.lookup.set_name(idx)),
            td.lookup.tileset_index(key.tile_id),
        );
    };
    let tileset_index = td.lookup.tileset_index(key.tile_id);
    let set_name = tileset_index.and_then(|idx| td.lookup.set_name(idx));
    let mut metadata = metadata_from_set_name(set_name, tileset_index);

    let Some(filename) = td.lookup.filename(key.tile_id as i32) else {
        return metadata;
    };
    let Some(bytes) = asset_manager.get(filename) else {
        return metadata;
    };
    let Ok(tmp) = TmpFile::from_bytes(&bytes) else {
        return metadata;
    };
    let Some(tile) = tmp
        .tiles
        .get(key.sub_tile as usize)
        .and_then(|t| t.as_ref())
    else {
        apply_land_type_semantics(&mut metadata, terrain_rules, warned_unknown_land_types);
        return metadata;
    };
    // Remember tileset-name road detection before TMP byte overrides it.
    let tileset_says_road = metadata.is_road;
    merge_tmp_metadata(&mut metadata, tile);
    apply_land_type_semantics(&mut metadata, terrain_rules, warned_unknown_land_types);
    // Some road/pavement tilesets encode terrain_type 0 (Clear) in TMP instead of
    // 11 (Road). If the tileset name says "road"/"pavement" but the TMP byte mapped
    // to Clear, trust the tileset name — the visual road should be a road.
    if tileset_says_road && !metadata.is_road && metadata.terrain_class == TerrainClass::Clear {
        metadata.is_road = true;
        metadata.terrain_class = TerrainClass::Road;
    }
    metadata
}

fn metadata_from_set_name(set_name: Option<&str>, tileset_index: Option<u16>) -> TileMetadata {
    let lower = set_name.unwrap_or("").to_ascii_lowercase();
    let is_water = lower.contains("water");
    let is_cliff_like =
        lower.contains("cliff") || lower.contains("rock") || lower.contains("shore");
    let is_rough = lower.contains("rough");
    let is_road = lower.contains("road") || lower.contains("pavement") || lower.contains("pave");
    let land_type = if is_water {
        crate::sim::pathfinding::passability::LandType::Water.as_index()
    } else if is_road {
        crate::sim::pathfinding::passability::LandType::Road.as_index()
    } else if is_rough {
        crate::sim::pathfinding::passability::LandType::Rough.as_index()
    } else if is_cliff_like {
        crate::sim::pathfinding::passability::LandType::Rock.as_index()
    } else {
        crate::sim::pathfinding::passability::LandType::Clear.as_index()
    };
    let terrain_class = if is_water {
        TerrainClass::Water
    } else if lower.contains("cliff") {
        TerrainClass::Cliff
    } else if lower.contains("rock") {
        TerrainClass::Rock
    } else if is_road {
        TerrainClass::Road
    } else if is_rough {
        TerrainClass::Rough
    } else if !lower.is_empty() {
        TerrainClass::Clear
    } else {
        TerrainClass::Unknown
    };

    TileMetadata {
        tileset_index,
        land_type,
        yr_cell_land_type: land_type,
        terrain_class,
        is_water,
        is_cliff_like,
        is_rough,
        is_road,
        ground_blocked: is_water || is_cliff_like,
        build_blocked: is_water || is_cliff_like,
        ..TileMetadata::default()
    }
}

fn merge_tmp_metadata(metadata: &mut TileMetadata, tile: &TmpTile) {
    metadata.raw_land_type = tile.terrain_type;
    metadata.yr_cell_land_type = yr_cell_land_type_from_tmp(tile.terrain_type);
    metadata.land_type =
        crate::sim::pathfinding::passability::tmp_terrain_to_land_type(tile.terrain_type)
            .as_index();
    metadata.slope_type = tile.ramp_type;
    metadata.template_height = tile.height;
    metadata.render_offset_x = tile.offset_x;
    metadata.render_offset_y = tile.offset_y;
    metadata.has_ramp = tile.ramp_type != 0;
    metadata.has_tmp_metadata = true;
    metadata.radar_left = tile.radar_left;
    metadata.radar_right = tile.radar_right;
    metadata.has_damaged_data = tile.has_damaged_data;
}

fn yr_cell_land_type_from_tmp(tmp_terrain_type: u8) -> u8 {
    if tmp_terrain_type == 5 {
        YR_CELL_LAND_TUNNEL
    } else {
        crate::sim::pathfinding::passability::tmp_terrain_to_land_type(tmp_terrain_type).as_index()
    }
}

/// Maps TMP ramp_type byte to canonical direction.
/// Values from TS++ TIBSUN_DEFINES.H. Tilt matrix angles:
/// 270 deg=W, 180 deg=N, 90 deg=E, 0 deg=S for slope types 1-4.
fn canonical_ramp_from_slope_type(slope_type: u8) -> Option<RampDirection> {
    match slope_type {
        1 => Some(RampDirection::West),
        2 => Some(RampDirection::North),
        3 => Some(RampDirection::East),
        4 => Some(RampDirection::South),
        _ => None,
    }
}

fn apply_land_type_semantics(
    metadata: &mut TileMetadata,
    terrain_rules: Option<&TerrainRules>,
    warned_unknown_land_types: &mut HashSet<u8>,
) {
    let Some(terrain_rules) = terrain_rules else {
        return;
    };
    if !metadata.has_tmp_metadata {
        return;
    }
    // Use the raw TMP byte (0-15) for rules.ini section lookup — that's how the
    // KNOWN_LAND_TYPES table is indexed.  The mapped land_type (0-7) is already
    // stored on the metadata for passability matrix lookups.
    let Some(semantics) = terrain_rules
        .semantics_for_land_type(metadata.raw_land_type)
        .copied()
    else {
        if warned_unknown_land_types.insert(metadata.raw_land_type) {
            log::warn!(
                "Unknown TMP LandType byte {}; falling back to tileset-name heuristics",
                metadata.raw_land_type
            );
        }
        return;
    };

    metadata.terrain_class = semantics.terrain_class;
    metadata.speed_costs = semantics.speed_costs;
    metadata.is_water = semantics.water;
    metadata.is_cliff_like = semantics.cliff_like;
    metadata.is_rough = semantics.rough;
    metadata.is_road = semantics.road;
    metadata.ground_blocked = semantics.ground_blocked;
    metadata.build_blocked = !semantics.buildable;
}

fn classify_overlay_effects(
    overlays: Option<&Vec<&OverlayEntry>>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    level: u8,
) -> OverlayEffects {
    let mut result = OverlayEffects::default();
    let Some(entries) = overlays else {
        return result;
    };
    for overlay in entries {
        let name = overlay_registry
            .and_then(|reg| reg.name(overlay.overlay_id))
            .unwrap_or("");
        // Bridge overlays identified by hardcoded index, matching original engine.
        let is_bridge = crate::map::overlay_types::is_bridge_overlay_index(overlay.overlay_id);

        let flags = overlay_registry.and_then(|reg| reg.flags(overlay.overlay_id));
        let is_wall = flags.map(|f| f.wall).unwrap_or(false);
        let is_tiberium = flags.map(|f| f.tiberium).unwrap_or(false);
        let is_crate = flags.map(|f| f.crate_type).unwrap_or(false);
        let is_gate = flags.map(|f| f.is_gate).unwrap_or(false);

        // Road/pavement overlays have Land=Road in rules.ini. In the original
        // engine, Wall=yes overlays with Land=Road act as road surfaces, not
        // movement blockers. Only walls WITHOUT Land=Road actually block.
        let is_road_overlay = flags
            .and_then(|f| f.land.as_deref())
            .map(|land| land.eq_ignore_ascii_case("Road"))
            .unwrap_or(false);

        if is_road_overlay {
            result.is_road = true;
        } else if is_wall {
            result.overlay_blocks = true;
            result.is_wall = true;
        }
        if is_tiberium {
            result.has_tiberium = true;
        }
        if is_crate {
            result.is_crate = true;
        }
        if is_gate {
            result.is_gate = true;
        }
        if is_bridge && result.bridge_layer.is_none() {
            result.has_bridge_deck = true;
            // Direction determined by index: 24/237=EW, 25/238=NS, rest=Low.
            let direction = match overlay.overlay_id {
                24 | 237 => BridgeDirection::EastWest,
                25 | 238 => BridgeDirection::NorthSouth,
                _ => BridgeDirection::Low,
            };
            // High bridges: deck 4 levels above ground (HighBridgeHeight=4).
            // Low bridges: deck at ground level (no elevation change).
            let deck_level = match direction {
                BridgeDirection::EastWest | BridgeDirection::NorthSouth => level.saturating_add(4),
                BridgeDirection::Low => level,
            };
            if direction == BridgeDirection::Low {
                result.is_low_bridge = true;
            }
            result.bridge_layer = Some(BridgeLayer {
                overlay_id: overlay.overlay_id,
                overlay_name: name.to_string(),
                deck_level,
                direction,
            });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::tmp_file::TmpTile;
    use crate::map::overlay::TerrainObject;
    use crate::map::overlay_types::OverlayTypeRegistry;
    use crate::map::tube_facts::TubeSource;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::terrain_rules::{TerrainClass, TerrainRules};
    use std::collections::HashSet;

    fn make_map(
        cells: Vec<MapCell>,
        overlays: Vec<OverlayEntry>,
        terrain_objects: Vec<TerrainObject>,
    ) -> MapFile {
        MapFile {
            header: crate::map::map_file::MapHeader {
                theater: "TEMPERATE".to_string(),
                width: 4,
                height: 4,
                local_left: 0,
                local_top: 0,
                local_width: 4,
                local_height: 4,
            },
            basic: crate::map::basic::BasicSection::default(),
            briefing: crate::map::briefing::BriefingSection::default(),
            preview: crate::map::preview::PreviewSection::default(),
            cells,
            entities: Vec::new(),
            overlays,
            overlay_data: crate::map::overlay::OverlayDataPack::default(),
            smudges: Vec::new(),
            terrain_objects,
            waypoints: HashMap::new(),
            cell_tags: HashMap::new(),
            tags: HashMap::new(),
            triggers: HashMap::new(),
            events: HashMap::new(),
            actions: HashMap::new(),
            local_variables: HashMap::new(),
            trigger_graph: crate::map::trigger_graph::TriggerGraph::default(),
            special_flags: crate::map::basic::SpecialFlagsSection::default(),
            explicit_tubes: Vec::new(),
            ini: IniFile::from_str(""),
        }
    }

    fn synthetic_theater_with_wood_bridge_set() -> TheaterData {
        let ini = b"[TileSet0000]\nTilesInSet=10\nFileName=clear\nSetName=Clear\n\n\
                    [TileSet0001]\nTilesInSet=20\nFileName=wood\nSetName=Wood Bridge\n";
        let lookup = crate::map::theater::parse_tileset_ini(ini, "tem").unwrap();
        let empty_palette = crate::assets::pal_file::Palette::from_bytes(&[0u8; 768])
            .expect("768-byte zero palette parses");
        TheaterData {
            lookup,
            iso_palette: empty_palette.clone(),
            unit_palette: empty_palette.clone(),
            tiberium_palette: empty_palette,
            extension: "tem",
            ini_data: Vec::new(),
            bridge_set: None,
            wood_bridge_set: Some(1),
            slope_set_pieces: None,
            slope_set_pieces2: None,
            bridge_top_left_1: None,
            bridge_top_left_2: None,
            bridge_top_right_1: None,
            bridge_top_right_2: None,
            bridge_middle_1: None,
            bridge_middle_2: None,
            tunnels: None,
            track_tunnels: None,
            dirt_tunnels: None,
            dirt_track_tunnels: None,
        }
    }

    fn make_test_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
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
            speed_costs: SpeedCostProfile::default(),
            is_water: false,
            is_cliff_like: false,
            is_cliff_redraw: false,
            variant: 0,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            overlay_blocks: false,
            zone_type: 0,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
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

    #[test]
    fn direction_8_steps_through_cell_tube() {
        let mut cells = vec![
            make_test_cell(0, 0),
            make_test_cell(1, 0),
            make_test_cell(2, 0),
        ];
        cells[1].yr_cell_land_type = YR_CELL_LAND_TUNNEL;
        cells[1].tube_index = Some(TubeId(0));
        let tube = TubeFact {
            entry: (1, 0),
            exit: (2, 0),
            direction: 2,
            path_steps: Vec::new(),
            source: TubeSource::AutoLowBridge,
        };
        let grid = ResolvedTerrainGrid::from_cells_with_tubes(3, 1, cells, vec![tube]);

        assert_eq!(grid.step_coord_by_direction((1, 0), 8), Some((2, 0)));
        assert_eq!(grid.walk_directions_from((0, 0), &[2, 8]), Some((2, 0)));
    }

    #[test]
    fn direction_8_without_valid_tube_returns_zero_coord() {
        let grid = ResolvedTerrainGrid::from_cells(1, 1, vec![make_test_cell(0, 0)]);

        assert_eq!(grid.step_coord_by_direction((0, 0), 8), Some((0, 0)));
    }

    #[test]
    fn explicit_map_tubes_seed_resolved_grid() {
        let mut map = make_map(
            vec![
                MapCell {
                    rx: 0,
                    ry: 0,
                    tile_index: 0,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 1,
                    ry: 0,
                    tile_index: 0,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 2,
                    ry: 0,
                    tile_index: 0,
                    sub_tile: 0,
                    z: 0,
                },
            ],
            Vec::new(),
            Vec::new(),
        );
        map.explicit_tubes = vec![TubeFact::explicit((0, 0), (2, 0), 2, vec![2, 2])];

        let grid = ResolvedTerrainGrid::build(&map, None, None, None, None, false, 0);

        let cell = grid.cell(0, 0).expect("entry cell");
        assert_eq!(cell.tube_index, Some(TubeId(0)));
        assert_eq!(grid.tube_facts().len(), 1);
        assert_eq!(grid.tube_facts()[0].source, TubeSource::ExplicitMap);
        assert_eq!(grid.step_coord_by_direction((0, 0), 8), Some((2, 0)));
    }

    #[test]
    fn test_resolved_grid_preserves_raw_fields_and_fills_clear_cells() {
        let map = make_map(
            vec![MapCell {
                rx: 1,
                ry: 1,
                tile_index: 5,
                sub_tile: 3,
                z: 2,
            }],
            Vec::new(),
            Vec::new(),
        );
        let grid = ResolvedTerrainGrid::build(&map, None, None, None, None, false, 0);
        assert_eq!(grid.width(), 2);
        assert_eq!(grid.height(), 2);

        let cell = grid.cell(1, 1).expect("resolved cell");
        assert_eq!(cell.source_tile_index, 5);
        assert_eq!(cell.source_sub_tile, 3);
        assert_eq!(cell.final_tile_index, 5);
        assert_eq!(cell.final_sub_tile, 3);
        assert_eq!(cell.level, 2);
        assert!(!cell.filled_clear);

        let clear = grid.cell(0, 0).expect("filled clear");
        assert!(clear.filled_clear);
        assert_eq!(clear.final_tile_index, 0);
        assert_eq!(clear.level, 0);
    }

    #[test]
    fn wood_bridge_repair_tile_uses_first_16_tiles_of_wood_bridge_set() {
        let theater = synthetic_theater_with_wood_bridge_set();
        let map = make_map(
            vec![
                MapCell {
                    rx: 0,
                    ry: 0,
                    tile_index: 9,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 1,
                    ry: 0,
                    tile_index: 10,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 2,
                    ry: 0,
                    tile_index: 25,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 3,
                    ry: 0,
                    tile_index: 26,
                    sub_tile: 0,
                    z: 0,
                },
            ],
            Vec::new(),
            Vec::new(),
        );

        let grid = ResolvedTerrainGrid::build(&map, Some(&theater), None, None, None, false, 0);

        assert!(!grid.cell(0, 0).unwrap().is_wood_bridge_repair_tile);
        assert!(grid.cell(1, 0).unwrap().is_wood_bridge_repair_tile);
        assert!(grid.cell(2, 0).unwrap().is_wood_bridge_repair_tile);
        assert!(!grid.cell(3, 0).unwrap().is_wood_bridge_repair_tile);
    }

    #[test]
    fn test_merge_tmp_metadata_reads_land_and_slope_bytes() {
        let mut metadata = TileMetadata::default();
        let tile = TmpTile {
            height: 4,
            terrain_type: 7,
            ramp_type: 2,
            radar_left: [100, 120, 80],
            radar_right: [90, 110, 70],
            pixels: Vec::new(),
            depth: Vec::new(),
            pixel_width: 60,
            pixel_height: 30,
            offset_x: -5,
            offset_y: -6,
            has_damaged_data: false,
        };
        merge_tmp_metadata(&mut metadata, &tile);
        assert_eq!(metadata.land_type, 7);
        assert_eq!(metadata.slope_type, 2);
        assert_eq!(metadata.template_height, 4);
        assert_eq!(metadata.render_offset_x, -5);
        assert_eq!(metadata.render_offset_y, -6);
        assert!(metadata.has_ramp);
        assert!(metadata.has_tmp_metadata);
        assert_eq!(metadata.radar_left, [100, 120, 80]);
        assert_eq!(metadata.radar_right, [90, 110, 70]);
    }

    #[test]
    fn test_canonical_ramp_detection_only_marks_slope_types_one_to_four() {
        assert_eq!(canonical_ramp_from_slope_type(1), Some(RampDirection::West));
        assert_eq!(
            canonical_ramp_from_slope_type(4),
            Some(RampDirection::South)
        );
        assert_eq!(canonical_ramp_from_slope_type(0), None);
        assert_eq!(canonical_ramp_from_slope_type(7), None);
    }

    #[test]
    fn test_bridge_overlay_creates_upper_layer_without_ground_block() {
        // BRIDGE1 is hardcoded at overlay index 24 in the original engine.
        // Build a registry large enough so index 24 resolves to "BRIDGE1".
        let mut ini_str = String::from("[OverlayTypes]\n");
        for i in 0..24 {
            ini_str.push_str(&format!("{i}=FILLER{i}\n"));
        }
        ini_str.push_str("24=BRIDGE1\n");
        let ini = IniFile::from_str(&ini_str);
        let reg = OverlayTypeRegistry::from_ini(&ini, None);
        let effects = classify_overlay_effects(
            Some(&vec![&OverlayEntry {
                rx: 0,
                ry: 0,
                overlay_id: 24,
                frame: 0,
            }]),
            Some(&reg),
            3,
        );
        assert!(effects.has_bridge_deck);
        assert!(!effects.overlay_blocks);
        assert_eq!(
            effects
                .bridge_layer
                .as_ref()
                .map(|b| b.overlay_name.as_str()),
            Some("BRIDGE1")
        );
        // BRIDGE1 = EastWest high bridge: deck_level = ground(3) + HighBridgeHeight(4) = 7.
        assert_eq!(effects.bridge_layer.as_ref().map(|b| b.deck_level), Some(7));
        assert_eq!(
            effects.bridge_layer.as_ref().map(|b| b.direction),
            Some(BridgeDirection::EastWest)
        );
    }

    #[test]
    fn test_rules_backed_land_type_overrides_tileset_heuristics_when_tmp_exists() {
        let terrain_rules =
            TerrainRules::from_ini(&IniFile::from_str("[Rough]\nBuildable=yes\nTrack=75%\n"));
        let mut metadata = metadata_from_set_name(Some("Water"), Some(2));
        let tile = TmpTile {
            height: 0,
            terrain_type: 14,
            ramp_type: 0,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            pixels: Vec::new(),
            depth: Vec::new(),
            pixel_width: 60,
            pixel_height: 30,
            offset_x: 0,
            offset_y: 0,
            has_damaged_data: false,
        };
        merge_tmp_metadata(&mut metadata, &tile);
        let mut warned = HashSet::new();
        apply_land_type_semantics(&mut metadata, Some(&terrain_rules), &mut warned);

        assert_eq!(metadata.terrain_class, TerrainClass::Rough);
        assert!(metadata.is_rough);
        assert!(!metadata.is_water);
        assert!(!metadata.ground_blocked);
        assert!(!metadata.build_blocked);
    }

    #[test]
    fn test_unknown_land_type_keeps_tileset_fallback() {
        // Use a LandType byte outside the 0-15 range (all 0-15 are now mapped).
        // Byte 200 is genuinely unknown and should fall back to tileset-name heuristics.
        let terrain_rules = TerrainRules::from_ini(&IniFile::from_str(""));
        let mut metadata = metadata_from_set_name(Some("Water Cliffs"), Some(5));
        let tile = TmpTile {
            height: 0,
            terrain_type: 200,
            ramp_type: 0,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            pixels: Vec::new(),
            depth: Vec::new(),
            pixel_width: 60,
            pixel_height: 30,
            offset_x: 0,
            offset_y: 0,
            has_damaged_data: false,
        };
        merge_tmp_metadata(&mut metadata, &tile);
        let mut warned = HashSet::new();
        apply_land_type_semantics(&mut metadata, Some(&terrain_rules), &mut warned);

        assert_eq!(metadata.terrain_class, TerrainClass::Water);
        assert!(metadata.is_water);
        assert!(metadata.is_cliff_like);
        assert!(metadata.ground_blocked);
        assert_eq!(warned, HashSet::from([200]));
    }

    #[test]
    fn test_tileset_water_fallback_sets_water_land_type() {
        let metadata = metadata_from_set_name(Some("TEMPERATE WATER"), Some(5));
        assert!(metadata.is_water);
        assert_eq!(
            metadata.land_type,
            crate::sim::pathfinding::passability::LandType::Water.as_index()
        );
    }

    #[test]
    fn test_canonical_ramp_is_ground_passable_but_stays_non_buildable() {
        let map = make_map(
            vec![MapCell {
                rx: 0,
                ry: 0,
                tile_index: 0,
                sub_tile: 0,
                z: 0,
            }],
            Vec::new(),
            Vec::new(),
        );
        let terrain_rules = TerrainRules::from_ini(&IniFile::from_str("[Cliff]\nBuildable=no\n"));
        let mut metadata = TileMetadata {
            has_tmp_metadata: true,
            raw_land_type: 15,
            land_type: crate::sim::pathfinding::passability::tmp_terrain_to_land_type(15)
                .as_index(),
            slope_type: 2,
            terrain_class: TerrainClass::Cliff,
            ground_blocked: true,
            build_blocked: true,
            is_cliff_like: true,
            has_ramp: true,
            ..TileMetadata::default()
        };
        let mut warned = HashSet::new();
        apply_land_type_semantics(&mut metadata, Some(&terrain_rules), &mut warned);

        let canonical_ramp = canonical_ramp_from_slope_type(metadata.slope_type);
        let base_ground_walk_blocked = canonical_ramp.is_none() && metadata.ground_blocked;
        assert!(!base_ground_walk_blocked);
        let grid = ResolvedTerrainGrid::from_cells(
            1,
            1,
            vec![ResolvedTerrainCell {
                rx: 0,
                ry: 0,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                is_wood_bridge_repair_tile: false,
                level: 0,
                filled_clear: false,
                tileset_index: Some(0),
                land_type: metadata.land_type,
                yr_cell_land_type: metadata.yr_cell_land_type,
                slope_type: metadata.slope_type,
                template_height: 0,
                render_offset_x: 0,
                render_offset_y: 0,
                terrain_class: metadata.terrain_class,
                speed_costs: metadata.speed_costs,
                is_water: false,
                is_cliff_like: true,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                is_cliff_redraw: false,
                variant: 0,
                has_ramp: true,
                canonical_ramp,
                ground_walk_blocked: false,
                terrain_object_blocks: false,
                overlay_blocks: false,
                zone_type: 0,
                base_ground_walk_blocked: false,
                base_build_blocked: true,
                build_blocked: true,
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
            }],
        );
        let cell = grid.cell(0, 0).expect("resolved ramp cell");
        assert_eq!(cell.canonical_ramp, Some(RampDirection::North));
        assert!(!cell.ground_walk_blocked);
        assert!(cell.build_blocked);
        assert_eq!(map.header.width, 4);
    }

    #[test]
    fn test_cliff_redraw_flag_set_when_height_diff_ge_4() {
        // Cell at (3,3) z=6, neighbor at (1,1) z=0. Height diff 6 >= 4.
        let map = make_map(
            vec![
                MapCell {
                    rx: 1,
                    ry: 1,
                    tile_index: 0,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 2,
                    ry: 1,
                    tile_index: 0,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 1,
                    ry: 2,
                    tile_index: 0,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 2,
                    ry: 2,
                    tile_index: 0,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 3,
                    ry: 3,
                    tile_index: 0,
                    sub_tile: 0,
                    z: 6,
                },
            ],
            Vec::new(),
            Vec::new(),
        );
        let grid = ResolvedTerrainGrid::build(&map, None, None, None, None, false, 0);
        let cell = grid.cell(3, 3).expect("high cell");
        assert!(
            cell.is_cliff_redraw,
            "height diff 6 >= 4 should flag cliff redraw"
        );
    }

    #[test]
    fn test_cliff_redraw_flag_not_set_when_height_diff_lt_4() {
        // Cell at (3,3) z=3, neighbors at z=0. Height diff 3 < 4.
        let map = make_map(
            vec![
                MapCell {
                    rx: 1,
                    ry: 1,
                    tile_index: 0,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 2,
                    ry: 2,
                    tile_index: 0,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 3,
                    ry: 3,
                    tile_index: 0,
                    sub_tile: 0,
                    z: 3,
                },
            ],
            Vec::new(),
            Vec::new(),
        );
        let grid = ResolvedTerrainGrid::build(&map, None, None, None, None, false, 0);
        let cell = grid.cell(3, 3).expect("slightly elevated cell");
        assert!(
            !cell.is_cliff_redraw,
            "height diff 3 < 4 should NOT flag cliff redraw"
        );
    }

    #[test]
    fn cliff_back_impassability_marks_low_cell() {
        // Cell (1,1) at level 0, cell (1,0) at level 4.
        // Neighbor offset (0,-1) means (1,0) is checked from (1,1).
        // Height diff = 4 >= 4 → cell (1,1) should be marked impassable.
        let map = make_map(
            vec![
                MapCell {
                    rx: 0,
                    ry: 0,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 1,
                    ry: 0,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 4,
                },
                MapCell {
                    rx: 0,
                    ry: 1,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 1,
                    ry: 1,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 0,
                },
            ],
            Vec::new(),
            Vec::new(),
        );
        let grid = ResolvedTerrainGrid::build(&map, None, None, None, None, false, 2);
        let cell = grid.cell(1, 1).unwrap();
        assert!(
            cell.ground_walk_blocked,
            "Cell at base of cliff should be blocked"
        );
        assert!(
            cell.is_cliff_like,
            "Cell at base of cliff should be cliff-like"
        );
        assert_eq!(
            cell.land_type,
            crate::sim::pathfinding::passability::LandType::Rock.as_index(),
            "Cell at base of cliff should have Rock land type"
        );
    }

    #[test]
    fn cliff_back_impassability_skips_when_disabled() {
        let map = make_map(
            vec![
                MapCell {
                    rx: 0,
                    ry: 0,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 1,
                    ry: 0,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 4,
                },
                MapCell {
                    rx: 0,
                    ry: 1,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 1,
                    ry: 1,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 0,
                },
            ],
            Vec::new(),
            Vec::new(),
        );
        // cliff_back_impassability = 0 → disabled
        let grid = ResolvedTerrainGrid::build(&map, None, None, None, None, false, 0);
        let cell = grid.cell(1, 1).unwrap();
        assert!(
            !cell.ground_walk_blocked,
            "Should NOT be blocked when disabled"
        );
    }

    #[test]
    fn cliff_back_impassability_ignores_small_height_diff() {
        let map = make_map(
            vec![
                MapCell {
                    rx: 0,
                    ry: 0,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 1,
                    ry: 0,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 3,
                },
                MapCell {
                    rx: 0,
                    ry: 1,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 0,
                },
                MapCell {
                    rx: 1,
                    ry: 1,
                    tile_index: -1,
                    sub_tile: 0,
                    z: 0,
                },
            ],
            Vec::new(),
            Vec::new(),
        );
        let grid = ResolvedTerrainGrid::build(&map, None, None, None, None, false, 2);
        let cell = grid.cell(1, 1).unwrap();
        assert!(
            !cell.ground_walk_blocked,
            "Height diff 3 should NOT trigger (threshold is 4)"
        );
    }
}
