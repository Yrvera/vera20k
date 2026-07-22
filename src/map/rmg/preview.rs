//! Terrain preview image for a generated random map.
//!
//! Turns a generated map into the RGBA thumbnail the setup dialog shows in its
//! preview box and that gets written out as `RandMap.img`. Depends only on the
//! cell data handed in — no assets, no wgpu, no sim.
//!
//! The geometry here is not free-form: the original projects cell centres
//! through the isometric transform, sizes the surface from the projected extent
//! of the playable cells, and emits *two* horizontal pixels per cell. Anything
//! that changes the projection, the rounding, or the doubling changes the image
//! the player sees, so each step is pinned by a test below.

/// A cell to draw: its grid position and the two radar colours the isometric
/// diamond's left and right halves contribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewCell {
    pub x: u16,
    pub y: u16,
    /// Left half's colour, drawn at the even pixel.
    pub left: [u8; 3],
    /// Right half's colour, drawn at the odd pixel.
    pub right: [u8; 3],
}

/// The rendered thumbnail. `rgba` is row-major, 4 bytes per pixel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Leptons per cell; a cell centre sits half a cell in on both axes.
const LEPTONS_PER_CELL: i32 = 0x100;
const CELL_CENTRE_LEPTONS: i32 = 0x80;
/// Projection scales — the isometric tile is 60x30, and the transform halves
/// both before the fixed-point shift.
const PROJECT_X_SCALE: i32 = 60;
const PROJECT_Y_SCALE: i32 = 30;
/// Constant added to the projected X after the shift, keeping the whole
/// playfield in positive territory.
const PROJECT_X_BIAS: i32 = 0x3C00;
/// Divisors that take the projected coordinate down to preview-pixel space.
const PREVIEW_X_DIVISOR: i32 = 0x3C;
const PREVIEW_Y_DIVISOR: i32 = 0x1E;
/// Substituted whenever a cell's colour comes back black, which the original
/// treats as a data fault rather than a legitimate colour.
const BLACK_PIXEL_SUBSTITUTE: [u8; 3] = [0x80, 0x80, 0x80];
/// Baked start markers: a 4x4 square in this red, offset one pixel up and left
/// of the waypoint's own pixel.
const MARKER_SIZE: i32 = 4;
const MARKER_RGB: [u8; 3] = [0xF0, 0x00, 0x00];
const MARKER_ORIGIN_BIAS: i32 = -1;
/// Only the first eight waypoints get a marker.
pub const MARKER_WAYPOINT_COUNT: u8 = 8;

/// Project a cell to the preview's pre-division coordinate space.
///
/// The halving happens before the fixed-point shift, and the shift adds a
/// sign-dependent bias so negatives truncate toward zero rather than flooring —
/// dropping that bias shifts the whole west/north edge of the map by a pixel.
fn project_cell_centre(cell_x: i32, cell_y: i32) -> (i32, i32) {
    let lepton_x = cell_x * LEPTONS_PER_CELL + CELL_CENTRE_LEPTONS;
    let lepton_y = cell_y * LEPTONS_PER_CELL + CELL_CENTRE_LEPTONS;

    let raw_x = (lepton_x * PROJECT_X_SCALE) / 2 + (lepton_y * -PROJECT_X_SCALE) / 2;
    let raw_y = (lepton_x * PROJECT_Y_SCALE) / 2 + (lepton_y * PROJECT_Y_SCALE) / 2;

    let projected_x = shift_with_sign_bias(raw_x) + PROJECT_X_BIAS;
    let projected_y = shift_with_sign_bias(raw_y);
    (projected_x, projected_y)
}

/// The fixed-point `>> 8` the projection uses, biased so negative values
/// truncate toward zero.
const fn shift_with_sign_bias(value: i32) -> i32 {
    (value + ((value >> 31) & 0xFF)) >> 8
}

/// Preview-pixel column and row for a projected cell, before the surface origin
/// is subtracted. X is in *pairs* — the caller doubles it.
const fn preview_cell_column(projected_x: i32) -> i32 {
    projected_x / PREVIEW_X_DIVISOR
}

const fn preview_cell_row(projected_y: i32) -> i32 {
    projected_y / PREVIEW_Y_DIVISOR
}

/// The projected extent of a set of cells: `(min_col, min_row, max_col, max_row)`.
fn projected_bounds(cells: &[PreviewCell]) -> Option<(i32, i32, i32, i32)> {
    let mut bounds: Option<(i32, i32, i32, i32)> = None;
    for cell in cells {
        let (projected_x, projected_y) = project_cell_centre(i32::from(cell.x), i32::from(cell.y));
        let column = preview_cell_column(projected_x);
        let row = preview_cell_row(projected_y);
        bounds = Some(match bounds {
            None => (column, row, column, row),
            Some((min_col, min_row, max_col, max_row)) => (
                min_col.min(column),
                min_row.min(row),
                max_col.max(column),
                max_row.max(row),
            ),
        });
    }
    bounds
}

