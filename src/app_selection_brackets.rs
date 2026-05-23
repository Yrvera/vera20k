//! Isometric 3D selection bracket lines for buildings.
//!
//! When a building is selected, draws white bracket stub lines at the 3 visible
//! corners of its isometric bounding box (at roof level). Each corner has 3 short
//! lines radiating outward along the 3 isometric axes (X, Y, Z).
//!
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::app_commands::preferred_local_owner_name;
use crate::app_instances::in_view;
use crate::map::entities::EntityCategory;
use crate::render::batch::SpriteInstance;
use crate::render::shroud_buffer::ShroudBuffer;
use crate::sim::vision::FogState;

/// Height= multiplier: 1 art.ini Height unit = 15 screen pixels (HeightFactor * AdjustForZ).
const HEIGHT_PX: f32 = 15.0;

/// Bracket stub depth — drawn flat in the no-depth overlay pass.
const BRACKET_DEPTH: f32 = 0.0006;

/// Bracket line color — solid white, fully opaque.
const BRACKET_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const ABUFFER_NEUTRAL: u8 = 0x7F;
const ABUFFER_BLACK: u8 = 0x00;

struct BracketPixelFilter<'a> {
    shroud: Option<&'a ShroudBuffer>,
    camera_x: f32,
    camera_y: f32,
    enabled: bool,
}

impl<'a> BracketPixelFilter<'a> {
    fn from_state(state: &'a AppState) -> Self {
        Self {
            shroud: state.shroud_buffer.as_ref(),
            camera_x: state.camera_x,
            camera_y: state.camera_y,
            enabled: !state.sandbox_full_visibility,
        }
    }

    fn tint_for_pixel(&self, x: i32, y: i32) -> Option<[f32; 3]> {
        if !self.enabled {
            return Some([BRACKET_COLOR[0], BRACKET_COLOR[1], BRACKET_COLOR[2]]);
        }
        let abuf = self
            .shroud
            .and_then(|s| s.sample_world(x as f32, y as f32, self.camera_x, self.camera_y))
            .unwrap_or(ABUFFER_NEUTRAL);
        match abuf {
            ABUFFER_BLACK => None,
            ABUFFER_NEUTRAL => Some([BRACKET_COLOR[0], BRACKET_COLOR[1], BRACKET_COLOR[2]]),
            value => {
                let scale = (value as f32 / ABUFFER_NEUTRAL as f32).clamp(0.0, 1.0);
                Some([
                    BRACKET_COLOR[0] * scale,
                    BRACKET_COLOR[1] * scale,
                    BRACKET_COLOR[2] * scale,
                ])
            }
        }
    }
}

/// Check if an entity is visible to the local player for overlay purposes.
fn is_visible(
    local_owner: Option<crate::sim::intern::InternedId>,
    fog: &FogState,
    pos: &crate::sim::components::Position,
    entity_owner: crate::sim::intern::InternedId,
    ignore_visibility: bool,
) -> bool {
    if ignore_visibility {
        return true;
    }
    let Some(owner) = local_owner else {
        return true;
    };
    if owner == entity_owner {
        return true;
    }
    fog.is_cell_revealed(owner, pos.rx, pos.ry) && !fog.is_cell_gap_covered(owner, pos.rx, pos.ry)
}

