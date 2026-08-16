//! Camera positioning — keyboard scroll, mouse edge scroll, zoom, and clamping.
//!
//! Extracted from app_sim_tick.rs to separate camera control from sim advancement.
//!
//! ## Coordinate frames
//! Three frames meet in this file and mixing them is the classic bug here:
//! * **World pixels** — what `terrain::iso_to_screen` / `terrain::lepton_to_screen`
//!   produce. `camera_x`/`camera_y` live in this frame and name the world point
//!   drawn at window pixel `(0, 0)`. Within world pixels there are two anchors
//!   that differ by half a tile: `iso_to_screen` gives a cell's *tile* corner,
//!   while an entity standing on that cell is drawn on the cell's diamond
//!   centre. `cell_centre_world_point` converts the first into the second and
//!   is what every camera move targets.
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
use crate::app::types::ScrollDir;
use crate::map::terrain;

/// Stock YR reserves a fixed 168-pixel right sidebar and a fixed 32-pixel
/// bottom strip when it builds the tactical view rectangle.
const TACTICAL_SIDEBAR_WIDTH_PX: u32 = 168;
const TACTICAL_BOTTOM_STRIP_HEIGHT_PX: u32 = 32;

/// Arrow-key scroll distance in world pixels per frame.
///
/// gamemd's main tick reads one flat constant for every arrow key. This path has
/// no coast ramp and no `ScrollRate` term — that machinery belongs to mouse edge
/// scrolling alone — and holding two arrows moves the full distance on both axes
/// because each key issues its own scroll.
const KEY_SCROLL_DISTANCE: f32 = 21.0;

/// Shift scales the arrow-scroll distance by 2.5 and the product goes through
/// the same truncating float-to-long as the rest of the scroll math, so the
/// boosted step is 52 px per frame rather than 52.5.
const KEY_SCROLL_SHIFT_MULTIPLIER: f32 = 2.5;

/// Ctrl replaces the distance with the map's longer side in cells, shifted left
/// by the lepton-per-cell shift. That overshoots every stock map, so the clamp
/// catches it and the view lands on the map border in a single frame.
const KEY_SCROLL_CTRL_CELL_SHIFT: u32 = 8;

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