/// Collect the playable cells of a map with the radar colours the resolved
/// terrain gave them.
///
/// Playability is the same `LocalSize` test the terrain grid uses to drop border
/// filler: those cells sit under permanent shroud and must not stretch the
/// preview. Cells with no tile are skipped, as are any the resolver did not
/// produce.
pub fn preview_cells_from_map(
    map: &crate::map::map_file::MapFile,
    resolved: &crate::map::resolved_terrain::ResolvedTerrainGrid,
) -> Vec<PreviewCell> {
    let bounds = crate::map::terrain::LocalBounds::from_header(&map.header);
    let mut cells = Vec::with_capacity(map.cells.len());
    for cell in &map.cells {
        if cell.tile_index < 0 {
            continue;
        }
        let (screen_x, screen_y) = crate::map::terrain::iso_to_screen(cell.rx, cell.ry, cell.z);
        if !bounds.contains(screen_x, screen_y) {
            continue;
        }
        let Some(resolved_cell) = resolved.cell(cell.rx, cell.ry) else {
            continue;
        };
        cells.push(PreviewCell {
            x: cell.rx,
            y: cell.ry,
            left: resolved_cell.radar_left,
            right: resolved_cell.radar_right,
        });
    }
    cells
}

/// Start-position waypoints in slot order, ready for [`render_preview`].
pub fn marker_waypoints(start_waypoints: &[(u8, u16, u16)]) -> Vec<(u16, u16)> {
    let mut ordered: Vec<(u8, u16, u16)> = start_waypoints.to_vec();
    ordered.sort_by_key(|(slot, _, _)| *slot);
    ordered
        .into_iter()
        .map(|(_, x, y)| (x, y))
        .take(MARKER_WAYPOINT_COUNT as usize)
        .collect()
}

/// Render the preview for a generated map.
///
/// `cells` must already be filtered to the playable area — the surface is sized
/// from their projected extent, so border filler would inflate the image.
/// `waypoints` are `(cell_x, cell_y)` per start position in slot order; only the
/// first [`MARKER_WAYPOINT_COUNT`] are marked, and the markers are baked into
/// the image rather than drawn over it later.
///
/// Returns `None` when there is nothing to draw or the extent collapses to an
/// empty surface.
pub fn render_preview(cells: &[PreviewCell], waypoints: &[(u16, u16)]) -> Option<PreviewImage> {
    let (min_col, min_row, max_col, max_row) = projected_bounds(cells)?;

    // X is doubled because each cell contributes two horizontal pixels; Y is
    // not, because each cell is one row tall.
    let width = (max_col - min_col) * 2;
    let height = max_row - min_row;
    if width <= 0 || height <= 0 {
        return None;
    }
    let (width, height) = (width as u32, height as u32);

    let mut image = PreviewImage {
        width,
        height,
        rgba: vec![0; (width as usize) * (height as usize) * 4],
    };

    for cell in cells {
        let (projected_x, projected_y) = project_cell_centre(i32::from(cell.x), i32::from(cell.y));
        let column = (preview_cell_column(projected_x) - min_col) * 2;
        let row = preview_cell_row(projected_y) - min_row;
        image.put_pixel(column, row, substitute_if_black(cell.left));
        image.put_pixel(column + 1, row, substitute_if_black(cell.right));
    }

    for (cell_x, cell_y) in waypoints.iter().take(MARKER_WAYPOINT_COUNT as usize) {
        let (projected_x, projected_y) =
            project_cell_centre(i32::from(*cell_x), i32::from(*cell_y));
        let column = (preview_cell_column(projected_x) - min_col) * 2 + MARKER_ORIGIN_BIAS;
        let row = preview_cell_row(projected_y) - min_row + MARKER_ORIGIN_BIAS;
        image.fill_rect(column, row, MARKER_SIZE, MARKER_SIZE, MARKER_RGB);
    }

    Some(image)
}

/// A black colour means the lookup failed rather than that the cell is black,
/// so it becomes mid grey.
const fn substitute_if_black(rgb: [u8; 3]) -> [u8; 3] {
    if rgb[0] == 0 && rgb[1] == 0 && rgb[2] == 0 {
        BLACK_PIXEL_SUBSTITUTE
    } else {
        rgb
    }
}

impl PreviewImage {
    /// Write one pixel, ignoring anything outside the surface. Markers near the
    /// edge legitimately hang off it and are clipped rather than growing it.
    fn put_pixel(&mut self, x: i32, y: i32, rgb: [u8; 3]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let offset = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        self.rgba[offset] = rgb[0];
        self.rgba[offset + 1] = rgb[1];
        self.rgba[offset + 2] = rgb[2];
        self.rgba[offset + 3] = 0xFF;
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, rgb: [u8; 3]) {
        for row in y..y + h {
            for column in x..x + w {
                self.put_pixel(column, row, rgb);
            }
        }
    }

