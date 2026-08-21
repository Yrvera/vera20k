use super::*;

#[test]
fn native_radar_screen_geometry_rejects_letterbox_and_maps_half_open_edges() {
    let wide = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 300, 180).unwrap();
    let screen = NativeRadarScreenGeometry::new(wide, [100.0, 50.0, 280.0, 216.0]);
    assert_eq!(screen.content_rect(), [100.0, 74.0, 280.0, 166.0]);
    assert_eq!(screen.screen_to_surface_pixel((100.0, 74.0)), Some((0, 0)));
    assert_eq!(screen.screen_to_surface_pixel((379.0, 239.0)), Some((139, 82)));
    assert_eq!(screen.screen_to_surface_pixel((100.0, 73.0)), None);
    assert_eq!(screen.screen_to_surface_pixel((380.0, 239.0)), None);
    assert_eq!(screen.screen_to_surface_pixel((379.0, 240.0)), None);

    let tall = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 180, 300).unwrap();
    let screen = NativeRadarScreenGeometry::new(tall, [100.0, 50.0, 280.0, 216.0]);
    assert_eq!(screen.content_rect(), [176.0, 50.0, 128.0, 216.0]);
    assert_eq!(screen.screen_to_surface_pixel((176.0, 50.0)), Some((0, 0)));
    assert_eq!(screen.screen_to_surface_pixel((303.0, 265.0)), Some((63, 107)));
    assert_eq!(screen.screen_to_surface_pixel((175.0, 50.0)), None);
}

#[test]
fn native_radar_click_inverse_uses_x87_rounding_at_corners_center_and_edge() {
    let geometry = NativeRadarSurfaceGeometry::from_raw_rect(30, 20, 300, 180).unwrap();
    assert_eq!(geometry.surface_pixel_to_cell((0, 0)), Some((-4i16 as u16, 25)));
    assert_eq!(geometry.surface_pixel_to_cell((70, 41)), Some((114, -5i16 as u16)));
    assert_eq!(geometry.surface_pixel_to_cell((139, 82)), Some((232, -35i16 as u16)));
}

#[test]
fn native_viewport_rect_matches_wide_and_tall_binary_arithmetic() {
    let wide = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 300, 180).unwrap();
    assert_eq!(
        native_viewport_rect(wide, (80, 50), 632, 570),
        NativeRadarRect { x: 9, y: 52, w: 10, h: 17 }
    );
    assert_eq!(
        native_viewport_rect(wide, (0, 0), 632, 570),
        NativeRadarRect { x: 0, y: 0, w: 10, h: 17 }
    );

    let tall = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 180, 300).unwrap();
    assert_eq!(
        native_viewport_rect(tall, (80, 60), 632, 570),
        NativeRadarRect { x: 4, y: 44, w: 8, h: 13 }
    );
    assert_eq!(
        native_viewport_rect(tall, (179, 0), 632, 570),
        NativeRadarRect { x: 55, y: 58, w: 8, h: 13 }
    );
}

#[test]
fn native_viewport_state_dirties_previous_then_rebuild_suppresses_baseline_replay() {
    let first = NativeRadarRect { x: 4, y: 5, w: 8, h: 6 };
    let second = NativeRadarRect { x: 9, y: 7, w: 8, h: 6 };
    let mut state = NativeRadarViewportState::default();
    let initial = state.update(first);
    assert_eq!(initial.previous, first);
    assert!(initial.dirty_previous_border.is_empty());

    let moved = state.update(second);
    assert_eq!(moved.previous, first);
    assert_eq!(moved.current, second);
    assert_eq!(moved.dirty_previous_border.len(), 28);
    assert_eq!(&moved.dirty_previous_border[..4], &[(4, 5), (11, 5), (4, 6), (11, 6)]);

    state.reset_for_rebuild();
    let rebuilt = state.update(second);
    assert_eq!(rebuilt.previous, second);
    assert!(rebuilt.dirty_previous_border.is_empty());
    state.reset_for_rebuild();
    assert!(state.update(second).dirty_previous_border.is_empty());
}

#[test]
fn native_viewport_outline_is_one_surface_pixel_and_uses_injected_sidebar_color() {
    let surface = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 300, 180).unwrap();
    let screen = NativeRadarScreenGeometry::new(surface, [100.0, 50.0, 280.0, 216.0]);
    let rect = NativeRadarRect { x: 10, y: 12, w: 8, h: 6 };
    let color = [164.0 / 255.0, 210.0 / 255.0, 1.0];
    let lines = native_viewport_outline_instances((3.0, 4.0), screen, rect, color);
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0].position, [123.0, 102.0]);
    assert_eq!(lines[0].size, [16.0, 2.0]);
    assert_eq!(lines[1].position, [123.0, 112.0]);
    assert_eq!(lines[2].size, [2.0, 12.0]);
    assert_eq!(lines[3].position, [137.0, 102.0]);
    assert!(lines.iter().all(|line| line.tint == color));
}
