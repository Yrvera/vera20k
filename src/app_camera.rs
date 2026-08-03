//! Camera positioning — keyboard scroll, mouse edge scroll, zoom, and clamping.
//!
//! Extracted from app_sim_tick.rs to separate camera control from sim advancement.
//!
//! ## Coordinate frames
//! Three frames meet in this file and mixing them is the classic bug here:
//! * **World pixels** — what `terrain::iso_to_screen` / `terrain::lepton_to_screen`
//!   produce. `camera_x`/`camera_y` live in this frame and name the world point
//!   drawn at window pixel `(0, 0)`.
//! * **Window pixels** — `render_width()` × `render_height()`, cursor position,
//!   sidebar width. The batch shader maps `screen = (world - camera) * zoom`, so
//!   converting a window-pixel extent into world pixels is a divide by `zoom`.
//! * **Tactical-viewport pixels** — window pixels minus the sidebar column on the
//!   right. gamemd anchors the camera on the centre of *this* rect, not the
//!   window's, and its scroll clamp carries the same `viewport/2` terms.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::map::terrain;

/// Camera scroll speed in pixels per frame (arrow keys).
const CAMERA_SCROLL_SPEED: f32 = 8.0;

/// Minimum zoom level — zoomed out enough to see a large portion of the map.
const MIN_ZOOM: f32 = 0.25;
/// Maximum zoom level — zoomed in close to pixel-level detail.
const MAX_ZOOM: f32 = 4.0;
/// Multiplicative zoom step per mouse wheel notch (smooth exponential zoom).
const ZOOM_STEP: f32 = 1.15;

// ---------------------------------------------------------------------------
// Edge auto-scroll — gamemd's CoastLevel ramp.
// ---------------------------------------------------------------------------
//
// Verified by live decompilation of the edge-scroll entry point this session.
// The mechanism is: a one-pixel at-edge band around the whole window; a
// nine-zone reference point that turns "which edge am I pushing" into a
// direction; a coast counter that ramps once per 16 ms while the cursor stays
// at the edge and decays at the same cadence once it leaves; and a speed table
// indexed by `8 - coast`, capped by the player's `[Options] ScrollRate`.

/// Raw per-frame scroll distances before the rules multiplier, indexed by
/// `8 - CoastLevel`. Read verbatim out of the running image this session.
///
/// Index 0 is unreachable in a stock game: the cap below forces the index to at
/// least `ScrollRate + 1`, and the options slider never reports `ScrollRate` below 0.
const EDGE_SCROLL_SPEED_TABLE: [i32; 9] = [448, 384, 320, 256, 192, 128, 64, 32, 16];

/// `[AudioVisual] ScrollMultiplier`, stock `.07` in both `rules.ini` and
/// `rulesmd.ini`. Scales the raw table into pixels per frame, giving the retail
/// ramp `1, 2, 4, 8, 13, 17, 22, 26` px/frame for coast levels 0..7.
///
/// VERA-internal: `GeneralRules` has no `ScrollMultiplier` field yet, so the
/// stock value is inlined. Wire this to the rules parser when that key lands.
const SCROLL_MULTIPLIER: f64 = 0.07;

/// Fraction of the window width that splits the nine-zone direction bands on X.
const EDGE_SCROLL_ZONE_FRACTION_X: f64 = 0.16;
/// Fraction of the window height that splits the nine-zone direction bands on Y.
const EDGE_SCROLL_ZONE_FRACTION_Y: f64 = 0.21;

/// gamemd's radar timer counts `timeGetTime() >> 4`, so one unit is 16 ms.
const RADAR_TIMER_MS_SHIFT: u32 = 4;
/// CoastLevel moves by one step per radar-timer tick, up or down.
const COAST_STEP_TICKS: u32 = 1;

/// Radians to 16-bit direction units. Negative because screen Y grows downward
/// while the direction word increases clockwise from north.
const DIR16_PER_RADIAN: f64 = -(32768.0 / std::f64::consts::PI);

