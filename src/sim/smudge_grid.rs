//! Per-cell mutable smudge state — runtime craters, scorch marks, and pre-placed map decals.
//!
//! Seeded from the map's [Smudge] section at sim init, mutated by the smudge
//! dispatcher during combat.
//!
//! Dependency rules: depends on rules/, map/, and other sim/ modules.
//! Never depends on render/, ui/, sidebar/, audio/, net/.

use crate::map::map_file::MapSmudgeEntry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::smudge_type::SmudgeTypeRegistry;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::rng::SimRng;

/// Smudge category — Burn for scorches, Crater for explosion craters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmudgeKind {
    Burn,
    Crater,
}

/// Per-cell smudge slot.
///
/// `type_id` indexes into SmudgeTypeRegistry. None = no smudge on this cell.
/// `footprint_origin` is the top-left cell of the W×H footprint that owns this cell.
/// `frame_offset` is the SHP frame index within the footprint
/// (computed as `(rx - origin.rx) + (ry - origin.ry) * footprint_width`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default,
         serde::Serialize, serde::Deserialize)]
pub struct SmudgeCell {
    pub type_id: Option<u16>,
    pub footprint_origin: Option<(u16, u16)>,
    pub frame_offset: u8,
}

/// Per-cell smudge grid. Flat Vec indexed by `ry * width + rx`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmudgeGrid {
    width: u16,
    height: u16,
    cells: Vec<SmudgeCell>,
    /// Cells mutated this tick — drained per tick by the render-update path.
    /// Not part of game state; never serialized.
    #[serde(skip, default)]
    dirty_cells: Vec<(u16, u16)>,
}

impl SmudgeGrid {
    pub fn new(width: u16, height: u16) -> Self {
        let count: usize = width as usize * height as usize;
        Self {
            width,
            height,
            cells: vec![SmudgeCell::default(); count],
            dirty_cells: Vec::new(),
        }
    }

    pub fn width(&self) -> u16 { self.width }
    pub fn height(&self) -> u16 { self.height }

    pub fn cell(&self, rx: u16, ry: u16) -> &SmudgeCell {
        match self.index_of(rx, ry) {
            Some(i) => &self.cells[i],
            None => &DEFAULT_CELL,
        }
    }

    fn index_of(&self, rx: u16, ry: u16) -> Option<usize> {
        if rx >= self.width || ry >= self.height {
            None
        } else {
            Some(ry as usize * self.width as usize + rx as usize)
        }
    }

    pub fn drain_dirty(&mut self) -> Vec<(u16, u16)> {
        std::mem::take(&mut self.dirty_cells)
    }

    pub fn iter_occupied(&self) -> impl Iterator<Item = (u16, u16, &SmudgeCell)> {
        self.cells.iter().enumerate().filter_map(move |(idx, c)| {
            if c.type_id.is_some() {
                let rx = (idx % self.width as usize) as u16;
                let ry = (idx / self.width as usize) as u16;
                Some((rx, ry, c))
            } else {
                None
            }
        })
    }

    /// Test-only direct cell mutation. Bypasses CanPlaceHere — use only in
    /// unit tests that need to seed a known SmudgeGrid state for hashing or
    /// snapshot round-trip verification.
    #[cfg(test)]
    pub fn test_force_set(&mut self, rx: u16, ry: u16, cell: SmudgeCell) {
        if let Some(idx) = self.index_of(rx, ry) {
            self.cells[idx] = cell;
            self.dirty_cells.push((rx, ry));
        }
    }
}

const DEFAULT_CELL: SmudgeCell = SmudgeCell {
    type_id: None,
    footprint_origin: None,
    frame_offset: 0,
};

impl SmudgeGrid {
    pub fn from_map_entries(
        entries: &[MapSmudgeEntry],
        registry: &SmudgeTypeRegistry,
        terrain: &ResolvedTerrainGrid,
        overlay: &OverlayGrid,
        width: u16,
        height: u16,
    ) -> Self {
        let mut grid = Self::new(width, height);
        for entry in entries {
            let Some(type_id) = registry.find_by_name(&entry.type_name) else {
                log::warn!(
                    "[Smudge] entry references unknown SmudgeType '{}', skipping",
                    entry.type_name
                );
                continue;
            };
            let Some(def) = registry.get(type_id) else { continue; };
            if !grid.passes_placement_gates(
                entry.rx, entry.ry, def.width, def.height, terrain, overlay, None,
            ) {
                continue;
            }
            grid.write_footprint(entry.rx, entry.ry, type_id, def.width, def.height);
        }
        // Map-load doesn't dirty render; clear the queue.
        grid.dirty_cells.clear();
        grid
    }
}

