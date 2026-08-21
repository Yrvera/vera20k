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
fn native_viewport_rect_exhausts_lower_right_equality_and_all_four_corners() {
    // Generated 140x83, w=10/h=17, horizontal half=4, vertical half=8.
    // `0x006570AD..0x0065712E` uses a strict lower-edge comparison and a
    // `size - 1 - rect_size` clamp, leaving the final generated row/column
    // outside the current camera-window outline.
    let wide = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 300, 180).unwrap();
    assert_eq!(wide.generated_size(), (140, 83));
    let fixture = |center| native_viewport_rect_from_center(wide, center, 632, 570);

    assert_eq!(fixture((3, 7)), NativeRadarRect { x: 0, y: 0, w: 10, h: 17 });
    assert_eq!(fixture((4, 8)), NativeRadarRect { x: 0, y: 0, w: 10, h: 17 });
    assert_eq!(fixture((134, 7)), NativeRadarRect { x: 129, y: 0, w: 10, h: 17 });
    assert_eq!(fixture((3, 74)), NativeRadarRect { x: 0, y: 65, w: 10, h: 17 });
    assert_eq!(fixture((134, 74)), NativeRadarRect { x: 129, y: 65, w: 10, h: 17 });

    // Just inside, equality, and just outside all settle on the native
    // lower clamp point; equality enters the clamp branch (`JL` is false).
    for center_x in [133, 134, 135] {
        assert_eq!(fixture((center_x, 40)).x, 129, "center_x={center_x}");
    }
    for center_y in [73, 74, 75] {
        assert_eq!(fixture((70, center_y)).y, 65, "center_y={center_y}");
    }

    // Repeat the boundary table through the height-constrained 64x108 branch.
    let tall = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 180, 300).unwrap();
    assert_eq!(tall.generated_size(), (64, 108));
    let fixture = |center| native_viewport_rect_from_center(tall, center, 632, 570);
    assert_eq!(fixture((2, 5)), NativeRadarRect { x: 0, y: 0, w: 8, h: 13 });
    assert_eq!(fixture((59, 5)), NativeRadarRect { x: 55, y: 0, w: 8, h: 13 });
    assert_eq!(fixture((2, 101)), NativeRadarRect { x: 0, y: 94, w: 8, h: 13 });
    assert_eq!(fixture((59, 101)), NativeRadarRect { x: 55, y: 94, w: 8, h: 13 });
}