/// Camera step per octant in window pixels, ordered N, NE, E, SE, S, SW, W, NW —
/// the same eight `(dx, dy)` pairs gamemd's scroll routine indexes.
///
/// Diagonals move the full distance on **both** axes; they are not normalised.
const OCTANT_DELTA: [(f32, f32); 8] = [
    (0.0, -1.0),
    (1.0, -1.0),
    (1.0, 0.0),
    (1.0, 1.0),
    (0.0, 1.0),
    (-1.0, 1.0),
    (-1.0, 0.0),
    (-1.0, -1.0),
];

/// Live edge-scroll ramp state. One instance per `AppState`.
#[derive(Debug, Clone)]
pub(crate) struct EdgeScrollState {
    /// Wall-clock origin for the radar timer. Edge scroll is driven by the real
    /// cursor, so it is presentation state and never enters the sim.
    epoch: std::time::Instant,
    /// gamemd's CoastLevel: 0 is the slowest creep, higher is faster.
    coast_level: i32,
    /// Last direction octant. Retained so the map keeps coasting in the same
    /// direction while the counter decays after the cursor leaves the edge.
    octant: usize,
    /// Radar-timer stamp of the last coast change. `None` matches gamemd's
    /// zero-delay start, where the very first step is allowed immediately.
    last_coast_change: Option<u32>,
}

impl Default for EdgeScrollState {
    fn default() -> Self {
        Self {
            epoch: std::time::Instant::now(),
            coast_level: 0,
            octant: 0,
            last_coast_change: None,
        }
    }
}

impl EdgeScrollState {
    /// Current radar-timer reading (16 ms units).
    pub(crate) fn radar_timer(&self) -> u32 {
        (self.epoch.elapsed().as_millis() as u32) >> RADAR_TIMER_MS_SHIFT
    }

    fn timer_expired(&self, now: u32) -> bool {
        match self.last_coast_change {
            None => true,
            Some(stamp) => now.wrapping_sub(stamp) >= COAST_STEP_TICKS,
        }
    }

    fn ramp_up(&mut self, now: u32) {
        if self.timer_expired(now) {
            self.last_coast_change = Some(now);
            self.coast_level += 1;
        }
    }

    fn decay(&mut self, now: u32) {
        if self.timer_expired(now) {
            self.last_coast_change = Some(now);
            self.coast_level = (self.coast_level - 1).max(0);
        }
    }
}

/// Nine-zone reference coordinate for one axis.
///
/// Below `fraction` of the axis the reference is the near edge, above
/// `1 - fraction` it is the far edge, and in between it is the axis midpoint.
/// The midpoint divide is gamemd's `size / 2` truncating toward zero.
fn zone_reference(value: i32, size: i32, fraction: f64) -> i32 {
    let value = value as f64;
    let size_f = size as f64;
    if value < size_f * fraction {
        0
    } else if value <= (1.0 - fraction) * size_f {
        size / 2
    } else {
        size - 1
    }
}

/// Turn an at-edge cursor position into one of eight scroll directions.
///
/// gamemd takes `atan2` from the window centre toward the nine-zone reference
/// point, converts the angle to a 16-bit direction word, rounds that to the
/// 8-bit direction byte, then rounds again to an octant. Only nine reference
/// points exist, so the result is a small fixed table in practice; the angle
/// arithmetic is reproduced rather than tabulated because the zone split
/// depends on the window's aspect ratio.
fn edge_scroll_octant(x: i32, y: i32, view_w: i32, view_h: i32) -> usize {
    let ref_x = zone_reference(x, view_w, EDGE_SCROLL_ZONE_FRACTION_X);
    let ref_y = zone_reference(y, view_h, EDGE_SCROLL_ZONE_FRACTION_Y);
    let centre_x = view_w / 2;
    let centre_y = view_h / 2;

    let angle = f64::from(centre_y - ref_y).atan2(f64::from(ref_x - centre_x));
    // gamemd's float-to-long helper runs with the x87 rounding mode set to
    // chop, so every conversion here truncates toward zero.
    let dir16 = ((angle - std::f64::consts::FRAC_PI_2) * DIR16_PER_RADIAN).trunc() as i32 as u16;
    // 16-bit direction word to direction byte, then byte to octant. Both are the
    // native "add half a step, then shift" rounding, and both wrap to north.
    let dir8 = (((dir16 >> 7) + 1) >> 1) as u8;
    ((usize::from(dir8 >> 4) + 1) >> 1) & 7
}