/// Compute the 8 corners of a building's isometric bounding box in screen space.
///
/// Returns `(ground_corners, roof_corners)` where each is `[FL, FR, BL, BR]`.
/// Coordinates are absolute screen pixels (entity screen pos + foundation offset).
fn compute_box_corners(
    sx: f32,
    sy: f32,
    fw: f32,
    fh: f32,
    z_screen: f32,
) -> ([ScreenPt; 4], [ScreenPt; 4]) {
    // Foundation center offset from entity screen position (NW corner cell center).
    // Raw lepton offset: (fw-1)*128, (fh-1)*128.
    // Projected: cx = sx + (fw-fh)*15, cy = sy + 7.5*(fw+fh) - 15.
    let cx = sx + (fw - fh) * 15.0;
    let cy = sy + (fw + fh) * 7.5 - 15.0;

    // 4 ground corners relative to foundation center.
    // From gamemd projection: screen_dx = 30*(dx-dy)/256, screen_dy = 15*(dx+dy)/256
    // where dx = ±hw, dy = ±hh in leptons (hw = fw*128, hh = fh*128).
    // Simplifies to: screen offsets use (fw, fh) cells directly.
    let ground = [
        ScreenPt {
            x: cx - (fw + fh) * 15.0,
            y: cy + (fh - fw) * 7.5,
        }, // FL
        ScreenPt {
            x: cx + (fw - fh) * 15.0,
            y: cy + (fw + fh) * 7.5,
        }, // FR
        ScreenPt {
            x: cx + (fh - fw) * 15.0,
            y: cy - (fw + fh) * 7.5,
        }, // BL
        ScreenPt {
            x: cx + (fw + fh) * 15.0,
            y: cy - (fh - fw) * 7.5,
        }, // BR
    ];
    // Roof corners = ground corners shifted up by z_screen.
    let roof = [
        ScreenPt {
            x: ground[0].x,
            y: ground[0].y - z_screen,
        }, // FL roof
        ScreenPt {
            x: ground[1].x,
            y: ground[1].y - z_screen,
        }, // FR roof
        ScreenPt {
            x: ground[2].x,
            y: ground[2].y - z_screen,
        }, // BL roof
        ScreenPt {
            x: ground[3].x,
            y: ground[3].y - z_screen,
        }, // BR roof
    ];
    (ground, roof)
}

#[derive(Clone, Copy)]
struct ScreenPt {
    x: f32,
    y: f32,
}

#[derive(Debug, Default)]
pub(crate) struct SelectionBracketInstances {
    pub back: Vec<SpriteInstance>,
    pub front_first: Vec<SpriteInstance>,
    pub front: Vec<SpriteInstance>,
}

/// Compute the quarter-point 25% from `a` toward `b`: (3a + b) / 4.
fn quarter_point(a: ScreenPt, b: ScreenPt) -> ScreenPt {
    ScreenPt {
        x: ((a.x * 3.0 + b.x) * 0.25).trunc(),
        y: ((a.y * 3.0 + b.y) * 0.25).trunc(),
    }
}

fn screen_i32(v: f32) -> i32 {
    v.trunc() as i32
}

fn emit_pixel(instances: &mut Vec<SpriteInstance>, x: i32, y: i32, tint: [f32; 3]) {
    instances.push(SpriteInstance {
        position: [x as f32, y as f32],
        size: [1.0, 1.0],
        uv_origin: [0.0, 0.0],
        uv_size: [1.0, 1.0],
        tint,
        alpha: BRACKET_COLOR[3],
        depth: BRACKET_DEPTH,
        ..Default::default()
    });
}

/// Emit 1px-wide line segments as pixel-stepping SpriteInstance quads.
///
/// Matches the visible part of gamemd's surface-line contract for ordinary
/// bracket segments: integer endpoints, start pixel included, final endpoint
/// excluded, Bresenham-style x/y stepping. The optional filter handles the
/// post-shroud final-front bracket redraw's ABuffer pixel predicate.
fn emit_line(instances: &mut Vec<SpriteInstance>, a: ScreenPt, b: ScreenPt) {
    emit_line_with_filter(instances, a, b, None);
}