/// Constructor fallback for `[AudioVisual] ScrollMultiplier`.
const DEFAULT_SCROLL_MULTIPLIER: f64 = 0.07;

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

    fn coasting_direction(&self) -> Option<ScrollDir> {
        (self.coast_level > 0).then(|| scroll_dir_from_octant(self.octant))
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

fn scroll_dir_from_octant(octant: usize) -> ScrollDir {
    match octant & 7 {
        0 => ScrollDir::N,
        1 => ScrollDir::NE,
        2 => ScrollDir::E,
        3 => ScrollDir::SE,
        4 => ScrollDir::S,
        5 => ScrollDir::SW,
        6 => ScrollDir::W,
        _ => ScrollDir::NW,
    }
}

fn scroll_dir_octant(dir: ScrollDir) -> usize {
    match dir {
        ScrollDir::N => 0,
        ScrollDir::NE => 1,
        ScrollDir::E => 2,
        ScrollDir::SE => 3,
        ScrollDir::S => 4,
        ScrollDir::SW => 5,
        ScrollDir::W => 6,
        ScrollDir::NW => 7,
    }
}

/// Active edge-scroll intent for a cursor position. The trigger is exactly the
/// outermost integer pixel of the whole window, sidebar included.
pub(crate) fn edge_scroll_intent(
    cursor: (f32, f32),
    view_w: i32,
    view_h: i32,
) -> Option<ScrollDir> {
    if view_w <= 0 || view_h <= 0 {
        return None;
    }
    let x = cursor.0.floor() as i32;
    let y = cursor.1.floor() as i32;
    (x <= 0 || y <= 0 || x >= view_w - 1 || y >= view_h - 1)
        .then(|| scroll_dir_from_octant(edge_scroll_octant(x, y, view_w, view_h)))
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
/// `movement_allowed` is the one-pixel preflight through the tactical clamp.
/// A blocked active edge skips the timer block; a blocked coast still decays.
/// `right_button_held` selects the slower `clamp(index + 1, 4, 8)` lane.
fn edge_scroll_step_with_context(
    state: &mut EdgeScrollState,
    cursor: (f32, f32),
    view_w: i32,
    view_h: i32,
    scroll_rate: u32,
    scroll_multiplier: f64,
    right_button_held: bool,
    movement_allowed: bool,
    now: u32,
) -> (f32, f32) {
    let active_direction = edge_scroll_intent(cursor, view_w, view_h);

    // ScrollRate caps the peak speed by flooring the table index, and the coast
    // counter is written back clamped so it cannot run away above the cap.
    // Native applies this cap before a blocked active-edge request returns.
    // The array clamp also makes a user-edited value outside the stock slider
    // range safe without changing ordinary 0..=6 behavior.
    let rate_floor = i32::try_from(scroll_rate)
        .unwrap_or(i32::MAX)
        .saturating_add(1);
    let base_index = ((8 - state.coast_level).max(rate_floor))
        .clamp(0, EDGE_SCROLL_SPEED_TABLE.len() as i32 - 1);
    state.coast_level = 8 - base_index;

    if active_direction.is_some() && !movement_allowed {
        // A blocked active edge returns after the cap but before direction,
        // timer, or ramp state changes.
        return (0.0, 0.0);
    }

    if let Some(direction) = active_direction {
        state.octant = scroll_dir_octant(direction);
    } else if state.coast_level == 0 {
        // Idle: nothing moves, but the decay timer is still serviced.
        state.decay(now);
        return (0.0, 0.0);
    }

    let index = if right_button_held {
        base_index.saturating_add(1).clamp(4, 8)
    } else {
        base_index
    };

    // Truncating float-to-long, matching gamemd's chop rounding mode: the stock
    // multiplier turns the table into 1, 2, 4, 8, 13, 17, 22, 26, 31 px/frame.
    let distance =
        (f64::from(EDGE_SCROLL_SPEED_TABLE[index as usize]) * scroll_multiplier).trunc() as f32;

    if active_direction.is_some() {
        state.ramp_up(now);
    } else {
        state.decay(now);
    }

    if !movement_allowed {
        // A blocked coast still reaches the normal decay above.
        return (0.0, 0.0);
    }

    let (dx, dy) = OCTANT_DELTA[state.octant];
    (dx * distance, dy * distance)
}

#[cfg(test)]
fn edge_scroll_step(
    state: &mut EdgeScrollState,
    cursor: (f32, f32),
    view_w: i32,
    view_h: i32,
    scroll_rate: u32,
    now: u32,
) -> (f32, f32) {
    edge_scroll_step_with_context(
        state,
        cursor,
        view_w,
        view_h,
        scroll_rate,
        DEFAULT_SCROLL_MULTIPLIER,
        false,
        true,
        now,
    )
}

// ---------------------------------------------------------------------------
// Camera bookmarks — the View1..4 / SetView1..4 commands.
// ---------------------------------------------------------------------------

/// Number of camera bookmark slots. gamemd ships exactly four commands per
/// direction (`View1..View4`, `SetView1..SetView4`), bound to F1..F4 and
/// Ctrl+F1..Ctrl+F4 in the stock keyboard INI.
pub(crate) const VIEW_BOOKMARK_SLOTS: usize = 4;

/// The four camera bookmark slots.
///
/// Each slot is a **cell**, not a pixel or lepton position: the recall command
/// projects the cell centre and lands it at the middle of the tactical viewport,
/// then runs the same clamp every other scroll source uses. Recall is an instant
/// jump, not a glide.
///
/// All four slots are seeded with the scenario's opening view at map load, so a
/// recall before any set is a valid "go home" rather than a jump to the map
/// corner. That is why the slots are plain cells rather than `Option`s — the
/// native structure has no empty state.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ViewBookmarks {
    slots: [(u16, u16); VIEW_BOOKMARK_SLOTS],
}

impl ViewBookmarks {
    /// Seed every slot with one cell (the map-load chained assignment).
    pub(crate) fn seed_all(&mut self, rx: u16, ry: u16) {
        self.slots = [(rx, ry); VIEW_BOOKMARK_SLOTS];
    }

    /// Store `cell` in one slot. Out-of-range slots are ignored.
    pub(crate) fn set(&mut self, slot: usize, rx: u16, ry: u16) {
        if let Some(entry) = self.slots.get_mut(slot) {
            *entry = (rx, ry);
        }
    }

    /// Read one slot.
    pub(crate) fn get(&self, slot: usize) -> Option<(u16, u16)> {
        self.slots.get(slot).copied()
    }
}

/// The cell under the centre of the tactical viewport — the point `SetView`
/// captures. gamemd reads the *view's* coordinate, not the cursor's, so a
/// bookmark records where the player is looking rather than where the mouse
/// happens to sit.
fn tactical_centre_cell(state: &AppState) -> (u16, u16) {
    let (tactical_w, tactical_h) =
        tactical_viewport_size_px(state.render_width(), state.render_height());
    let world_x = state.input.camera_x + tactical_w as f32 / (2.0 * state.input.zoom_level);
    let world_y = state.input.camera_y + tactical_h as f32 / (2.0 * state.input.zoom_level);
    crate::app::match_runtime::sim_tick::world_point_to_cell(
        world_x,
        world_y,
        &state.height_map(),
        Some(&state.match_presentation.tactical_bridge_inverse_map),
    )
}

/// `SetView<slot+1>` — capture the current view into a bookmark.
pub(crate) fn set_view_bookmark(state: &mut AppState, slot: usize) {
    let (rx, ry) = tactical_centre_cell(state);
    state.input.view_bookmarks.set(slot, rx, ry);
    log::info!("SetView{}: bookmark set to cell ({rx}, {ry})", slot + 1);
}

/// `View<slot+1>` — jump the camera to a bookmark.
pub(crate) fn recall_view_bookmark(state: &mut AppState, slot: usize) {
    let Some((rx, ry)) = state.input.view_bookmarks.get(slot) else {
        return;
    };
    center_camera_on_cell(state, rx, ry);
}

/// Seed all four bookmarks with the cell the view is currently centred on.
/// Called from the map-load and spawn-pick paths, which are where gamemd's
/// scenario reader fills the four slots with the opening view.
pub(crate) fn seed_view_bookmarks_from_current_view(state: &mut AppState) {
    let (rx, ry) = tactical_centre_cell(state);
    state.input.view_bookmarks.seed_all(rx, ry);
}

// ---------------------------------------------------------------------------
// Right-drag map pan.
// ---------------------------------------------------------------------------

/// Default `SM_CXDRAG` / `SM_CYDRAG` value used by the non-Windows development
/// fallback. The retail Windows path reads the live per-machine metrics.
#[cfg(not(windows))]
const DEFAULT_SYSTEM_DRAG_METRIC_PX: i32 = 4;

#[cfg(windows)]
const SM_CXDRAG: i32 = 68;
#[cfg(windows)]
const SM_CYDRAG: i32 = 69;

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn GetSystemMetrics(index: i32) -> i32;
}

/// Current Windows drag metrics, or the ordinary Windows default on development
/// targets that do not expose `GetSystemMetrics`.
fn system_drag_metrics_px() -> (i32, i32) {
    #[cfg(windows)]
    {
        // SAFETY: GetSystemMetrics is a process-wide read with no pointer
        // arguments. SM_CXDRAG and SM_CYDRAG return pixel counts.
        unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) }
    }
    #[cfg(not(windows))]
    {
        (DEFAULT_SYSTEM_DRAG_METRIC_PX, DEFAULT_SYSTEM_DRAG_METRIC_PX)
    }
}

/// gamemd crosses the right-drag latch when either axis is strictly greater
/// than twice its corresponding Windows drag metric. This is per-axis, not a
/// Euclidean-distance test.
fn right_drag_threshold_crossed(
    delta_x: f32,
    delta_y: f32,
    drag_metric_x: i32,
    drag_metric_y: i32,
) -> bool {
    delta_x.abs() > drag_metric_x.saturating_mul(2) as f32
        || delta_y.abs() > drag_metric_y.saturating_mul(2) as f32
}