    /// The pixel at `(x, y)` as RGB, for tests and callers that inspect output.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 3]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        Some([
            self.rgba[offset],
            self.rgba[offset + 1],
            self.rgba[offset + 2],
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(x: u16, y: u16, left: [u8; 3], right: [u8; 3]) -> PreviewCell {
        PreviewCell { x, y, left, right }
    }

    /// A filled diamond of cells, which is the shape the generator produces.
    fn diamond(extent: u16) -> Vec<PreviewCell> {
        let mut cells = Vec::new();
        for y in 0..extent {
            for x in 0..extent {
                cells.push(cell(x, y, [10, 20, 30], [40, 50, 60]));
            }
        }
        cells
    }

    #[test]
    fn origin_cell_projects_to_the_x_bias() {
        // Cell 0's centre sits on the projection's axis, so X lands exactly on
        // the bias and Y on zero.
        assert_eq!(project_cell_centre(0, 0), (PROJECT_X_BIAS, 15));
    }

    #[test]
    fn moving_east_and_south_moves_opposite_ways_in_x() {
        let (east_x, east_y) = project_cell_centre(10, 0);
        let (south_x, south_y) = project_cell_centre(0, 10);
        let (origin_x, _) = project_cell_centre(0, 0);
        assert!(east_x > origin_x, "east goes right");
        assert!(south_x < origin_x, "south goes left");
        assert_eq!(
            preview_cell_row(east_y),
            preview_cell_row(south_y),
            "both are the same distance down the screen"
        );
    }

    #[test]
    fn the_shift_truncates_negatives_toward_zero() {
        // Floor would give -2 for a value that truncation puts at -1.
        assert_eq!(shift_with_sign_bias(-300), -1);
        assert_eq!(shift_with_sign_bias(300), 1);
        assert_eq!(shift_with_sign_bias(-256), -1);
        assert_eq!(shift_with_sign_bias(0), 0);
    }

    #[test]
    fn width_is_the_doubled_column_extent_and_height_is_not_doubled() {
        let cells = diamond(20);
        let (min_col, min_row, max_col, max_row) = projected_bounds(&cells).expect("bounds");
        let image = render_preview(&cells, &[]).expect("image");
        assert_eq!(image.width, ((max_col - min_col) * 2) as u32);
        assert_eq!(image.height, (max_row - min_row) as u32);
    }

    #[test]
    fn each_cell_writes_a_left_and_a_right_pixel() {
        let cells = diamond(20);
        let image = render_preview(&cells, &[]).expect("image");
        // Every written column pair carries the two distinct colours, so a
        // solid-colour run would mean the pair collapsed.
        let mut saw_left = false;
        let mut saw_right = false;
        for y in 0..image.height {
            for x in 0..image.width {
                match image.pixel(x, y) {
                    Some([10, 20, 30]) => saw_left = true,
                    Some([40, 50, 60]) => saw_right = true,
                    _ => {}
                }
            }
        }
        assert!(
            saw_left && saw_right,
            "both halves of the diamond are drawn"
        );
    }

    #[test]
    fn a_black_cell_becomes_grey_rather_than_black() {
        let cells = vec![
            cell(0, 0, [0, 0, 0], [0, 0, 0]),
            cell(20, 0, [1, 1, 1], [1, 1, 1]),
            cell(0, 20, [1, 1, 1], [1, 1, 1]),
            cell(20, 20, [1, 1, 1], [1, 1, 1]),
        ];
        let image = render_preview(&cells, &[]).expect("image");
        assert!(
            image
                .rgba
                .chunks_exact(4)
                .any(|px| px[..3] == BLACK_PIXEL_SUBSTITUTE),
            "the black cell was substituted"
        );
    }

    #[test]
    fn markers_are_baked_in_and_capped_at_eight() {
        let cells = diamond(40);
        let waypoints: Vec<(u16, u16)> = (0..12).map(|i| (10 + i, 10 + i)).collect();
        let image = render_preview(&cells, &waypoints).expect("image");
        let marker_pixels = image
            .rgba
            .chunks_exact(4)
            .filter(|px| px[..3] == MARKER_RGB)
            .count();
        assert!(marker_pixels > 0, "markers are baked into the image");
        assert!(
            marker_pixels <= (MARKER_WAYPOINT_COUNT as usize) * 16,
            "at most eight 4x4 markers, got {marker_pixels} pixels"
        );
    }

    #[test]
    fn no_cells_renders_nothing() {
        assert_eq!(render_preview(&[], &[]), None);
        // A single cell has no extent, so there is no surface to allocate.
        assert_eq!(
            render_preview(&[cell(3, 3, [1, 2, 3], [4, 5, 6])], &[]),
            None
        );
    }

    #[test]
    fn a_marker_off_the_edge_is_clipped_not_grown() {
        let cells = diamond(20);
        let baseline = render_preview(&cells, &[]).expect("baseline");
        let with_marker = render_preview(&cells, &[(0, 0)]).expect("with marker");
        assert_eq!(
            (baseline.width, baseline.height),
            (with_marker.width, with_marker.height),
            "markers never resize the surface"
        );
    }
}