/// One edge-scroll step. Returns the camera delta in **window pixels**.
///
/// `view_w`/`view_h` are the full window, sidebar included: gamemd's at-edge
/// test adds the sidebar surface width to the composition surface width, so the
/// east band sits at the far right of the screen, past the sidebar — hovering
/// just left of the sidebar does *not* scroll.
///
/// `scroll_rate` is the internal `[Options] ScrollRate` (0 fastest .. 6 slowest).
///
/// Two verified native behaviours are **not** reproduced here, both recorded as
/// open DRIFT:
/// * **Blocked-scroll ramp gate.** gamemd asks whether the move is possible
///   before scrolling. When it is not — the camera is already pinned at a map
///   border — it swaps in the barred cursor and skips the whole timer block, so
///   the counter does not charge. VERA ramps unconditionally, so holding the
///   cursor against a map edge winds the counter to the cap and the next scroll
///   in a different direction starts at full speed instead of creeping. Fires
///   whenever a player pushes into a map border, which is routine.
/// * **Right-button slowdown.** With the right button held, gamemd bumps the
///   table index one step slower and clamps it into `4..=8`, capping edge scroll
///   at 13 px/frame. VERA has no persistent right-button-held flag to read.
pub(crate) fn edge_scroll_step(
    state: &mut EdgeScrollState,
    cursor: (f32, f32),
    view_w: i32,
    view_h: i32,
    scroll_rate: u32,
    now: u32,
) -> (f32, f32) {
    // The native test is on integer mouse coordinates; VERA's cursor is scaled
    // into render space as a float, so floor before comparing.
    let x = cursor.0.floor() as i32;
    let y = cursor.1.floor() as i32;
    let at_edge = x <= 0 || y <= 0 || x >= view_w - 1 || y >= view_h - 1;

    if at_edge {
        state.octant = edge_scroll_octant(x, y, view_w, view_h);
    } else if state.coast_level == 0 {
        // Idle: nothing moves, but the decay timer is still serviced.
        state.decay(now);
        return (0.0, 0.0);
    }

    // ScrollRate caps the peak speed by flooring the table index, and the coast
    // counter is written back clamped so it cannot run away above the cap.
    // The `clamp` only guards the array bound — the options slider is 0..=6, so
    // the index never exceeds 7 in a stock game.
    let index = ((8 - state.coast_level).max(scroll_rate as i32 + 1))
        .clamp(0, EDGE_SCROLL_SPEED_TABLE.len() as i32 - 1);
    state.coast_level = 8 - index;

    // Truncating float-to-long, matching gamemd's chop rounding mode: the stock
    // multiplier turns the table into 1, 2, 4, 8, 13, 17, 22, 26, 31 px/frame.
    let distance =
        (f64::from(EDGE_SCROLL_SPEED_TABLE[index as usize]) * SCROLL_MULTIPLIER).trunc() as f32;

    if at_edge {
        state.ramp_up(now);
    } else {
        state.decay(now);
    }

    let (dx, dy) = OCTANT_DELTA[state.octant];
    (dx * distance, dy * distance)
}