fn emit_line_with_filter(
    instances: &mut Vec<SpriteInstance>,
    a: ScreenPt,
    b: ScreenPt,
    filter: Option<&BracketPixelFilter<'_>>,
) {
    let mut x = screen_i32(a.x);
    let mut y = screen_i32(a.y);
    let end_x = screen_i32(b.x);
    let end_y = screen_i32(b.y);
    if x == end_x && y == end_y {
        return;
    }

    let dx = (end_x - x).abs();
    let dy = -(end_y - y).abs();
    let sx = if x < end_x { 1 } else { -1 };
    let sy = if y < end_y { 1 } else { -1 };
    let mut err = dx + dy;

    while x != end_x || y != end_y {
        let tint = match filter {
            Some(f) => f.tint_for_pixel(x, y),
            None => Some([BRACKET_COLOR[0], BRACKET_COLOR[1], BRACKET_COLOR[2]]),
        };
        if let Some(tint) = tint {
            emit_pixel(instances, x, y, tint);
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Emit a bracket stub: a 25% line from corner `a` toward `b`.
fn emit_stub(instances: &mut Vec<SpriteInstance>, a: ScreenPt, b: ScreenPt) {
    let qp = quarter_point(a, b);
    emit_line(instances, a, qp);
}

fn emit_stub_with_filter(
    instances: &mut Vec<SpriteInstance>,
    a: ScreenPt,
    b: ScreenPt,
    filter: &BracketPixelFilter<'_>,
) {
    let qp = quarter_point(a, b);
    emit_line_with_filter(instances, a, qp, Some(filter));
}

/// Build bracket instances for all selected buildings.
pub(crate) fn build_selection_bracket_instances(
    state: &AppState,
    sw: f32,
    sh: f32,
) -> SelectionBracketInstances {
    let Some(sim) = &state.simulation else {
        return SelectionBracketInstances::default();
    };
    let local_owner = preferred_local_owner_name(state);
    let local_owner_id = local_owner.as_deref().and_then(|n| sim.interner.get(n));
    let ignore_visibility = state.sandbox_full_visibility;
    let cam_x = state.camera_x;
    let cam_y = state.camera_y;
    let final_front_filter = BracketPixelFilter::from_state(state);
    let mut out = SelectionBracketInstances::default();

    for e in sim.entities.values() {
        if e.category != EntityCategory::Structure || !e.selected {
            continue;
        }
        let type_str = sim.interner.resolve(e.type_ref);
        if !is_visible(
            local_owner_id,
            &sim.fog,
            &e.position,
            e.owner,
            ignore_visibility,
        ) {
            continue;
        }

        let (sx, sy) = (e.position.screen_x, e.position.screen_y);

        // Look up foundation and Height from rules/art.
        let obj = state.rules.as_ref().and_then(|r| r.object(type_str));
        let (fw_u, fh_u) = obj
            .map(|o| crate::rules::foundation::foundation_dimensions(&o.foundation))
            .unwrap_or((2, 2));
        let fw = fw_u as f32;
        let fh = fh_u as f32;

        let art_key: &str = obj
            .map(|o| {
                let img = o.image.as_str();
                if img.is_empty() { o.id.as_str() } else { img }
            })
            .unwrap_or(type_str);
        let art_height: f32 = state
            .art_registry
            .as_ref()
            .and_then(|art| art.get(art_key))
            .map(|entry| entry.height as f32)
            .unwrap_or(2.0);
        let z_screen = art_height * HEIGHT_PX;

        // Compute 8 corners.
        let (g, r) = compute_box_corners(sx, sy, fw, fh, z_screen);
        // g = [FL, FR, BL, BR] ground, r = [FL, FR, BL, BR] roof

        // Viewport cull: bounding box of all roof corners (ground is below roof).
        let min_x = r[0].x.min(r[1].x).min(r[2].x).min(r[3].x);
        let max_x = r[0].x.max(r[1].x).max(r[2].x).max(r[3].x);
        let min_y = r[0].y.min(r[1].y).min(r[2].y).min(r[3].y);
        let max_y = g[0].y.max(g[1].y).max(g[2].y).max(g[3].y); // ground is lower
        if !in_view(
            min_x,
            min_y,
            max_x - min_x,
            max_y - min_y,
            cam_x,
            cam_y,
            sw,
            sh,
            60.0,
        ) {
            continue;
        }
        // --- 12 edges of the isometric bounding box ---
        // Indices: FL=0, FR=1, BL=2, BR=3

        // DrawBehind edges (5): stubs at both ends, behind sprite (hidden by building art).
        // These are drawn anyway — the building sprite naturally occludes them.
        emit_stub(&mut out.back, g[2], r[2]); // Edge 1: BL ground->BL roof (BL vertical)
        emit_stub(&mut out.back, r[2], g[2]);
        emit_stub(&mut out.back, g[3], g[2]); // Edge 2: BR ground->BL ground (back ground)
        emit_stub(&mut out.back, g[2], g[3]);
        emit_stub(&mut out.back, g[2], g[0]); // Edge 3: BL ground->FL ground (left ground)
        emit_stub(&mut out.back, g[0], g[2]);
        emit_stub(&mut out.back, r[0], r[2]); // Edge 4: FL roof->BL roof (left roof)
        emit_stub(&mut out.back, r[2], r[0]);
        emit_stub(&mut out.back, r[3], r[2]); // Edge 5: BR roof->BL roof (back roof)
        emit_stub(&mut out.back, r[2], r[3]);

        // DrawExtras bracket corner edges (4): stubs at both ends, in front of sprite.
        emit_stub(&mut out.front_first, g[0], g[1]); // Edge 6: FL ground->FR ground (front ground)
        emit_stub_with_filter(&mut out.front, g[0], g[1], &final_front_filter);
        emit_stub(&mut out.front_first, g[1], g[0]);
        emit_stub_with_filter(&mut out.front, g[1], g[0], &final_front_filter);
        emit_stub(&mut out.front_first, g[3], g[1]); // Edge 7: BR ground->FR ground (right ground)
        emit_stub_with_filter(&mut out.front, g[3], g[1], &final_front_filter);
        emit_stub(&mut out.front_first, g[1], g[3]);
        emit_stub_with_filter(&mut out.front, g[1], g[3], &final_front_filter);
        emit_stub(&mut out.front_first, r[0], g[0]); // Edge 8: FL roof->FL ground (FL vertical)
        emit_stub_with_filter(&mut out.front, r[0], g[0], &final_front_filter);
        emit_stub(&mut out.front_first, g[0], r[0]);
        emit_stub_with_filter(&mut out.front, g[0], r[0], &final_front_filter);
        emit_stub(&mut out.front_first, r[3], g[3]); // Edge 9: BR roof->BR ground (BR vertical)
        emit_stub_with_filter(&mut out.front, r[3], g[3], &final_front_filter);
        emit_stub(&mut out.front_first, g[3], r[3]);
        emit_stub_with_filter(&mut out.front, g[3], r[3], &final_front_filter);

        // DrawExtras single-stub edges (3): only stub at the visible end.
        // All converge at hidden FR_roof corner.
        emit_stub(&mut out.front_first, r[0], r[1]); // Edge 10: FL roof->FR roof (front roof, stub at FL)
        emit_stub_with_filter(&mut out.front, r[0], r[1], &final_front_filter);
        emit_stub(&mut out.front_first, r[3], r[1]); // Edge 11: BR roof->FR roof (right roof, stub at BR)
        emit_stub_with_filter(&mut out.front, r[3], r[1], &final_front_filter);
        emit_stub(&mut out.front_first, g[1], r[1]); // Edge 12: FR ground->FR roof (FR vertical, stub at FR ground)
        emit_stub_with_filter(&mut out.front, g[1], r[1], &final_front_filter);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{ScreenPt, emit_line};
    use crate::render::batch::SpriteInstance;

    fn line_positions(a: ScreenPt, b: ScreenPt) -> Vec<(i32, i32)> {
        let mut instances: Vec<SpriteInstance> = Vec::new();
        emit_line(&mut instances, a, b);
        instances
            .iter()
            .map(|i| (i.position[0] as i32, i.position[1] as i32))
            .collect()
    }

    #[test]
    fn emit_line_excludes_final_endpoint() {
        let pts = line_positions(ScreenPt { x: 0.0, y: 0.0 }, ScreenPt { x: 3.0, y: 0.0 });
        assert_eq!(pts, vec![(0, 0), (1, 0), (2, 0)]);
    }

    #[test]
    fn emit_line_handles_reverse_direction() {
        let pts = line_positions(ScreenPt { x: 3.0, y: 0.0 }, ScreenPt { x: 0.0, y: 0.0 });
        assert_eq!(pts, vec![(3, 0), (2, 0), (1, 0)]);
    }

    #[test]
    fn emit_line_truncates_float_endpoints_toward_zero() {
        let pts = line_positions(ScreenPt { x: -1.8, y: 0.9 }, ScreenPt { x: 1.8, y: 0.9 });
        assert_eq!(pts, vec![(-1, 0), (0, 0)]);
    }
}