/// gamemd divides the anchor displacement by `ScrollRate + 1` and truncates, so
/// a right drag held 100 px from its anchor with the stock rate moves the camera
/// 25 px along that axis every frame.
const RIGHT_DRAG_RATE_BIAS: u32 = 1;

/// Distance from a screen border, in pixels, inside which an anchor makes a
/// drag pushing further toward that border boost.
const RIGHT_DRAG_EDGE_BAND_PX: f32 = 10.0;
/// Minimum displacement the edge boost substitutes before the multiply.
const RIGHT_DRAG_EDGE_MIN_PX: i32 = 5;
/// The edge-boost multiplier (a left shift by two in the original).
const RIGHT_DRAG_EDGE_BOOST: i32 = 4;

/// Tactical mouse capture plus the right-drag pan's anchor and latches.
///
/// gamemd sets one "a button is captured" byte on the left *and* the right press
/// inside the play area and clears it on the matching release. While it is set,
/// the per-frame mouse block routes to the band-box or right-drag branch and
/// edge auto-scroll early-returns — which is why pushing a band box into a
/// screen border does not scroll the map in the original.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TacticalMouseState {
    /// A tactical mouse button holds the capture.
    pub(crate) captured: bool,
    /// Left button physically down.
    pub(crate) left_held: bool,
    /// Right button physically down.
    pub(crate) right_held: bool,
    /// Right-press anchor in render-target pixels.
    pub(crate) right_anchor: (f32, f32),
    /// The right drag has passed the system drag threshold. Once set, the
    /// release no longer runs the cancel/deselect ladder.
    pub(crate) right_threshold_crossed: bool,
    /// The right drag owns the camera (the band box did not win the race).
    right_pan_engaged: bool,
}

impl TacticalMouseState {
    /// Right press inside the play area: record the anchor and take the capture.
    /// The press itself has no game effect in gamemd.
    pub(crate) fn begin_right_drag(&mut self, cursor: (f32, f32)) {
        self.right_anchor = cursor;
        self.right_threshold_crossed = false;
        self.right_pan_engaged = false;
        self.captured = true;
    }

    /// Drop the capture and both right-drag latches.
    pub(crate) fn release(&mut self) {
        self.captured = false;
        self.right_threshold_crossed = false;
        self.right_pan_engaged = false;
    }

    /// True while the right button owns the per-frame mouse block. The left
    /// button is tested first in the original, so a band box in progress keeps
    /// the pan from running.
    fn right_drag_owns_frame(&self) -> bool {
        self.captured && self.right_held && !self.left_held
    }
}

/// One right-drag pan step, in world pixels, for `ScrollMethod = 0` (the stock
/// value — the two cursor-warping methods have no in-game UI and ship disabled).
///
/// The displacement is measured from the **anchor**, not from the previous
/// frame, so the gesture behaves like a joystick: the further the cursor sits
/// from the press point, the faster the map slides, every frame the button is
/// held. Both axes truncate toward zero, matching the original's float-to-long.
fn right_drag_pan_step(
    anchor: (f32, f32),
    cursor: (f32, f32),
    view_w: f32,
    view_h: f32,
    scroll_rate: u32,
) -> (f32, f32) {
    // The native handler works on integer mouse coordinates.
    let mut dx = (cursor.0 - anchor.0).trunc() as i32;
    let mut dy = (cursor.1 - anchor.1).trunc() as i32;

    // An anchor pinned against a border still scrolls outward when the cursor
    // cannot travel any further. The native test is asymmetric — the X arm
    // compares against `width - 1` and the Y arm against the raw height.
    if dx == 0 {
        if anchor.0 <= 0.0 {
            dx = -1;
        } else if anchor.0 >= view_w - 1.0 {
            dx = 1;
        }
    }
    if dy == 0 {
        if anchor.1 <= 0.0 {
            dy = -1;
        } else if anchor.1 >= view_h {
            dy = 1;
        }
    }

    // Anchor within ten pixels of a border, dragging further that way: force at
    // least five pixels of displacement, then multiply by four.
    if anchor.0 < RIGHT_DRAG_EDGE_BAND_PX && dx < 0 {
        dx = dx.min(-RIGHT_DRAG_EDGE_MIN_PX) * RIGHT_DRAG_EDGE_BOOST;
    } else if anchor.0 > view_w - RIGHT_DRAG_EDGE_BAND_PX && dx > 0 {
        dx = dx.max(RIGHT_DRAG_EDGE_MIN_PX) * RIGHT_DRAG_EDGE_BOOST;
    }
    if anchor.1 < RIGHT_DRAG_EDGE_BAND_PX && dy < 0 {
        dy = dy.min(-RIGHT_DRAG_EDGE_MIN_PX) * RIGHT_DRAG_EDGE_BOOST;
    } else if anchor.1 > view_h - RIGHT_DRAG_EDGE_BAND_PX && dy > 0 {
        dy = dy.max(RIGHT_DRAG_EDGE_MIN_PX) * RIGHT_DRAG_EDGE_BOOST;
    }

    let divisor = (scroll_rate + RIGHT_DRAG_RATE_BIAS) as f32;
    ((dx as f32 / divisor).trunc(), (dy as f32 / divisor).trunc())
}

/// Drive the right-drag map pan for this frame.
fn update_right_drag_pan(state: &mut AppState) {
    if !state.input.tactical_mouse.right_drag_owns_frame() {
        return;
    }
    let anchor = state.input.tactical_mouse.right_anchor;
    let cursor = (state.input.cursor_x, state.input.cursor_y);
    let (drag_metric_x, drag_metric_y) = system_drag_metrics_px();
    if !state.input.tactical_mouse.right_threshold_crossed
        && right_drag_threshold_crossed(
            cursor.0 - anchor.0,
            cursor.1 - anchor.1,
            drag_metric_x,
            drag_metric_y,
        )
    {
        state.input.tactical_mouse.right_threshold_crossed = true;
    }
    if !state.input.tactical_mouse.right_threshold_crossed {
        return;
    }
    if !state.input.tactical_mouse.right_pan_engaged {
        // A live band box wins the race: the original cancels the drag instead
        // of engaging the pan, and only engages on a later frame.
        if state.selection_state.is_band_box_active() {
            state.selection_state.cancel_drag();
            return;
        }
        state.input.tactical_mouse.right_pan_engaged = true;
    }

    let (dx, dy) = right_drag_pan_step(
        anchor,
        cursor,
        state.render_width() as f32,
        state.render_height() as f32,
        state.in_game_options.scroll_rate,
    );
    // The pan distance is in window pixels. Stock YR has no world zoom, so the
    // divide is VERA-internal and exact at zoom 1.0.
    state.input.camera_x += dx / state.input.zoom_level;
    state.input.camera_y += dy / state.input.zoom_level;
}