#[test]
fn native_viewport_rect_stores_vertical_divisor_and_preserves_oversize_branch_order() {
    let tiny = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 19, 10).unwrap();
    assert_eq!(tiny.generated_size(), (140, 73));

    // At `0x00657050`, native stores 30/zoom to f32. With the old extended
    // Rust quotient this fixture produced h=280 and half-height=140.
    assert_eq!(
        native_viewport_rect_from_center(tiny, (0, 0), 632, 570),
        NativeRadarRect { x: 0, y: 0, w: 156, h: 279 },
    );

    // The clamp is an ordered if/else-if on each axis. It deliberately does
    // not revisit the lower bound after an oversize right/bottom clamp, so an
    // oversize window approached from inside can acquire a negative origin.
    assert_eq!(
        native_viewport_rect_from_center(tiny, (300, 300), 632, 570),
        NativeRadarRect { x: -17, y: -207, w: 156, h: 279 },
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
fn native_previous_border_visits_keep_duplicates_and_native_edge_order() {
    let rect = NativeRadarRect { x: 4, y: 5, w: 3, h: 2 };
    assert_eq!(
        native_previous_border_visits(rect),
        vec![
            (4, 5), (6, 5), (4, 6), (6, 6),
            (4, 5), (4, 6), (5, 5), (5, 6), (6, 5), (6, 6),
        ],
    );
    assert_eq!(
        native_previous_border_visits(NativeRadarRect { x: 4, y: 5, w: 1, h: 1 }),
        vec![(4, 5), (4, 5), (4, 5), (4, 5)],
        "MarkCellDirty receives all four corner visits even for one pixel",
    );
    assert_eq!(
        native_previous_border_visits(NativeRadarRect { x: 4, y: 5, w: 0, h: 2 }),
        vec![(4, 5), (3, 5), (4, 6), (3, 6)],
        "zero width skips only the horizontal loop",
    );
    assert_eq!(
        native_previous_border_visits(NativeRadarRect { x: 4, y: 5, w: 2, h: 0 }),
        vec![(4, 5), (4, 4), (5, 5), (5, 4)],
        "zero height skips only the vertical loop",
    );
    assert!(native_previous_border_visits(NativeRadarRect { x: 4, y: 5, w: 0, h: 0 }).is_empty());
}

#[test]
fn native_viewport_state_dirties_overlap_before_action40_rebuild_resets_baseline() {
    let wide = NativeRadarRect { x: 20, y: 20, w: 10, h: 17 };
    let overlapping = NativeRadarRect { x: 21, y: 21, w: 10, h: 17 };
    let tall = NativeRadarRect { x: 8, y: 30, w: 8, h: 13 };
    let mut state = NativeRadarViewportState::default();
    assert!(state.update(wide).dirty_previous_border.is_empty());
    assert_eq!(
        state.update(overlapping).dirty_previous_border,
        native_previous_border_visits(wide),
        "overlap does not suppress the old-border visit chronology",
    );

    state.reset_for_rebuild();
    let rebuilt = state.update(tall);
    assert_eq!(rebuilt.previous, tall);
    assert!(rebuilt.dirty_previous_border.is_empty());
    state.reset_for_rebuild();
    assert!(state.update(tall).dirty_previous_border.is_empty());
}

#[test]
fn native_viewport_outline_is_one_surface_pixel_and_uses_injected_sidebar_color() {
    let surface = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 300, 180).unwrap();
    let screen = NativeRadarScreenGeometry::new(surface, [100.0, 50.0, 280.0, 216.0]);
    let rect = NativeRadarRect { x: 10, y: 12, w: 8, h: 6 };
    let color = [164.0 / 255.0, 210.0 / 255.0, 1.0];
    let lines = native_viewport_outline_instances(
        (3.0, 4.0),
        screen,
        rect,
        [84.0, 0.0, 336.0, 600.0],
        color,
    );
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0].position, [123.0, 102.0]);
    assert_eq!(lines[0].size, [16.0, 2.0]);
    assert_eq!(lines[1].position, [137.0, 102.0]);
    assert_eq!(lines[1].size, [2.0, 12.0]);
    assert_eq!(lines[2].position, [123.0, 112.0]);
    assert_eq!(lines[3].position, [123.0, 102.0]);
    assert_eq!(lines[3].size, [2.0, 12.0]);
    assert!(lines.iter().all(|line| line.tint == color));
}

#[test]
fn native_viewport_lower_right_outline_is_inclusive_and_leaves_native_final_pixel_gap() {
    let surface = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 300, 180).unwrap();
    let screen = NativeRadarScreenGeometry::new(surface, [0.0, 0.0, 140.0, 108.0]);
    let rect = NativeRadarRect { x: 129, y: 65, w: 10, h: 17 };
    let lines = native_viewport_outline_instances(
        (0.0, 0.0),
        screen,
        rect,
        [-16.0, 0.0, 168.0, 600.0],
        [1.0; 3],
    );
    // Generated 140x83 is centered at sidebar-aperture y=12. Inclusive axis
    // endpoints are x=129..138/y=77..93; generated x=139/y=94 remain clear.
    assert_eq!(lines[0].position, [129.0, 77.0]);
    assert_eq!(lines[0].size, [10.0, 1.0]);
    assert_eq!(lines[1].position, [138.0, 77.0]);
    assert_eq!(lines[1].size, [1.0, 17.0]);
    assert_eq!(lines[2].position, [129.0, 93.0]);
    assert_eq!(lines[2].size, [10.0, 1.0]);
    assert_eq!(lines[3].position, [129.0, 77.0]);
    assert_eq!(lines[3].size, [1.0, 17.0]);
}

