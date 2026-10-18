use gpui_xyflow::core::{
    geometry::XYPosition,
    xyresizer::{
        Boundaries, ResizeParams, ResizePhase, XYResizer, XYResizerOptions, get_control_direction,
        get_resize_direction,
    },
};

fn params(x: f64, y: f64, width: f64, height: f64) -> ResizeParams {
    ResizeParams {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn control_direction_right() {
    let control = get_control_direction("right");
    assert!(control.is_horizontal);
    assert!(!control.is_vertical);
    assert!(!control.affects_x);
    assert!(!control.affects_y);
}

#[test]
fn control_direction_top_left() {
    let control = get_control_direction("top-left");
    assert!(control.is_horizontal);
    assert!(control.is_vertical);
    assert!(control.affects_x);
    assert!(control.affects_y);
}

#[test]
fn resize_direction_signs() {
    let direction = get_resize_direction(120.0, 100.0, 90.0, 100.0, false, false);
    assert_eq!(direction, [1.0, -1.0]);
}

#[test]
fn resize_direction_inverts_with_affects() {
    let direction = get_resize_direction(120.0, 100.0, 100.0, 100.0, true, false);
    assert_eq!(direction, [-1.0, 0.0]);
}

#[test]
fn resizer_starts_and_reports_phase() {
    let mut resizer = XYResizer::new(XYResizerOptions::default());
    assert_eq!(resizer.phase(), ResizePhase::Idle);
    resizer.start(
        "bottom-right",
        params(0.0, 0.0, 100.0, 100.0),
        XYPosition::new(100.0, 100.0),
    );
    assert!(resizer.is_resizing());
    assert_eq!(resizer.control_position(), Some("bottom-right"));
}

#[test]
fn resizer_bottom_right_grows() {
    let mut resizer = XYResizer::new(XYResizerOptions::default());
    resizer.start(
        "bottom-right",
        params(0.0, 0.0, 100.0, 100.0),
        XYPosition::new(100.0, 100.0),
    );
    let change = resizer.resize(XYPosition::new(150.0, 140.0)).unwrap();
    assert!((change.width - 150.0).abs() < 1e-9);
    assert!((change.height - 140.0).abs() < 1e-9);
    assert_eq!(change.x, 0.0);
    assert_eq!(change.y, 0.0);
}

#[test]
fn resizer_top_left_moves_origin() {
    let mut resizer = XYResizer::new(XYResizerOptions::default());
    resizer.start(
        "top-left",
        params(0.0, 0.0, 100.0, 100.0),
        XYPosition::new(0.0, 0.0),
    );
    let change = resizer.resize(XYPosition::new(-20.0, -10.0)).unwrap();
    assert!((change.width - 120.0).abs() < 1e-9);
    assert!((change.height - 110.0).abs() < 1e-9);
    assert!((change.x + 20.0).abs() < 1e-9);
}

#[test]
fn resizer_respects_min_width() {
    let options = XYResizerOptions {
        boundaries: Boundaries::with_defaults(50.0, 50.0),
        ..XYResizerOptions::default()
    };
    let mut resizer = XYResizer::new(options);
    resizer.start(
        "bottom-right",
        params(0.0, 0.0, 100.0, 100.0),
        XYPosition::new(100.0, 100.0),
    );
    let change = resizer.resize(XYPosition::new(10.0, 10.0)).unwrap();
    assert!(change.width >= 50.0);
    assert!(change.height >= 50.0);
}

#[test]
fn resizer_respects_max_width() {
    let options = XYResizerOptions {
        boundaries: Boundaries::new(0.0, 0.0, 200.0, 200.0),
        ..XYResizerOptions::default()
    };
    let mut resizer = XYResizer::new(options);
    resizer.start(
        "bottom-right",
        params(0.0, 0.0, 100.0, 100.0),
        XYPosition::new(100.0, 100.0),
    );
    let change = resizer.resize(XYPosition::new(500.0, 500.0)).unwrap();
    assert!(change.width <= 200.0);
    assert!(change.height <= 200.0);
}

#[test]
fn resizer_keep_aspect_ratio() {
    let options = XYResizerOptions {
        keep_aspect_ratio: true,
        ..XYResizerOptions::default()
    };
    let mut resizer = XYResizer::new(options);
    resizer.start(
        "bottom-right",
        params(0.0, 0.0, 100.0, 50.0),
        XYPosition::new(100.0, 50.0),
    );
    let change = resizer.resize(XYPosition::new(200.0, 50.0)).unwrap();
    let ratio = change.width / change.height;
    assert!((ratio - 2.0).abs() < 1e-6);
}

#[test]
fn resizer_snap_to_grid() {
    let options = XYResizerOptions {
        snap_to_grid: true,
        snap_grid: [10.0, 10.0],
        ..XYResizerOptions::default()
    };
    let mut resizer = XYResizer::new(options);
    resizer.start(
        "bottom-right",
        params(0.0, 0.0, 0.0, 0.0),
        XYPosition::new(0.0, 0.0),
    );
    let change = resizer.resize(XYPosition::new(43.0, 47.0)).unwrap();
    assert!((change.width - 40.0).abs() < 1e-9);
    assert!((change.height - 50.0).abs() < 1e-9);
}

#[test]
fn resizer_end_returns_last_change() {
    let mut resizer = XYResizer::new(XYResizerOptions::default());
    resizer.start(
        "bottom-right",
        params(0.0, 0.0, 100.0, 100.0),
        XYPosition::new(100.0, 100.0),
    );
    resizer.resize(XYPosition::new(120.0, 120.0));
    let last = resizer.end().unwrap();
    assert!((last.width - 120.0).abs() < 1e-9);
    assert_eq!(resizer.phase(), ResizePhase::Idle);
    assert!(resizer.last().is_none());
}

#[test]
fn resizer_resize_without_start_is_none() {
    let mut resizer = XYResizer::new(XYResizerOptions::default());
    assert!(resizer.resize(XYPosition::new(10.0, 10.0)).is_none());
}

#[test]
fn resizer_cancel_resets() {
    let mut resizer = XYResizer::new(XYResizerOptions::default());
    resizer.start(
        "bottom-right",
        params(0.0, 0.0, 100.0, 100.0),
        XYPosition::new(100.0, 100.0),
    );
    resizer.cancel();
    assert_eq!(resizer.phase(), ResizePhase::Idle);
    assert!(resizer.control_position().is_none());
}

#[test]
fn resizer_handle_and_line_classification() {
    assert!(XYResizer::is_handle("top-left"));
    assert!(!XYResizer::is_handle("top"));
    assert!(XYResizer::is_line("top"));
    assert!(!XYResizer::is_line("top-left"));
}

#[test]
fn resizer_direction_reported() {
    let mut resizer = XYResizer::new(XYResizerOptions::default());
    resizer.start(
        "bottom-right",
        params(0.0, 0.0, 100.0, 100.0),
        XYPosition::new(100.0, 100.0),
    );
    let change = resizer.resize(XYPosition::new(80.0, 130.0)).unwrap();
    assert_eq!(change.direction, [-1.0, 1.0]);
}