/// Arrow-key scroll distance for this frame, in world pixels.
///
/// Shift is tested before Ctrl in the original, so Shift wins when both are
/// held. The Ctrl distance is derived from the map's longer side; with no map
/// loaded there is nothing to jump across and the distance is zero.
fn keyboard_scroll_distance(state: &AppState) -> f32 {
    if crate::app::input::dispatch::is_shift_held(state) {
        (KEY_SCROLL_DISTANCE * KEY_SCROLL_SHIFT_MULTIPLIER).trunc()
    } else if crate::app::input::dispatch::is_ctrl_held(state) {
        let cells = state
            .sim_runtime
            .as_ref()
            .map(|rt| &rt.simulation)
            .map_or(0u32, |sim| u32::from(sim.fog.width.max(sim.fog.height)));
        (cells << KEY_SCROLL_CTRL_CELL_SHIFT) as f32
    } else {
        KEY_SCROLL_DISTANCE
    }
}

/// Update camera position based on keyboard and mouse edge scrolling.
pub(crate) fn update_camera(state: &mut AppState) {
    let sw: f32 = state.render_width() as f32;
    let sh: f32 = state.render_height() as f32;

    let key_distance = keyboard_scroll_distance(state);
    if state
        .input.keys_held
        .contains(&winit::keyboard::KeyCode::ArrowLeft)
    {
        state.input.camera_x -= key_distance / state.input.zoom_level;
    }
    if state
        .input.keys_held
        .contains(&winit::keyboard::KeyCode::ArrowRight)
    {
        state.input.camera_x += key_distance / state.input.zoom_level;
    }
    if state.input.keys_held.contains(&winit::keyboard::KeyCode::ArrowUp) {
        state.input.camera_y -= key_distance / state.input.zoom_level;
    }
    if state
        .input.keys_held
        .contains(&winit::keyboard::KeyCode::ArrowDown)
    {
        state.input.camera_y += key_distance / state.input.zoom_level;
    }

    update_right_drag_pan(state);
    // Each native scroll request reaches the clamp before the next source
    // probes. Keep the current point valid before edge/coast preflight.
    clamp_camera_to_playable_area(state, sw, sh);

    // gamemd's edge scroll early-returns while any tactical mouse button holds
    // the capture, so the map is frozen for the whole of a band-box or
    // right-drag gesture. The minimap-drag inhibit is VERA-internal: gamemd's
    // minimap re-centres only on press, while this flag owns the gesture here.
    if !state.input.tactical_mouse.captured && !state.minimap_dragging {
        let now = state.input.edge_scroll.radar_timer();
        let scroll_rate = state.in_game_options.scroll_rate;
        let active_direction =
            edge_scroll_intent((state.input.cursor_x, state.input.cursor_y), sw as i32, sh as i32);
        let requested_direction =
            active_direction.or_else(|| state.input.edge_scroll.coasting_direction());
        let movement_allowed = requested_direction
            .is_none_or(|direction| camera_scroll_direction_allowed(state, direction, sw, sh));
        let scroll_multiplier = state
            .rules()
            .map_or(DEFAULT_SCROLL_MULTIPLIER, |rules| {
                rules.general.scroll_multiplier
            });
        let (dx, dy) = edge_scroll_step_with_context(
            &mut state.input.edge_scroll,
            (state.input.cursor_x, state.input.cursor_y),
            sw as i32,
            sh as i32,
            scroll_rate,
            scroll_multiplier,
            state.in_game_gadgets.right_held,
            movement_allowed,
            now,
        );
        // The speed table is in window pixels. Stock YR has no world zoom, so
        // the divide is VERA-internal: it keeps the on-screen scroll rate
        // constant across VERA's zoom range and is exact at zoom 1.0.
        state.input.camera_x += dx / state.input.zoom_level;
        state.input.camera_y += dy / state.input.zoom_level;
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

/// Set zoom target, anchored on the cursor position.
///
/// Records the world point under the cursor so `animate_zoom` can keep it
/// pinned at that screen position during the smooth ease.
///
/// **Currently unbound.** Stock YR has no world zoom at all, and the wheel — the
/// only input that used to reach this — is the sidebar strip scroll in gamemd.
/// The zoom machinery is kept intact (`animate_zoom` still runs, and every
/// camera path still divides by `zoom_level`) so a future non-wheel binding or a
/// spectator/debug view can drive it, but nothing calls this today.
#[allow(dead_code)]
pub(crate) fn apply_zoom(state: &mut AppState, delta_lines: f32) {
    let old_target = state.input.zoom_target;
    let factor = ZOOM_STEP.powf(delta_lines);
    let new_target = (old_target * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    if (new_target - old_target).abs() < 1e-6 {
        return;
    }

    // Record the world point under the cursor — animate_zoom keeps it stable.
    let z = state.input.zoom_level;
    state.input.zoom_anchor_world = [
        state.input.cursor_x / z + state.input.camera_x,
        state.input.cursor_y / z + state.input.camera_y,
    ];
    state.input.zoom_anchor_screen = [state.input.cursor_x, state.input.cursor_y];
    state.input.zoom_target = new_target;
}

/// Animate zoom_level toward zoom_target each frame, adjusting the camera so
/// the anchor world point stays at the anchor screen position.
pub(crate) fn animate_zoom(state: &mut AppState) {
    let diff = state.input.zoom_target - state.input.zoom_level;
    if diff.abs() < ZOOM_SNAP {
        if (state.input.zoom_level - state.input.zoom_target).abs() > 1e-7 {
            state.input.zoom_level = state.input.zoom_target;
            let sw = state.render_width() as f32;
            let sh = state.render_height() as f32;
            clamp_camera_to_playable_area(state, sw, sh);
        }
        return;
    }

    state.input.zoom_level += diff * ZOOM_EASE;

    // Adjust camera so the anchor world point stays at the anchor screen position:
    //   anchor_world_x = anchor_screen_x / zoom + camera_x
    //   camera_x = anchor_world_x - anchor_screen_x / zoom
    state.input.camera_x = state.input.zoom_anchor_world[0] - state.input.zoom_anchor_screen[0] / state.input.zoom_level;
    state.input.camera_y = state.input.zoom_anchor_world[1] - state.input.zoom_anchor_screen[1] / state.input.zoom_level;

    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    clamp_camera_to_playable_area(state, sw, sh);
}

/// Camera top-left, in world pixels, that puts `world` at the centre of the
/// **tactical** viewport rather than the centre of the window.
///
/// The tactical extents are window pixels, so their half-extents are divided by
/// `zoom` to come back to world pixels.
pub(crate) fn tactical_camera_top_left(
    world: (f32, f32),
    tactical_w: f32,
    tactical_h: f32,
    zoom: f32,
) -> (f32, f32) {
    (
        world.0 - tactical_w / (2.0 * zoom),
        world.1 - tactical_h / (2.0 * zoom),
    )
}

/// The tactical viewport, in render-target pixels, as `(x, y, w, h)`.
///
/// The native engine keeps exactly one tactical rect and clips **every**
/// battlefield draw to it: an object intersects its own screen rect with the
/// tactical rect and hands the intersection to its blitter as a clip rect, and
/// the depth and shroud buffers are allocated at the rect's dimensions, not the
/// screen's. The sidebar lives on its own surface that is blitted alongside, so
/// no battlefield pixel can reach the sidebar column even when a sprite,
/// bracket or health bar overhangs the boundary.
///
/// VERA composites the whole frame into one render target, so that guarantee has
/// to come from a scissor rect instead — otherwise the battlefield is drawn
/// across the full window and only *covered* by whatever sidebar art happens to
/// be opaque that frame.
///
pub(crate) fn tactical_viewport_px(state: &AppState) -> (u32, u32, u32, u32) {
    let (width, height) = tactical_viewport_size_px(state.render_width(), state.render_height());
    (0, 0, width, height)
}

/// Active YR tactical dimensions. The right sidebar and bottom strip are fixed
/// native-pixel reservations at every supported resolution. Degenerate test
/// targets retain a one-pixel scissor rather than producing an invalid extent.
pub(crate) fn tactical_viewport_size_px(render_w: u32, render_h: u32) -> (u32, u32) {
    (
        render_w.saturating_sub(TACTICAL_SIDEBAR_WIDTH_PX).max(1),
        render_h
            .saturating_sub(TACTICAL_BOTTOM_STRIP_HEIGHT_PX)
            .max(1),
    )
}

/// World-pixel position of a cell's **projected cell coordinate** — the point a
/// "go here" camera move should land on, and where an entity standing on that
/// cell is drawn.
///
/// gamemd's camera-set builds the cell-centre lepton coordinate `(cell << 8) +
/// 0x80` on both axes and projects it. VERA's reproduction of that same point is
/// `util::lepton::lepton_to_screen` at the cell centre, which is the centre of
/// the cell's diamond: `iso_to_screen + (TILE_WIDTH/2, TILE_HEIGHT/2)`. Both
/// half-tiles are needed because `iso_to_screen` anchors the north-west corner
/// of the tile's diamond bounding box, and the projected cell coordinate sits
/// half a tile east and half a tile south of it.
///
/// Keeping this identical to the entity projection is the whole contract:
/// centring on a cell has to put a unit standing there at the tactical centre,
/// and `centring_lands_a_unit_on_that_cell_at_the_tactical_centre` fails the
/// moment the two drift apart.
pub(crate) fn cell_centre_world_point(rx: u16, ry: u16, z: u8) -> (f32, f32) {
    let (nw_x, nw_y) = terrain::iso_to_screen(rx, ry, z);
    (
        nw_x + terrain::TILE_WIDTH / 2.0,
        nw_y + terrain::TILE_HEIGHT / 2.0,
    )
}

pub(crate) fn center_camera_on_cell(state: &mut AppState, rx: u16, ry: u16) {
    let z = state.height_map().get(&(rx, ry)).copied().unwrap_or(0);
    let world = cell_centre_world_point(rx, ry, z);
    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    let (tactical_w, tactical_h) =
        tactical_viewport_size_px(state.render_width(), state.render_height());
    let (cx, cy) = tactical_camera_top_left(
        world,
        tactical_w as f32,
        tactical_h as f32,
        state.input.zoom_level,
    );
    state.input.camera_x = cx;
    state.input.camera_y = cy;
    clamp_camera_to_playable_area(state, sw, sh);
}

pub(crate) fn clamp_camera_to_playable_area(state: &mut AppState, sw: f32, sh: f32) {
    let (camera_x, camera_y) =
        clamp_camera_point_for_state(state, (state.input.camera_x, state.input.camera_y), sw, sh);
    state.input.camera_y = camera_y;
    state.input.camera_x = camera_x;
}

fn clamp_camera_point_for_state(
    state: &AppState,
    point: (f32, f32),
    sw: f32,
    sh: f32,
) -> (f32, f32) {
    let Some(grid) = &state.match_presentation.terrain_grid else {
        return point;
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
    let (viewport_w, viewport_h) =
        tactical_viewport_size_px(sw.max(0.0) as u32, sh.max(0.0) as u32);
    clamp_camera_point_to_local_bounds(
        point,
        (area_x, area_y, area_w, area_h),
        (viewport_w as f32, viewport_h as f32),
        state.input.zoom_level,
    )
}

fn clamp_camera_point_to_local_bounds(
    point: (f32, f32),
    area: (f32, f32, f32, f32),
    viewport: (f32, f32),
    zoom: f32,
) -> (f32, f32) {
    let (area_x, area_y, area_w, area_h) = area;
    let visible_w = viewport.0 / zoom;
    let visible_h = viewport.1 / zoom;
    let x_min = area_x - 30.0;
    let x_max = x_min + area_w - visible_w;
    let y_min = area_y;
    let y_max = y_min + area_h - visible_h - 15.0;

    // Native comparison order is Y low/high, then X low/high. Each side is a
    // strict comparison, so an exact boundary remains untouched. Do not invent
    // a centred fallback when a custom map is smaller than the viewport.
    let mut x = point.0;
    let mut y = point.1;
    if y < y_min {
        y = y_min;
    } else if y > y_max {
        y = y_max;
    }
    if x < x_min {
        x = x_min;
    } else if x > x_max {
        x = x_max;
    }
    (x, y)
}

fn camera_scroll_direction_allowed(
    state: &AppState,
    direction: ScrollDir,
    sw: f32,
    sh: f32,
) -> bool {
    if state.match_presentation.terrain_grid.is_none() {
        return true;
    }
    let (dx, dy) = OCTANT_DELTA[scroll_dir_octant(direction)];
    let candidate = (
        state.input.camera_x + dx / state.input.zoom_level,
        state.input.camera_y + dy / state.input.zoom_level,
    );
    let clamped = clamp_camera_point_for_state(state, candidate, sw, sh);
    requested_scroll_survives_clamp((state.input.camera_x, state.input.camera_y), clamped, direction)
}

fn requested_scroll_survives_clamp(
    current: (f32, f32),
    clamped: (f32, f32),
    direction: ScrollDir,
) -> bool {
    let (dx, dy) = OCTANT_DELTA[scroll_dir_octant(direction)];
    (dx != 0.0 && clamped.0 != current.0) || (dy != 0.0 && clamped.1 != current.1)
}

/// Active edge intent plus whether the tactical clamp removes all requested
/// components. Cursor feedback and motion both use this exact probe.
pub(crate) fn edge_scroll_cursor_state(state: &AppState) -> Option<(ScrollDir, bool)> {
    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    let direction = edge_scroll_intent((state.input.cursor_x, state.input.cursor_y), sw as i32, sh as i32)?;
    Some((
        direction,
        !camera_scroll_direction_allowed(state, direction, sw, sh),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW_W: f32 = 1024.0;
    const WINDOW_H: f32 = 768.0;
    const SIDEBAR_W: f32 = 168.0;
    const TACTICAL_W: f32 = WINDOW_W - SIDEBAR_W;
    const TACTICAL_H: f32 = WINDOW_H - TACTICAL_BOTTOM_STRIP_HEIGHT_PX as f32;

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
                TACTICAL_W,
                TACTICAL_H,
                1.0,
            );
            let (sx, sy) = screen_of(unit, camera, 1.0);
            assert!(
                (sx - TACTICAL_W / 2.0).abs() < 1e-3,
                "cell ({rx},{ry},z={z}): sx {sx}"
            );
            assert!(
                (sy - TACTICAL_H / 2.0).abs() < 1e-3,
                "cell ({rx},{ry},z={z}): sy {sy}"
            );
        }
    }

    /// The centring target IS the tile diamond's centre, because that is where
    /// gamemd projects a cell's coordinate and therefore where it draws a unit
    /// standing on that cell.
    ///
    /// This test previously asserted the opposite — that the target was the tile
    /// row, 15 px above the diamond centre — and it was right about the *code*
    /// and wrong about the *engine*: the entity projection it was checked
    /// against was itself half a tile high, so both sides agreed on the wrong
    /// row. With the entity anchor now on the diamond centre, the target follows
    /// it there. The invariant the pair really encodes is the assertion below
    /// that the two are equal; the literals are the hand-walked fixture.
    #[test]
    fn centring_target_is_the_tile_diamond_centre_where_a_unit_stands() {
        // Hand-walked fixture: cell (10, 10) at ground level.
        //   iso_to_screen        = (30*(10-10) - 30, 15*(10+10) + 15) = (-30, 315)
        //   diamond centre       = iso_to_screen + (30, 15)           = (  0, 330)
        assert_eq!(cell_centre_world_point(10, 10, 0), (0.0, 330.0));
        assert_eq!(
            cell_centre_world_point(10, 10, 0),
            entity_world_point(10, 10, 0),
            "centring on a cell must target exactly where a unit on it is drawn"
        );

        let camera = tactical_camera_top_left(
            cell_centre_world_point(10, 10, 0),
            TACTICAL_W,
            TACTICAL_H,
            1.0,
        );
        // Tactical rect is 1024 - 168 = 856 wide, so its centre is x = 428.
        assert_eq!(camera, (-428.0, -38.0));
        // Guard against the pre-fix behaviour, which anchored on the window
        // centre and put the target 84 px east of the tactical centre.
        assert_ne!(camera.0, 0.0 - WINDOW_W / 2.0);
    }

    #[test]
    fn centring_stays_on_the_tactical_centre_across_zoom() {
        for zoom in [0.25_f32, 0.5, 1.0, 2.0, 4.0] {
            let unit = entity_world_point(37, 12, 3);
            let camera = tactical_camera_top_left(
                cell_centre_world_point(37, 12, 3),
                TACTICAL_W,
                TACTICAL_H,
                zoom,
            );
            let (sx, sy) = screen_of(unit, camera, zoom);
            assert!((sx - TACTICAL_W / 2.0).abs() < 1e-3, "zoom {zoom}: sx {sx}");
            assert!((sy - TACTICAL_H / 2.0).abs() < 1e-3);
        }
    }

    /// Half a tile on **both** axes from the tile corner — the Y half is the one
    /// that used to be missing, which put every camera move 15 px north of the
    /// unit it was supposed to centre on.
    #[test]
    fn cell_centre_shifts_half_a_tile_on_both_axes_from_the_tile_corner() {
        for (rx, ry, z) in [(0_u16, 0_u16, 0_u8), (5, 9, 2), (63, 1, 0)] {
            let (nw_x, nw_y) = terrain::iso_to_screen(rx, ry, z);
            let (cx, cy) = cell_centre_world_point(rx, ry, z);
            assert_eq!(cx - nw_x, terrain::TILE_WIDTH / 2.0);
            assert_eq!(cy - nw_y, terrain::TILE_HEIGHT / 2.0);
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
    fn item82_edge_band_is_one_pixel_and_east_is_the_outer_window_edge() {
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

    #[test]
    fn item82_blocked_active_edge_caps_coast_without_changing_direction_or_timer() {
        let mut st = state();
        st.coast_level = 5;
        st.octant = 3;
        st.last_coast_change = Some(2);
        let before = (st.octant, st.last_coast_change);
        assert_eq!(
            edge_scroll_step_with_context(
                &mut st,
                (0.0, 300.0),
                VIEW_W,
                VIEW_H,
                DEFAULT_SCROLL_RATE,
                DEFAULT_SCROLL_MULTIPLIER,
                false,
                false,
                7,
            ),
            (0.0, 0.0),
        );
        assert_eq!(st.coast_level, 4, "ScrollRate cap still applies");
        assert_eq!((st.octant, st.last_coast_change), before);
    }

    #[test]
    fn item82_blocked_coast_still_decays() {
        let mut st = state();
        for tick in 0..5 {
            edge_scroll_step_with_context(
                &mut st,
                (0.0, 300.0),
                VIEW_W,
                VIEW_H,
                DEFAULT_SCROLL_RATE,
                DEFAULT_SCROLL_MULTIPLIER,
                false,
                true,
                tick,
            );
        }
        let before = st.coast_level;
        assert!(before > 0);
        assert_eq!(
            edge_scroll_step_with_context(
                &mut st,
                (500.0, 300.0),
                VIEW_W,
                VIEW_H,
                DEFAULT_SCROLL_RATE,
                DEFAULT_SCROLL_MULTIPLIER,
                false,
                false,
                5,
            ),
            (0.0, 0.0),
        );
        // The ScrollRate cap first pulls the overshot 5 back to 4, then the
        // ordinary off-edge timer decay takes it to 3.
        assert_eq!((before, st.coast_level), (5, 3));
    }

    #[test]
    fn item82_diagonal_probe_slides_when_one_component_survives() {
        assert!(requested_scroll_survives_clamp(
            (70.0, 300.0),
            (70.0, 299.0),
            ScrollDir::NW,
        ));
        assert!(!requested_scroll_survives_clamp(
            (70.0, 200.0),
            (70.0, 200.0),
            ScrollDir::NW,
        ));
    }

    #[test]
    fn item82_uncaptured_rmb_caps_edge_speed_at_thirteen() {
        let mut st = state();
        st.coast_level = 7;
        let (dx, dy) = edge_scroll_step_with_context(
            &mut st,
            ((VIEW_W - 1) as f32, 300.0),
            VIEW_W,
            VIEW_H,
            0,
            DEFAULT_SCROLL_MULTIPLIER,
            true,
            true,
            0,
        );
        assert_eq!((dx, dy), (13.0, 0.0));
    }

    #[test]
    fn item82_edge_distance_uses_loaded_scroll_multiplier() {
        let mut st = state();
        assert_eq!(
            edge_scroll_step_with_context(
                &mut st,
                ((VIEW_W - 1) as f32, 300.0),
                VIEW_W,
                VIEW_H,
                DEFAULT_SCROLL_RATE,
                0.125,
                false,
                true,
                0,
            ),
            (2.0, 0.0),
        );
    }

    /// Fixed view dimensions returned by the active right-sidebar path.
    #[test]
    fn item82_tactical_view_is_fixed_168_by_32_inset() {
        assert_eq!(tactical_viewport_size_px(800, 600), (632, 568));
        assert_eq!(tactical_viewport_size_px(640, 480), (472, 448));
        assert_eq!(tactical_viewport_size_px(1920, 1080), (1752, 1048));
    }

    #[test]
    fn item82_local_clamp_uses_native_minus_30_and_minus_15_bounds() {
        let area = (100.0, 200.0, 1_000.0, 800.0);
        let viewport = (632.0, 568.0);
        assert_eq!(
            clamp_camera_point_to_local_bounds((-999.0, -999.0), area, viewport, 1.0),
            (70.0, 200.0),
        );
        assert_eq!(
            clamp_camera_point_to_local_bounds((9_999.0, 9_999.0), area, viewport, 1.0),
            (438.0, 417.0),
        );
        assert_eq!(
            clamp_camera_point_to_local_bounds((70.0, 417.0), area, viewport, 1.0),
            (70.0, 417.0),
            "equality is not clamped",
        );
    }

    /// A degenerate layout must not scissor the battlefield out of existence —
    /// a zero-width scissor drops every tactical draw call silently.
    #[test]
    fn tactical_viewport_width_never_collapses_or_overruns_the_target() {
        assert_eq!(tactical_viewport_size_px(168, 32), (1, 1));
        assert_eq!(tactical_viewport_size_px(100, 20), (1, 1));
    }

    // -- camera bookmarks ----------------------------------------------------

    /// All four slots start on the scenario's opening view, so a recall before
    /// any set is a "go home" rather than a jump to the map corner, and setting
    /// one slot leaves the other three alone.
    #[test]
    fn bookmarks_seed_all_four_and_set_touches_one() {
        let mut marks = ViewBookmarks::default();
        marks.seed_all(37, 12);
        for slot in 0..VIEW_BOOKMARK_SLOTS {
            assert_eq!(marks.get(slot), Some((37, 12)), "slot {slot}");
        }
        marks.set(0, 10, 10);
        assert_eq!(marks.get(0), Some((10, 10)));
        for slot in 1..VIEW_BOOKMARK_SLOTS {
            assert_eq!(marks.get(slot), Some((37, 12)), "slot {slot}");
        }
        // Four slots, no more.
        assert_eq!(marks.get(VIEW_BOOKMARK_SLOTS), None);
    }

    /// Recalling a bookmark lands the stored cell at the centre of the tactical
    /// viewport — the same anchoring `center_camera_on_cell` performs, which is
    /// what the native recall does after projecting the cell centre.
    #[test]
    fn recalling_a_bookmark_centres_that_cell_in_the_tactical_viewport() {
        let mut marks = ViewBookmarks::default();
        marks.seed_all(37, 12);
        marks.set(1, 10, 10);
        let (rx, ry) = marks.get(1).expect("slot 1");
        let camera = tactical_camera_top_left(
            cell_centre_world_point(rx, ry, 0),
            TACTICAL_W,
            TACTICAL_H,
            1.0,
        );
        let (sx, sy) = screen_of(entity_world_point(rx, ry, 0), camera, 1.0);
        assert!((sx - TACTICAL_W / 2.0).abs() < 1e-3, "sx {sx}");
        assert!((sy - TACTICAL_H / 2.0).abs() < 1e-3, "sy {sy}");
    }

    // -- keyboard scroll -----------------------------------------------------

    /// The Shift boost truncates: 21 * 2.5 is 52.5 and the native float-to-long
    /// chops it to 52, not 53.
    #[test]
    fn shift_boosted_keyboard_scroll_truncates_to_52() {
        assert_eq!(KEY_SCROLL_DISTANCE, 21.0);
        assert_eq!(
            (KEY_SCROLL_DISTANCE * KEY_SCROLL_SHIFT_MULTIPLIER).trunc(),
            52.0
        );
    }

    /// The Ctrl distance is the map's longer side in cells shifted by 8, which
    /// overshoots the widest stock map by orders of magnitude — the clamp is
    /// what actually stops the view, so this is a map-edge jump.
    #[test]
    fn ctrl_keyboard_scroll_overshoots_the_whole_map() {
        for cells in [64u32, 128, 256] {
            let jump = (cells << KEY_SCROLL_CTRL_CELL_SHIFT) as f32;
            // World width of a square map that many cells on a side.
            let world_span = cells as f32 * terrain::TILE_WIDTH;
            assert!(jump > world_span, "{cells} cells: {jump} vs {world_span}");
        }
    }

    // -- right-drag pan ------------------------------------------------------

    const PAN_VIEW_W: f32 = 1024.0;
    const PAN_VIEW_H: f32 = 768.0;
    /// Stock `[Options] ScrollRate` default (0 fastest .. 6 slowest).
    const PAN_SCROLL_RATE: u32 = 3;

    /// The native threshold is strict and independent for each axis. A custom
    /// 4x6 system metric therefore requires more than 8px X or more than 12px Y.
    #[test]
    fn right_drag_threshold_is_strict_and_axis_independent() {
        assert!(!right_drag_threshold_crossed(8.0, 12.0, 4, 6));
        assert!(right_drag_threshold_crossed(8.01, 0.0, 4, 6));
        assert!(right_drag_threshold_crossed(0.0, -12.01, 4, 6));
        assert!(!right_drag_threshold_crossed(7.0, 11.0, 4, 6));
    }

    /// The pan is measured from the anchor, not from the previous frame, so
    /// holding the cursor still keeps the map sliding at a constant rate.
    #[test]
    fn pan_speed_is_the_anchor_displacement_over_scroll_rate_plus_one() {
        let anchor = (400.0, 300.0);
        assert_eq!(
            right_drag_pan_step(
                anchor,
                (500.0, 300.0),
                PAN_VIEW_W,
                PAN_VIEW_H,
                PAN_SCROLL_RATE
            ),
            (25.0, 0.0)
        );
        assert_eq!(
            right_drag_pan_step(
                anchor,
                (300.0, 380.0),
                PAN_VIEW_W,
                PAN_VIEW_H,
                PAN_SCROLL_RATE
            ),
            (-25.0, 20.0)
        );
        // Slower ScrollRate, same gesture, smaller step.
        assert_eq!(
            right_drag_pan_step(anchor, (500.0, 300.0), PAN_VIEW_W, PAN_VIEW_H, 6),
            (14.0, 0.0)
        );
    }

    /// Both axes truncate toward zero, so a drag shorter than the divisor moves
    /// the camera not at all rather than by a rounded pixel.
    #[test]
    fn pan_truncates_toward_zero_on_both_signs() {
        let anchor = (400.0, 300.0);
        assert_eq!(
            right_drag_pan_step(
                anchor,
                (403.0, 297.0),
                PAN_VIEW_W,
                PAN_VIEW_H,
                PAN_SCROLL_RATE
            ),
            (0.0, 0.0)
        );
        assert_eq!(
            right_drag_pan_step(
                anchor,
                (407.0, 293.0),
                PAN_VIEW_W,
                PAN_VIEW_H,
                PAN_SCROLL_RATE
            ),
            (1.0, -1.0)
        );
    }

    /// An anchor pressed against a border boosts: the displacement is forced to
    /// at least five pixels outward and then multiplied by four, which is how a
    /// right drag started at the screen edge still scrolls.
    #[test]
    fn pan_boosts_when_the_anchor_sits_on_a_border() {
        // Anchor 4 px from the left edge, cursor 1 px further left: the raw
        // displacement is -1, the boost substitutes -5 and quadruples it.
        let (dx, _) = right_drag_pan_step(
            (4.0, 300.0),
            (3.0, 300.0),
            PAN_VIEW_W,
            PAN_VIEW_H,
            PAN_SCROLL_RATE,
        );
        assert_eq!(dx, -5.0);
        // Away from the border the same gesture does nothing.
        let (plain, _) = right_drag_pan_step(
            (400.0, 300.0),
            (399.0, 300.0),
            PAN_VIEW_W,
            PAN_VIEW_H,
            PAN_SCROLL_RATE,
        );
        assert_eq!(plain, 0.0);
    }

    /// Capture routing: the left button is tested first, so a band box in
    /// progress keeps the right drag from stealing the camera.
    #[test]
    fn left_button_wins_the_per_frame_mouse_block() {
        let mut mouse = TacticalMouseState::default();
        mouse.right_held = true;
        mouse.begin_right_drag((100.0, 100.0));
        assert!(mouse.right_drag_owns_frame());
        mouse.left_held = true;
        assert!(!mouse.right_drag_owns_frame());
        mouse.left_held = false;
        mouse.release();
        assert!(!mouse.captured);
        assert!(!mouse.right_drag_owns_frame());
    }
}
