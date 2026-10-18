use gpui_xyflow::core::*;

fn approx(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-6,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn clamp_limits_values() {
    assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
    assert_eq!(clamp(-5.0, 0.0, 10.0), 0.0);
    assert_eq!(clamp(50.0, 0.0, 10.0), 10.0);
}

#[test]
fn clamp_position_accounts_for_dimensions() {
    let clamped = clamp_position(
        XYPosition::new(500.0, -20.0),
        [[0.0, 0.0], [300.0, 300.0]],
        Dimensions::new(100.0, 50.0),
    );
    assert_eq!(clamped, XYPosition::new(200.0, 0.0));
}

#[test]
fn rect_box_roundtrip() {
    let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
    let bounds = rect_to_box(rect);
    assert_eq!(bounds, Box2::new(10.0, 20.0, 40.0, 60.0));
    assert_eq!(box_to_rect(bounds), rect);
}

#[test]
fn overlapping_area_is_ceiled() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    let b = Rect::new(5.0, 5.0, 10.0, 10.0);
    assert_eq!(get_overlapping_area(a, b), 25.0);

    let disjoint = Rect::new(100.0, 100.0, 10.0, 10.0);
    assert_eq!(get_overlapping_area(a, disjoint), 0.0);
}

#[test]
fn snap_position_rounds_to_grid() {
    assert_eq!(
        snap_position(XYPosition::new(22.0, 38.0), [15.0, 15.0]),
        XYPosition::new(15.0, 45.0)
    );
    assert_eq!(
        snap_position(XYPosition::new(23.0, 7.0), [15.0, 15.0]),
        XYPosition::new(30.0, 0.0)
    );
}

#[test]
fn transform_apply_and_invert_are_inverse() {
    let transform = Transform::new(120.0, -40.0, 1.7);
    let point = XYPosition::new(33.0, 77.0);
    let inverted = transform.invert(transform.apply(point));
    approx(inverted.x, point.x);
    approx(inverted.y, point.y);
}

#[test]
fn point_to_renderer_point_respects_snap() {
    let transform = Transform::new(100.0, 50.0, 2.0);
    let result = point_to_renderer_point(
        XYPosition::new(200.0, 150.0),
        transform,
        false,
        [15.0, 15.0],
    );
    assert_eq!(result, XYPosition::new(50.0, 50.0));

    let snapped =
        point_to_renderer_point(XYPosition::new(200.0, 150.0), transform, true, [15.0, 15.0]);
    assert_eq!(snapped, XYPosition::new(45.0, 45.0));
}

#[test]
fn renderer_point_to_point_scales_and_translates() {
    let transform = Transform::new(10.0, 20.0, 2.0);
    assert_eq!(
        renderer_point_to_point(XYPosition::new(5.0, 5.0), transform),
        XYPosition::new(20.0, 30.0)
    );
}

#[test]
fn viewport_for_bounds_pads_and_centers() {
    let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
    let viewport = get_viewport_for_bounds(bounds, 800.0, 600.0, 0.5, 2.0, Padding::all(0.0));

    approx(viewport.zoom, 2.0);
    approx(viewport.x, 300.0);
    approx(viewport.y, 200.0);
}

#[test]
fn viewport_for_bounds_respects_min_zoom() {
    let bounds = Rect::new(0.0, 0.0, 100000.0, 100000.0);
    let viewport = get_viewport_for_bounds(bounds, 800.0, 600.0, 0.5, 2.0, Padding::all(0.0));
    approx(viewport.zoom, 0.5);
}

#[test]
fn viewport_for_bounds_padding_number_matches_flow_formula() {
    let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
    let viewport = get_viewport_for_bounds(bounds, 1000.0, 1000.0, 0.1, 4.0, Padding::all(0.1));
    approx(viewport.zoom, 4.0);
}

#[test]
fn viewport_for_bounds_px_padding_is_applied() {
    let bounds = Rect::new(0.0, 0.0, 400.0, 400.0);
    let viewport = get_viewport_for_bounds(
        bounds,
        500.0,
        500.0,
        0.1,
        4.0,
        Padding::all(PaddingWithUnit::Px(50.0)),
    );

    approx(viewport.zoom, 1.0);
    approx(viewport.x, 50.0);
    approx(viewport.y, 50.0);
}

#[test]
fn bezier_path_matches_flow_control_points() {
    let graphics = get_bezier_path(
        0.0,
        0.0,
        Position::Bottom,
        100.0,
        200.0,
        Position::Top,
        BezierPathOptions::default(),
    );

    assert_eq!(graphics.path, "M0,0 C0,100 100,100 100,200");
    approx(graphics.label_x, 50.0);
    approx(graphics.label_y, 100.0);
    approx(graphics.offset_x, 50.0);
    approx(graphics.offset_y, 100.0);
}

#[test]
fn bezier_path_uses_curvature_when_backwards() {
    let graphics = get_bezier_path(
        0.0,
        100.0,
        Position::Right,
        -100.0,
        100.0,
        Position::Left,
        BezierPathOptions::default(),
    );
    assert!(graphics.path.starts_with("M0,100 C"));
    assert!(!graphics.path.contains("C-50,100"));
}

#[test]
fn straight_path_is_direct_line() {
    let graphics = get_straight_path(0.0, 20.0, 150.0, 100.0);
    assert_eq!(graphics.path, "M 0,20L 150,100");
    approx(graphics.label_x, 75.0);
    approx(graphics.label_y, 60.0);
    approx(graphics.offset_x, 75.0);
    approx(graphics.offset_y, 40.0);
}