/// Update camera position based on keyboard and mouse edge scrolling.
pub(crate) fn update_camera(state: &mut AppState) {
    let sw: f32 = state.render_width() as f32;
    let sh: f32 = state.render_height() as f32;

    if state
        .keys_held
        .contains(&winit::keyboard::KeyCode::ArrowLeft)
    {
        state.camera_x -= CAMERA_SCROLL_SPEED / state.zoom_level;
    }
    if state
        .keys_held
        .contains(&winit::keyboard::KeyCode::ArrowRight)
    {
        state.camera_x += CAMERA_SCROLL_SPEED / state.zoom_level;
    }
    if state.keys_held.contains(&winit::keyboard::KeyCode::ArrowUp) {
        state.camera_y -= CAMERA_SCROLL_SPEED / state.zoom_level;
    }
    if state
        .keys_held
        .contains(&winit::keyboard::KeyCode::ArrowDown)
    {
        state.camera_y += CAMERA_SCROLL_SPEED / state.zoom_level;
    }

    // gamemd gates edge scroll on a ScrollInhibited flag; VERA's only inhibit
    // source today is an in-progress minimap drag.
    if !state.minimap_dragging {
        let now = state.edge_scroll.radar_timer();
        let scroll_rate = state.in_game_options.scroll_rate;
        let (dx, dy) = edge_scroll_step(
            &mut state.edge_scroll,
            (state.cursor_x, state.cursor_y),
            sw as i32,
            sh as i32,
            scroll_rate,
            now,
        );
        // The speed table is in window pixels. Stock YR has no world zoom, so
        // the divide is VERA-internal: it keeps the on-screen scroll rate
        // constant across VERA's zoom range and is exact at zoom 1.0.
        state.camera_x += dx / state.zoom_level;
        state.camera_y += dy / state.zoom_level;
    }

    clamp_camera_to_playable_area(state, sw, sh);

    // Smoothly animate zoom_level toward zoom_target each frame.
    animate_zoom(state);
}

/// Smoothing factor for zoom animation. Each frame, zoom_level moves this
/// fraction of the remaining distance toward zoom_target. 0.35 ≈ snappy ease-out.
const ZOOM_EASE: f32 = 0.35;
/// Snap threshold — when zoom_level is this close to zoom_target, jump to it.
const ZOOM_SNAP: f32 = 0.002;

