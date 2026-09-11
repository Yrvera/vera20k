//! Live host for the native high-rim selector and restart loop.
//!
//! The original untagged575EE0 traversal remains mandatory even on a no-write
//! refresh, because its GetCell calls can move the retained shared dummy.

use super::*;
use crate::map::bridge_rim_tiles::HighBridgeRimTiles;
use crate::sim::bridge_state::rim::{self, HighBridgeRimHost, RimCell, RimCoord};

pub(super) fn update(publication: &mut LivePublication<'_>, input: RimCoord) {
    let Some(tiles) = publication.terrain().high_bridge_rim_tiles() else {
        // Synthetic grids without an active theater have no native tile keys.
        return;
    };
    let size = publication
        .sim
        .playfield_bounds
        .zip(publication.sim.playfield_size_height)
        .map(|(bounds, height)| (bounds.base, height))
        .or_else(|| {
            publication
                .sim
                .bridge_state
                .as_ref()?
                .native_zone_source_size()
        });
    let Some(size) = size else { return };
    rim::update_adjacent(
        &mut LiveRim {
            publication,
            tiles,
            size,
        },
        input,
    );
}

struct LiveRim<'a, 'world> {
    publication: &'a mut LivePublication<'world>,
    tiles: HighBridgeRimTiles,
    size: (i32, i32),
}

impl HighBridgeRimHost for LiveRim<'_, '_> {
    type Cell = Cell;

    fn tiles(&self) -> HighBridgeRimTiles {
        self.tiles
    }
    fn map_size(&self) -> (i32, i32) {
        self.size
    }
    fn cell(&mut self, coord: RimCoord) -> Cell {
        self.publication.lookup(coord)
    }
    fn read(&self, cell: Cell) -> RimCell {
        let terrain = self.publication.terrain();
        let (tile, subtile) = match cell {
            Cell::Real(index) => {
                let resolved = &terrain.cells()[index];
                (resolved.final_tile_index, resolved.final_sub_tile)
            }
            // Constructor47BBF0 supplies +38=FFFF/+11A=0. Live dummy tile
            // replacement is not modeled; no general malformed-map parity is
            // claimed by the stock corpus. Do not substitute a real cell.
            Cell::Dummy => (0xffff, 0),
        };
        RimCell {
            coord: self.publication.coord(cell),
            flags: self.publication.flags(cell),
            tile,
            subtile,
            anchor: terrain
                .native_cell_anchor(cell)
                .map(|anchor| terrain.native_cell_coord(anchor)),
        }
    }
    fn allocated(&self, coord: RimCoord) -> bool {
        self.publication
            .terrain()
            .native_fixed_cell_index(coord.0, coord.1)
            .is_some()
    }
    fn clear_group(&mut self, cell: Cell, direction: u8) {
        publication::set_bridge_direction(self.publication, cell, direction, false);
    }
    fn clear_overlay_and_mark_radar(&mut self, cell: Cell) {
        self.publication.write_state(cell, 0);
        self.publication.clear_overlay(cell);
        self.publication.radar(cell);
    }
    fn notify_span(&mut self, first: RimCoord, end: RimCoord) {
        rim::notify_span_cells(self, first, end);
    }
    fn notify_cell(&mut self, _cell: Cell) {
        // TagClass event31 dispatch requires the live trigger authority
        // (Phase10 rows209/222-224). Keep that behavior explicitly open.
    }
    // Native rectangles schedule partial tactical redraws. VERA clears color
    // and depth and rebuilds/uploads all three bridge batches every frame
    // (render/build_instances.rs, render/bridges.rs); each reads deck_present.
    // This no-tile-write mechanism needs no extra retained redraw authority.
    // Literal56EB80 delivery must also migrate the immutable terrain-template
    // and presentation-grid consumers; that separate gap remains open.
    fn mark_span(&mut self, _start: RimCoord, _end: RimCoord) {}
    fn reset_span_marks(&mut self) {}
    fn mark_screen(&mut self) {}
}