#[test]
fn smooth_step_path_produces_bends() {
    let graphics = get_smooth_step_path(
        0.0,
        0.0,
        Position::Bottom,
        100.0,
        200.0,
        Position::Top,
        SmoothStepPathOptions::default(),
        None,
    );

    assert!(graphics.path.starts_with('M'));
    assert!(graphics.path.contains('Q'));
}

#[test]
fn smooth_step_path_with_zero_radius_is_corner() {
    let graphics = get_smooth_step_path(
        0.0,
        0.0,
        Position::Right,
        100.0,
        100.0,
        Position::Left,
        SmoothStepPathOptions {
            border_radius: 0.0,
            offset: 20.0,
            step_position: 0.5,
        },
        None,
    );

    assert_eq!(
        graphics.path,
        "M0 0L20 0L 50,0Q 50,0 50,0L 50,100Q 50,100 50,100L80 100L100 100"
    );
}

#[test]
fn step_path_has_no_rounded_corners() {
    let graphics = get_step_path(
        0.0,
        0.0,
        Position::Bottom,
        0.0,
        100.0,
        Position::Top,
        StepPathOptions::default(),
    );
    assert!(!graphics.path.contains('Q'));
    assert!(graphics.path.contains('L'));
}

#[test]
fn edge_path_dispatch_selects_by_type() {
    let straight = get_edge_path(
        "straight",
        0.0,
        0.0,
        Position::Bottom,
        10.0,
        10.0,
        Position::Top,
        None,
    );
    assert_eq!(straight.path, "M 0,0L 10,10");

    let bezier = get_edge_path(
        "default",
        0.0,
        0.0,
        Position::Bottom,
        10.0,
        10.0,
        Position::Top,
        None,
    );
    assert!(bezier.path.starts_with("M0,0 C"));

    let stepped = get_edge_path(
        "step",
        0.0,
        0.0,
        Position::Bottom,
        0.0,
        100.0,
        Position::Top,
        None,
    );
    assert!(!stepped.path.contains('Q'));
}

#[test]
fn simple_bezier_path_uses_horizontal_controls() {
    let graphics = get_simple_bezier_path(
        0.0,
        0.0,
        100.0,
        100.0,
        BezierPathOptions { curvature: 0.25 },
    );
    assert!(graphics.path.starts_with("M0,0 C"));
}

#[test]
fn edge_center_handles_reverse_direction() {
    let (cx, cy, ox, oy) = get_edge_center(100.0, 100.0, 0.0, 0.0);
    approx(cx, 50.0);
    approx(cy, 50.0);
    approx(ox, 50.0);
    approx(oy, 50.0);
}

#[test]
fn opposite_position_flips_axis() {
    assert_eq!(Position::Left.opposite(), Position::Right);
    assert_eq!(Position::Top.opposite(), Position::Bottom);
    assert_eq!(Position::Right.direction(), XYPosition::new(1.0, 0.0));
}

#[test]
fn auto_pan_velocity_is_zero_inside_dead_zone() {
    let pan = calc_auto_pan(
        XYPosition::new(200.0, 200.0),
        Dimensions::new(400.0, 400.0),
        15.0,
        40.0,
    );
    assert_eq!(pan, XYPosition::ZERO);
}

#[test]
fn auto_pan_pushes_away_from_edges() {
    let left = calc_auto_pan(
        XYPosition::new(10.0, 200.0),
        Dimensions::new(400.0, 400.0),
        15.0,
        40.0,
    );
    assert!(left.x > 0.0);
    assert_eq!(left.y, 0.0);

    let right = calc_auto_pan(
        XYPosition::new(395.0, 200.0),
        Dimensions::new(400.0, 400.0),
        15.0,
        40.0,
    );
    assert!(right.x < 0.0);
}

#[test]
fn parse_padding_supports_units() {
    assert_eq!(
        parse_paddings(Padding::all(PaddingWithUnit::Px(20.0)), 1000.0, 800.0),
        ResolvedPadding {
            top: 20.0,
            right: 20.0,
            bottom: 20.0,
            left: 20.0,
            x: 40.0,
            y: 40.0,
        }
    );

    assert_eq!(
        parse_paddings(Padding::all(PaddingWithUnit::Percent(10.0)), 1000.0, 800.0),
        ResolvedPadding {
            top: 80.0,
            right: 100.0,
            bottom: 80.0,
            left: 100.0,
            x: 200.0,
            y: 160.0,
        }
    );
}

#[test]
fn parse_padding_number_uses_fraction_formula() {
    let resolved = parse_paddings(Padding::all(0.2), 1000.0, 500.0);
    assert_eq!(resolved.left, 83.0);
    assert_eq!(resolved.right, 83.0);
    assert_eq!(resolved.top, 41.0);
    assert_eq!(resolved.bottom, 41.0);
}

#[test]
fn padding_with_unit_parses_strings() {
    assert_eq!(
        PaddingWithUnit::parse("10px"),
        Some(PaddingWithUnit::Px(10.0))
    );
    assert_eq!(
        PaddingWithUnit::parse("25%"),
        Some(PaddingWithUnit::Percent(25.0))
    );
    assert_eq!(
        PaddingWithUnit::parse("0.1"),
        Some(PaddingWithUnit::Number(0.1))
    );
    assert_eq!(PaddingWithUnit::parse("abc"), None);
}