#[test]
fn native_viewport_outline_clips_oversize_edges_to_full_sidebar_not_radar_aperture() {
    let surface = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 140, 108).unwrap();
    assert_eq!(surface.generated_size(), (140, 108));
    let screen = NativeRadarScreenGeometry::new(surface, [216.0, 49.0, 140.0, 108.0]);
    let sidebar = [200.0, 0.0, 168.0, 300.0];

    // top=-1, right=395, left=196 are outside the retained surface. The
    // bottom line crosses the whole surface at y=198. Its x=200..215 pixels
    // deliberately survive even though they are left of the radar aperture.
    let lines = native_viewport_outline_instances(
        (7.0, 11.0),
        screen,
        NativeRadarRect { x: -20, y: -50, w: 200, h: 200 },
        sidebar,
        [1.0; 3],
    );
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].position, [207.0, 209.0]);
    assert_eq!(lines[0].size, [168.0, 1.0]);

    let final_screen_x = lines[0].position[0] - 7.0;
    let final_screen_y = lines[0].position[1] - 11.0;
    assert_eq!([final_screen_x, final_screen_y], [200.0, 198.0]);
    assert!(final_screen_x < 216.0, "the radar aperture must not be the clip rect");
}

#[test]
fn native_viewport_outline_clips_all_sidebar_edges_with_inclusive_equality() {
    let surface = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 140, 108).unwrap();
    let screen = NativeRadarScreenGeometry::new(surface, [216.0, 49.0, 140.0, 108.0]);
    let sidebar = [200.0, 0.0, 168.0, 300.0];

    let horizontal = native_viewport_outline_instances(
        (0.0, 0.0),
        screen,
        NativeRadarRect { x: -20, y: -49, w: 200, h: 10 },
        sidebar,
        [1.0; 3],
    );
    assert_eq!(horizontal.len(), 2);
    assert_eq!(horizontal[0].position, [200.0, 0.0]);
    assert_eq!(horizontal[0].size, [168.0, 1.0]);
    assert_eq!(horizontal[1].position, [200.0, 9.0]);
    assert_eq!(horizontal[1].size, [168.0, 1.0]);

    let vertical = native_viewport_outline_instances(
        (0.0, 0.0),
        screen,
        NativeRadarRect { x: -16, y: -60, w: 168, h: 400 },
        sidebar,
        [1.0; 3],
    );
    assert_eq!(vertical.len(), 2);
    assert_eq!(vertical[0].position, [367.0, 0.0]);
    assert_eq!(vertical[0].size, [1.0, 300.0]);
    assert_eq!(vertical[1].position, [200.0, 0.0]);
    assert_eq!(vertical[1].size, [1.0, 300.0]);

    // x=151 relative to the generated surface lands on the sidebar's final
    // inclusive column (367). x=152 lands at the exclusive edge and vanishes.
    let last_column = native_viewport_outline_instances(
        (0.0, 0.0),
        screen,
        NativeRadarRect { x: 151, y: 0, w: 1, h: 1 },
        sidebar,
        [1.0; 3],
    );
    assert_eq!(last_column.len(), 4);
    assert!(last_column.iter().all(|line| line.position == [367.0, 49.0]));
    assert!(last_column.iter().all(|line| line.size == [1.0, 1.0]));
    assert!(native_viewport_outline_instances(
        (0.0, 0.0),
        screen,
        NativeRadarRect { x: 152, y: 0, w: 1, h: 1 },
        sidebar,
        [1.0; 3],
    )
    .is_empty());

    let last_row = native_viewport_outline_instances(
        (0.0, 0.0),
        screen,
        NativeRadarRect { x: 0, y: 250, w: 1, h: 1 },
        sidebar,
        [1.0; 3],
    );
    assert_eq!(last_row.len(), 4);
    assert!(last_row.iter().all(|line| line.position == [216.0, 299.0]));
    assert!(native_viewport_outline_instances(
        (0.0, 0.0),
        screen,
        NativeRadarRect { x: 0, y: 251, w: 1, h: 1 },
        sidebar,
        [1.0; 3],
    )
    .is_empty());
}