/// Set zoom target from mouse wheel input, anchored on the cursor position.
///
/// Records the world point under the cursor so `animate_zoom` can keep it
/// pinned at that screen position during the smooth ease.
pub(crate) fn apply_zoom(state: &mut AppState, delta_lines: f32) {
    let old_target = state.zoom_target;
    let factor = ZOOM_STEP.powf(delta_lines);
    let new_target = (old_target * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    if (new_target - old_target).abs() < 1e-6 {
        return;
    }

    // Record the world point under the cursor — animate_zoom keeps it stable.
    let z = state.zoom_level;
    state.zoom_anchor_world = [
        state.cursor_x / z + state.camera_x,
        state.cursor_y / z + state.camera_y,
    ];
    state.zoom_anchor_screen = [state.cursor_x, state.cursor_y];
    state.zoom_target = new_target;
}

/// Animate zoom_level toward zoom_target each frame, adjusting the camera so
/// the anchor world point stays at the anchor screen position.
pub(crate) fn animate_zoom(state: &mut AppState) {
    let diff = state.zoom_target - state.zoom_level;
    if diff.abs() < ZOOM_SNAP {
        if (state.zoom_level - state.zoom_target).abs() > 1e-7 {
            state.zoom_level = state.zoom_target;
            let sw = state.render_width() as f32;
            let sh = state.render_height() as f32;
            clamp_camera_to_playable_area(state, sw, sh);
        }
        return;
    }

    state.zoom_level += diff * ZOOM_EASE;

    // Adjust camera so the anchor world point stays at the anchor screen position:
    //   anchor_world_x = anchor_screen_x / zoom + camera_x
    //   camera_x = anchor_world_x - anchor_screen_x / zoom
    state.camera_x = state.zoom_anchor_world[0] - state.zoom_anchor_screen[0] / state.zoom_level;
    state.camera_y = state.zoom_anchor_world[1] - state.zoom_anchor_screen[1] / state.zoom_level;

    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    clamp_camera_to_playable_area(state, sw, sh);
}

/// Camera top-left, in world pixels, that puts `world` at the centre of the
/// **tactical** viewport rather than the centre of the window.
///
/// `world` is in world pixels; `window_w`, `window_h` and `sidebar_w` are window
/// pixels, so the half-extents are divided by `zoom` to come back to world
/// pixels. Only X loses width: the sidebar is a full-height column on the right,
/// which is why gamemd's own clamp carries `viewport_width / 2` on X but the
/// full height on Y.
pub(crate) fn tactical_camera_top_left(
    world: (f32, f32),
    window_w: f32,
    window_h: f32,
    sidebar_w: f32,
    zoom: f32,
) -> (f32, f32) {
    let tactical_w = (window_w - sidebar_w).max(1.0);
    (
        world.0 - tactical_w / (2.0 * zoom),
        world.1 - window_h / (2.0 * zoom),
    )
}

/// World-pixel position of a cell's **projected cell coordinate** — the point a
/// "go here" camera move should land on, and where an entity standing on that
/// cell is drawn.
///
/// gamemd's camera-set builds the cell-centre lepton coordinate `(cell << 8) +
/// 0x80` on both axes and projects it. VERA's reproduction of that same point is
/// `util::lepton::lepton_to_screen` at the cell centre, which equals
/// `iso_to_screen + (30, 0)`: X shifts by half a tile because `iso_to_screen`
/// anchors the NW corner of the tile's diamond bounding box, while Y already
/// carries the `+15` from the cell-centre projection.
///
/// There is deliberately **no** `+TILE_HEIGHT/2` here. That would be the centre
/// of the tile diamond, which is not the projected cell coordinate and is 15 px
/// below where the entity path puts a unit.
pub(crate) fn cell_centre_world_point(rx: u16, ry: u16, z: u8) -> (f32, f32) {
    let (nw_x, nw_y) = terrain::iso_to_screen(rx, ry, z);
    (nw_x + terrain::TILE_WIDTH / 2.0, nw_y)
}

pub(crate) fn center_camera_on_cell(state: &mut AppState, rx: u16, ry: u16) {
    let z = state.height_map.get(&(rx, ry)).copied().unwrap_or(0);
    let world = cell_centre_world_point(rx, ry, z);
    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    let (cx, cy) = tactical_camera_top_left(
        world,
        sw,
        sh,
        state.sidebar_layout_spec.sidebar_width,
        state.zoom_level,
    );
    state.camera_x = cx;
    state.camera_y = cy;
    clamp_camera_to_playable_area(state, sw, sh);
}

pub(crate) fn clamp_camera_to_playable_area(state: &mut AppState, sw: f32, sh: f32) {
    let Some(grid) = &state.terrain_grid else {
        return;
    };
    let (area_x, area_y, area_w, area_h) = match grid.local_bounds {
        Some(b) => (b.pixel_x, b.pixel_y, b.pixel_w, b.pixel_h),
        None => (
            grid.origin_x,
            grid.origin_y,
            grid.world_width,
            grid.world_height,
        ),
    };
    // Visible world area = screen pixels / zoom.
    let zoom = state.zoom_level;
    let clamp_axis = |origin: f32, world_size: f32, viewport: f32| -> (f32, f32) {
        let visible = viewport / zoom;
        if world_size <= visible {
            let center: f32 = origin + (world_size - visible) / 2.0;
            (center, center)
        } else {
            (origin, origin + world_size - visible)
        }
    };
    // Use game viewport width (excluding sidebar) for X clamping, not full window width.
    // The sidebar covers the right portion of the window and isn't part of the game view.
    let game_viewport_w: f32 = sw - state.sidebar_layout_spec.sidebar_width;
    let (cx_min, cx_max) = clamp_axis(area_x, area_w, game_viewport_w);
    let (cy_min, cy_max) = clamp_axis(area_y, area_h, sh);
    state.camera_x = state.camera_x.clamp(cx_min, cx_max);
    state.camera_y = state.camera_y.clamp(cy_min, cy_max);
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW_W: f32 = 1024.0;
    const WINDOW_H: f32 = 768.0;
    const SIDEBAR_W: f32 = 168.0;

    /// Screen position the batch shader gives a world point for a camera.
    fn screen_of(world: (f32, f32), camera: (f32, f32), zoom: f32) -> (f32, f32) {
        ((world.0 - camera.0) * zoom, (world.1 - camera.1) * zoom)
    }

    /// Where the production entity path draws a ground unit standing on a cell.
    /// `render::locomotor_visual::ground_screen_position` forwards straight to
    /// this, so it is the same projection a real unit gets.
    fn entity_world_point(rx: u16, ry: u16, z: u8) -> (f32, f32) {
        use crate::util::lepton::{CELL_CENTER_LEPTON, lepton_to_screen};
        lepton_to_screen(rx, ry, CELL_CENTER_LEPTON, CELL_CENTER_LEPTON, z)
    }

    #[test]
    fn centring_lands_a_unit_on_that_cell_at_the_tactical_centre() {
        // The invariant that matters: put a unit on a cell, centre on that cell,
        // and the unit must sit at the tactical viewport's centre. Anchoring the
        // camera anywhere else -- window centre, or the tile diamond's centre --
        // fails this.
        for (rx, ry, z) in [(10_u16, 10_u16, 0_u8), (0, 0, 0), (37, 12, 3), (63, 1, 2)] {
            let unit = entity_world_point(rx, ry, z);
            let camera = tactical_camera_top_left(
                cell_centre_world_point(rx, ry, z),
                WINDOW_W,
                WINDOW_H,
                SIDEBAR_W,
                1.0,
            );
            let (sx, sy) = screen_of(unit, camera, 1.0);
            assert!(
                (sx - (WINDOW_W - SIDEBAR_W) / 2.0).abs() < 1e-3,
                "cell ({rx},{ry},z={z}): sx {sx}"
            );
            assert!(
                (sy - WINDOW_H / 2.0).abs() < 1e-3,
                "cell ({rx},{ry},z={z}): sy {sy}"
            );
        }
    }

    #[test]
    fn centring_target_is_the_projected_cell_coordinate_not_the_tile_diamond_centre() {
        // Hand-walked fixture: cell (10, 10) at ground level.
        //   iso_to_screen           = (30*(10-10) - 30, 15*(10+10) + 15) = (-30, 315)
        //   projected cell coord    = iso_to_screen + (30, 0)            = (0, 315)
        // The tile diamond's centre would be (0, 330); that is 15 px below the
        // unit and is NOT what gamemd centres on.
        assert_eq!(cell_centre_world_point(10, 10, 0), (0.0, 315.0));
        assert_eq!(
            cell_centre_world_point(10, 10, 0),
            entity_world_point(10, 10, 0)
        );

        let camera = tactical_camera_top_left(
            cell_centre_world_point(10, 10, 0),
            WINDOW_W,
            WINDOW_H,
            SIDEBAR_W,
            1.0,
        );
        // Tactical rect is 1024 - 168 = 856 wide, so its centre is x = 428.
        assert_eq!(camera, (-428.0, -69.0));
        // Guard against the pre-fix behaviour, which anchored on the window
        // centre and put the target 84 px east of the tactical centre.
        assert_ne!(camera.0, 0.0 - WINDOW_W / 2.0);
    }

    #[test]
    fn centring_stays_on_the_tactical_centre_across_ui_scale_and_zoom() {
        // 0.5 and 1.5 are the only scales `auto_detect_ui_scale` produces, so
        // 84 and 252 are the real sidebar widths in the field; 168 is the
        // unscaled base.
        for zoom in [0.25_f32, 0.5, 1.0, 2.0, 4.0] {
            for sidebar in [0.0_f32, 84.0, 168.0, 252.0] {
                let unit = entity_world_point(37, 12, 3);
                let camera = tactical_camera_top_left(
                    cell_centre_world_point(37, 12, 3),
                    WINDOW_W,
                    WINDOW_H,
                    sidebar,
                    zoom,
                );
                let (sx, sy) = screen_of(unit, camera, zoom);
                assert!(
                    (sx - (WINDOW_W - sidebar) / 2.0).abs() < 1e-3,
                    "zoom {zoom} sidebar {sidebar}: sx {sx}"
                );
                assert!((sy - WINDOW_H / 2.0).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn cell_centre_shifts_only_in_x_from_the_tile_corner() {
        for (rx, ry, z) in [(0_u16, 0_u16, 0_u8), (5, 9, 2), (63, 1, 0)] {
            let (nw_x, nw_y) = terrain::iso_to_screen(rx, ry, z);
            let (cx, cy) = cell_centre_world_point(rx, ry, z);
            assert_eq!(cx - nw_x, terrain::TILE_WIDTH / 2.0);
            assert_eq!(cy - nw_y, 0.0);
        }
    }

    // -- edge scroll ---------------------------------------------------------

    const VIEW_W: i32 = 1024;
    const VIEW_H: i32 = 768;
    /// Stock `[Options] ScrollRate` default (0 fastest .. 6 slowest).
    const DEFAULT_SCROLL_RATE: u32 = 3;

    fn state() -> EdgeScrollState {
        EdgeScrollState::default()
    }

    #[test]
    fn nine_zone_direction_covers_all_eight_octants() {
        // 0.16 * 1024 = 163.84, 0.21 * 768 = 161.28.
        let left = 0;
        let right = VIEW_W - 1;
        let top = 0;
        let bottom = VIEW_H - 1;
        let mid_x = VIEW_W / 2;
        let mid_y = VIEW_H / 2;

        assert_eq!(edge_scroll_octant(mid_x, top, VIEW_W, VIEW_H), 0, "N");
        assert_eq!(edge_scroll_octant(right, top, VIEW_W, VIEW_H), 1, "NE");
        assert_eq!(edge_scroll_octant(right, mid_y, VIEW_W, VIEW_H), 2, "E");
        assert_eq!(edge_scroll_octant(right, bottom, VIEW_W, VIEW_H), 3, "SE");
        assert_eq!(edge_scroll_octant(mid_x, bottom, VIEW_W, VIEW_H), 4, "S");
        assert_eq!(edge_scroll_octant(left, bottom, VIEW_W, VIEW_H), 5, "SW");
        assert_eq!(edge_scroll_octant(left, mid_y, VIEW_W, VIEW_H), 6, "W");
        assert_eq!(edge_scroll_octant(left, top, VIEW_W, VIEW_H), 7, "NW");
    }

    #[test]
    fn touching_the_left_edge_high_up_scrolls_diagonally_not_straight_west() {
        // y = 100 is inside the top 21 % band, so the reference point is the
        // top-left corner and the scroll runs NW. Four independent axis tests
        // would report plain west here.
        assert_eq!(edge_scroll_octant(0, 100, VIEW_W, VIEW_H), 7);
        // y = 300 is in the middle band, so the same x gives plain west.
        assert_eq!(edge_scroll_octant(0, 300, VIEW_W, VIEW_H), 6);
    }

    #[test]
    fn edge_band_is_one_pixel_and_the_east_band_is_past_the_sidebar() {
        let mut st = state();
        // Ten pixels in from the left does nothing — the pre-fix 10 px band did.
        assert_eq!(
            edge_scroll_step(
                &mut st,
                (10.0, 300.0),
                VIEW_W,
                VIEW_H,
                DEFAULT_SCROLL_RATE,
                0
            ),
            (0.0, 0.0)
        );
        // Just left of the sidebar does nothing either.
        let sidebar_x = (VIEW_W as f32) - 168.0 - 1.0;
        assert_eq!(
            edge_scroll_step(
                &mut st,
                (sidebar_x, 300.0),
                VIEW_W,
                VIEW_H,
                DEFAULT_SCROLL_RATE,
                0
            ),
            (0.0, 0.0)
        );
        // The far right column of the window does scroll east.
        let (dx, dy) = edge_scroll_step(
            &mut st,
            ((VIEW_W - 1) as f32, 300.0),
            VIEW_W,
            VIEW_H,
            DEFAULT_SCROLL_RATE,
            0,
        );
        assert!(dx > 0.0, "dx {dx}");
        assert_eq!(dy, 0.0);
    }

    #[test]
    fn coast_ramps_geometrically_and_scroll_rate_caps_the_peak() {
        let mut st = state();
        // One step per radar tick; the cursor sits on the left edge throughout.
        let mut seen = Vec::new();
        for tick in 0..12_u32 {
            let (dx, _) = edge_scroll_step(
                &mut st,
                (0.0, 300.0),
                VIEW_W,
                VIEW_H,
                DEFAULT_SCROLL_RATE,
                tick,
            );
            seen.push(-dx);
        }
        // Retail curve for ScrollRate 3: 1, 2, 4, 8, 13 then held at the cap.
        assert_eq!(&seen[..5], &[1.0, 2.0, 4.0, 8.0, 13.0]);
        assert!(seen[5..].iter().all(|&v| v == 13.0), "{seen:?}");
    }

    #[test]
    fn scroll_rate_changes_the_peak_speed() {
        // ScrollRate is no longer inert: 0 is fastest, 6 slowest.
        let peaks: Vec<f32> = (0..=6_u32)
            .map(|rate| {
                let mut st = state();
                let mut last = 0.0;
                for tick in 0..20_u32 {
                    let (dx, _) =
                        edge_scroll_step(&mut st, (0.0, 300.0), VIEW_W, VIEW_H, rate, tick);
                    last = -dx;
                }
                last
            })
            .collect();
        assert_eq!(peaks, vec![26.0, 22.0, 17.0, 13.0, 8.0, 4.0, 2.0]);
    }

    #[test]
    fn leaving_the_edge_coasts_down_instead_of_stopping_dead() {
        let mut st = state();
        for tick in 0..8_u32 {
            edge_scroll_step(
                &mut st,
                (0.0, 300.0),
                VIEW_W,
                VIEW_H,
                DEFAULT_SCROLL_RATE,
                tick,
            );
        }
        // Cursor moves back into the middle of the screen.
        let mut coast = Vec::new();
        for tick in 8..16_u32 {
            let (dx, _) = edge_scroll_step(
                &mut st,
                (500.0, 300.0),
                VIEW_W,
                VIEW_H,
                DEFAULT_SCROLL_RATE,
                tick,
            );
            coast.push(-dx);
        }
        // Still moving west, decaying one table step per tick. gamemd stops
        // dead once the counter reaches zero — there is no trailing 1 px creep.
        assert_eq!(coast, vec![13.0, 8.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn coast_only_steps_once_per_radar_tick() {
        let mut st = state();
        // Five calls inside the same 16 ms tick must all use the same speed
        // after the first (zero-delay) step.
        let first = edge_scroll_step(
            &mut st,
            (0.0, 300.0),
            VIEW_W,
            VIEW_H,
            DEFAULT_SCROLL_RATE,
            7,
        );
        assert_eq!(first, (-1.0, 0.0));
        for _ in 0..4 {
            assert_eq!(
                edge_scroll_step(
                    &mut st,
                    (0.0, 300.0),
                    VIEW_W,
                    VIEW_H,
                    DEFAULT_SCROLL_RATE,
                    7
                ),
                (-2.0, 0.0)
            );
        }
        assert_eq!(
            edge_scroll_step(
                &mut st,
                (0.0, 300.0),
                VIEW_W,
                VIEW_H,
                DEFAULT_SCROLL_RATE,
                8
            ),
            (-2.0, 0.0)
        );
        assert_eq!(
            edge_scroll_step(
                &mut st,
                (0.0, 300.0),
                VIEW_W,
                VIEW_H,
                DEFAULT_SCROLL_RATE,
                9
            ),
            (-4.0, 0.0)
        );
    }

    #[test]
    fn corner_scroll_moves_the_full_distance_on_both_axes() {
        let mut st = state();
        let (dx, dy) =
            edge_scroll_step(&mut st, (0.0, 0.0), VIEW_W, VIEW_H, DEFAULT_SCROLL_RATE, 0);
        assert_eq!((dx, dy), (-1.0, -1.0));
    }
}