impl SmudgeGrid {
    /// Six-gate placement check: in-bounds, no smudge, no overlay,
    /// no building, slope==0, accepts_smudge. All cells in the W×H footprint must pass.
    fn passes_placement_gates(
        &self,
        rx: u16, ry: u16, w: u8, h: u8,
        terrain: &ResolvedTerrainGrid,
        overlay: &OverlayGrid,
        occupancy: Option<&OccupancyGrid>,
    ) -> bool {
        for dy in 0..h as u16 {
            for dx in 0..w as u16 {
                let cx = rx + dx;
                let cy = ry + dy;
                if cx >= self.width || cy >= self.height {
                    return false;
                }
                if self.cell(cx, cy).type_id.is_some() {
                    return false;
                }
                if overlay.cell(cx, cy).overlay_id.is_some() {
                    return false;
                }
                let Some(tcell) = terrain.cell(cx, cy) else {
                    return false;
                };
                if tcell.slope_type != 0 {
                    return false;
                }
                if !tcell.accepts_smudge {
                    return false;
                }
                if let Some(occ) = occupancy {
                    if cell_has_building(occ, cx, cy) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn write_footprint(&mut self, rx: u16, ry: u16, type_id: u16, w: u8, h: u8) {
        for dy in 0..h as u16 {
            for dx in 0..w as u16 {
                let cx = rx + dx;
                let cy = ry + dy;
                let Some(idx) = self.index_of(cx, cy) else { continue; };
                self.cells[idx] = SmudgeCell {
                    type_id: Some(type_id),
                    footprint_origin: Some((rx, ry)),
                    frame_offset: (dx as u8) + (dy as u8) * w,
                };
                self.dirty_cells.push((cx, cy));
            }
        }
    }
}

fn cell_has_building(occupancy: &OccupancyGrid, rx: u16, ry: u16) -> bool {
    use crate::sim::movement::locomotor::MovementLayer;
    occupancy
        .get(rx, ry)
        .map_or(false, |c| c.has_blockers_on(MovementLayer::Ground))
}

impl SmudgeGrid {
    /// Try to place a smudge of the given kind at `coord` (lepton-space).
    /// Runs the full filter + size selector + CanPlaceHere chain.
    ///
    /// Returns true if a smudge was placed, false otherwise.
    /// Callers MUST destroy the underlying ore (reduce_tiberium(6)) BEFORE this
    /// for the crater path — ore is destroyed even on placement failure.
    #[allow(clippy::too_many_arguments)]
    pub fn try_place(
        &mut self,
        kind: SmudgeKind,
        coord: SimCoord,
        dmg: i32,
        dmg2: i32,
        force_big: bool,
        registry: &SmudgeTypeRegistry,
        terrain: &ResolvedTerrainGrid,
        overlay: &OverlayGrid,
        occupancy: &OccupancyGrid,
        rng: &mut SimRng,
    ) -> bool {
        let rx: u16 = (coord.x >> 8).clamp(0, self.width as i32 - 1) as u16;
        let ry: u16 = (coord.y >> 8).clamp(0, self.height as i32 - 1) as u16;

        let unfiltered: Vec<u16> = registry.iter_with_id()
            .filter(|(_, def)| match kind {
                SmudgeKind::Burn => def.burn,
                SmudgeKind::Crater => def.crater,
            })
            .map(|(id, _)| id)
            .collect();
        if unfiltered.is_empty() { return false; }

        let mut filtered: Vec<u16> = if force_big {
            unfiltered.iter().copied()
                .filter(|&id| {
                    let d = registry.get(id).unwrap();
                    d.width >= 2 && d.height >= 2
                }).collect()
        } else {
            unfiltered.iter().copied()
                .filter(|&id| {
                    let d = registry.get(id).unwrap();
                    (d.width == 1 && d.height == 1)
                        || (0x3C < dmg && 0x32 < dmg2)
                }).collect()
        };
        if filtered.is_empty() {
            filtered = unfiltered;
        }

        let pick_idx = (rng.next_range_u32(filtered.len() as u32)) as usize;
        let chosen_id = filtered[pick_idx];
        let chosen = registry.get(chosen_id).unwrap();

        if !self.passes_placement_gates(
            rx, ry, chosen.width, chosen.height, terrain, overlay, Some(occupancy),
        ) {
            return false;
        }
        self.write_footprint(rx, ry, chosen_id, chosen.width, chosen.height);
        true
    }
}

/// Lepton-space coord (256 leptons = 1 cell, matches gamemd's CoordStruct).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::ResolvedTerrainCell;

    fn make_terrain(w: u16, h: u16, accepts: bool) -> ResolvedTerrainGrid {
        let mut cells: Vec<ResolvedTerrainCell> = Vec::with_capacity((w * h) as usize);
        for ry in 0..h {
            for rx in 0..w {
                cells.push(ResolvedTerrainCell {
                    rx, ry,
                    source_tile_index: 0, source_sub_tile: 0,
                    final_tile_index: 0, final_sub_tile: 0,
                    level: 0, filled_clear: true, tileset_index: Some(0),
                    land_type: 0, slope_type: 0, template_height: 0,
                    render_offset_x: 0, render_offset_y: 0,
                    terrain_class: Default::default(),
                    speed_costs: Default::default(),
                    is_water: false, is_cliff_like: false,
                    is_rough: false, is_road: false,
                    is_cliff_redraw: false, variant: 0,
                    has_ramp: false, canonical_ramp: None,
                    ground_walk_blocked: false, terrain_object_blocks: false,
                    overlay_blocks: false, zone_type: 0,
                    base_ground_walk_blocked: false, base_build_blocked: false,
                    build_blocked: false,
                    has_bridge_deck: false, bridge_walkable: false,
                    bridge_transition: false, bridge_deck_level: 0,
                    bridge_layer: None,
                    radar_left: [0; 3], radar_right: [0; 3],
                    accepts_smudge: accepts,
                });
            }
        }
        ResolvedTerrainGrid::from_cells(w, h, cells)
    }

    fn make_registry_with_one_crater_1x1() -> SmudgeTypeRegistry {
        let ini = crate::rules::ini_parser::IniFile::from_bytes(
            b"[SmudgeTypes]\n1=CR1\n[CR1]\nCrater=yes\nWidth=1\nHeight=1\n"
        ).unwrap();
        SmudgeTypeRegistry::from_rules_ini(&ini)
    }

    #[test]
    fn try_place_writes_one_cell_for_1x1() {
        let mut grid = SmudgeGrid::new(8, 8);
        let registry = make_registry_with_one_crater_1x1();
        let terrain = make_terrain(8, 8, true);
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        assert!(grid.try_place(
            SmudgeKind::Crater, coord, 30, 30, false,
            &registry, &terrain, &overlay, &occupancy, &mut rng,
        ));
        assert!(grid.cell(4, 4).type_id.is_some());
    }

    #[test]
    fn rejects_when_accepts_smudge_false() {
        let mut grid = SmudgeGrid::new(8, 8);
        let registry = make_registry_with_one_crater_1x1();
        let terrain = make_terrain(8, 8, false); // Morphable=no
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        assert!(!grid.try_place(
            SmudgeKind::Crater, coord, 30, 30, false,
            &registry, &terrain, &overlay, &occupancy, &mut rng,
        ));
        assert!(grid.cell(4, 4).type_id.is_none());
    }

    #[test]
    fn rejects_when_overlay_present() {
        let mut grid = SmudgeGrid::new(8, 8);
        let registry = make_registry_with_one_crater_1x1();
        let terrain = make_terrain(8, 8, true);
        let mut overlay = OverlayGrid::new(8, 8);
        overlay.place_overlay(4, 4, 0, 0);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        assert!(!grid.try_place(
            SmudgeKind::Crater, coord, 30, 30, false,
            &registry, &terrain, &overlay, &occupancy, &mut rng,
        ));
    }

    #[test]
    fn threshold_strict_less_than_for_size_filter() {
        // Registry: one 1x1 crater + one 2x2 crater. With dmg=60, dmg2=50 (strict < fails),
        // only the 1x1 should be selectable.
        let ini = crate::rules::ini_parser::IniFile::from_bytes(
            b"[SmudgeTypes]\n1=CR1\n2=CR2\n\
              [CR1]\nCrater=yes\nWidth=1\nHeight=1\n\
              [CR2]\nCrater=yes\nWidth=2\nHeight=2\n"
        ).unwrap();
        let registry = SmudgeTypeRegistry::from_rules_ini(&ini);
        let mut grid = SmudgeGrid::new(8, 8);
        let terrain = make_terrain(8, 8, true);
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        // Run try_place 50 times; with dmg=60, dmg2=50 only CR1 (1x1) should be picked.
        // Verify no 2x2 footprints land (CR2 would write 4 cells; CR1 writes 1).
        for _ in 0..50 {
            grid = SmudgeGrid::new(8, 8); // reset
            grid.try_place(
                SmudgeKind::Crater, coord, 60, 50, false,
                &registry, &terrain, &overlay, &occupancy, &mut rng,
            );
            // Count occupied cells; must be 0 or 1, never 4.
            let occupied = grid.iter_occupied().count();
            assert!(occupied <= 1, "1x1 only; saw {} cells", occupied);
        }
    }

    #[test]
    fn empty_filter_falls_back_to_unfiltered() {
        // Registry has only a 2x2 crater; with force_big=false and dmg below threshold,
        // size filter eliminates it but fallback to unfiltered should still pick it.
        let ini = crate::rules::ini_parser::IniFile::from_bytes(
            b"[SmudgeTypes]\n1=CR2\n[CR2]\nCrater=yes\nWidth=2\nHeight=2\n"
        ).unwrap();
        let registry = SmudgeTypeRegistry::from_rules_ini(&ini);
        let mut grid = SmudgeGrid::new(8, 8);
        let terrain = make_terrain(8, 8, true);
        let overlay = OverlayGrid::new(8, 8);
        let occupancy = OccupancyGrid::new();
        let mut rng = SimRng::new(1);
        let coord = SimCoord { x: 4 * 256 + 128, y: 4 * 256 + 128, z: 0 };
        assert!(grid.try_place(
            SmudgeKind::Crater, coord, 30, 30, false,
            &registry, &terrain, &overlay, &occupancy, &mut rng,
        ));
        // 2x2 footprint placed at (4,4): 4 cells written.
        assert_eq!(grid.iter_occupied().count(), 4);
    }
}