#[test]
fn native_viewport_outline_preserves_thin_zero_and_fully_outside_contracts() {
    let surface = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 140, 108).unwrap();
    let screen = NativeRadarScreenGeometry::new(surface, [16.0, 49.0, 140.0, 108.0]);
    let sidebar = [0.0, 0.0, 168.0, 300.0];

    let one_pixel = native_viewport_outline_instances(
        (0.0, 0.0),
        screen,
        NativeRadarRect { x: 10, y: 10, w: 1, h: 1 },
        sidebar,
        [1.0; 3],
    );
    assert_eq!(one_pixel.len(), 4, "native calls all four line slots");
    assert!(one_pixel.iter().all(|line| line.position == [26.0, 59.0]));
    assert!(one_pixel.iter().all(|line| line.size == [1.0, 1.0]));

    let vertical = native_viewport_outline_instances(
        (0.0, 0.0),
        screen,
        NativeRadarRect { x: 10, y: 10, w: 1, h: 3 },
        sidebar,
        [1.0; 3],
    );
    assert_eq!(vertical[0].size, [1.0, 1.0]);
    assert_eq!(vertical[1].size, [1.0, 3.0]);
    assert_eq!(vertical[2].position, [26.0, 61.0]);
    assert_eq!(vertical[3].size, [1.0, 3.0]);

    // The rectangle worker performs `right=x+w-1` / `bottom=y+h-1`
    // without rejecting zero extents, yielding the four sides of a reversed
    // 2x2 inclusive box.
    let zero = native_viewport_outline_instances(
        (0.0, 0.0),
        screen,
        NativeRadarRect { x: 10, y: 10, w: 0, h: 0 },
        sidebar,
        [1.0; 3],
    );
    assert_eq!(zero.len(), 4);
    assert_eq!(zero[0].position, [25.0, 59.0]);
    assert_eq!(zero[0].size, [2.0, 1.0]);
    assert_eq!(zero[1].position, [25.0, 58.0]);
    assert_eq!(zero[1].size, [1.0, 2.0]);
    assert_eq!(zero[2].position, [25.0, 58.0]);
    assert_eq!(zero[2].size, [2.0, 1.0]);
    assert_eq!(zero[3].position, [26.0, 58.0]);
    assert_eq!(zero[3].size, [1.0, 2.0]);

    assert!(native_viewport_outline_instances(
        (0.0, 0.0),
        screen,
        NativeRadarRect { x: -500, y: -500, w: 4, h: 4 },
        sidebar,
        [1.0; 3],
    )
    .is_empty());
}

#[test]
fn native_viewport_outline_scales_then_clips_without_sidebar_bleed() {
    let surface = NativeRadarSurfaceGeometry::from_raw_rect(0, 0, 140, 108).unwrap();
    let screen = NativeRadarScreenGeometry::new(surface, [208.0, 24.5, 70.0, 54.0]);
    let sidebar = [200.0, 0.0, 84.0, 150.0];
    let lines = native_viewport_outline_instances(
        (13.0, 17.0),
        screen,
        NativeRadarRect { x: -20, y: -50, w: 200, h: 200 },
        sidebar,
        [1.0; 3],
    );
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].position, [213.0, 116.0]);
    assert_eq!(lines[0].size, [84.0, 0.5]);

    for line in lines {
        let left = line.position[0] - 13.0;
        let top = line.position[1] - 17.0;
        assert!(left >= sidebar[0] && top >= sidebar[1]);
        assert!(left + line.size[0] <= sidebar[0] + sidebar[2]);
        assert!(top + line.size[1] <= sidebar[1] + sidebar[3]);
    }
}
