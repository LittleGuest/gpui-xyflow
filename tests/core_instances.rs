use gpui_xyflow::core::{
    geometry::{CoordinateExtent, PanOnScrollMode, Transform, Viewport, XYPosition},
    panzoom::{PanOnDrag, PanZoomOptions},
    xyminimap::{MinimapPhase, MinimapUpdate, XYMinimap},
    xypanzoom::{
        eventhandler::ZoomEvent,
        filter::PointerEvent,
        instance::{PanZoomUpdateOptions, XYPanZoom},
    },
};

fn extent() -> CoordinateExtent {
    [[0.0, 0.0], [800.0, 600.0]]
}

fn instance() -> XYPanZoom {
    XYPanZoom::new(
        Viewport::new(0.0, 0.0, 1.0),
        PanZoomOptions::default(),
        extent(),
    )
}

#[test]
fn xypanzoom_starts_with_viewport() {
    let pan_zoom = instance();
    let viewport = pan_zoom.get_viewport();
    assert_eq!(viewport.zoom, 1.0);
}

#[test]
fn xypanzoom_update_returns_handlers() {
    let mut pan_zoom = instance();
    let options = PanZoomUpdateOptions::default();
    let handlers = pan_zoom.update(&options);
    assert!(!handlers.is_pan_on_scroll);
    assert!(handlers.zoom_on_scroll.is_some());
    assert!(handlers.pan_on_scroll.is_none());
    assert!(handlers.zoom_on_double_click);
}

#[test]
fn xypanzoom_pan_on_scroll_swaps_handlers() {
    let mut pan_zoom = instance();
    let options = PanZoomUpdateOptions {
        pan_on_scroll: true,
        ..PanZoomUpdateOptions::default()
    };
    let handlers = pan_zoom.update(&options);
    assert!(handlers.is_pan_on_scroll);
    assert!(handlers.pan_on_scroll.is_some());
    assert!(handlers.zoom_on_scroll.is_none());
}

#[test]
fn xypanzoom_pan_on_scroll_disabled_by_zoom_key() {
    let mut pan_zoom = instance();
    let options = PanZoomUpdateOptions {
        pan_on_scroll: true,
        zoom_activation_key_pressed: true,
        ..PanZoomUpdateOptions::default()
    };
    let handlers = pan_zoom.update(&options);
    assert!(!handlers.is_pan_on_scroll);
}

#[test]
fn xypanzoom_selection_on_drag_sets_infinite_click_distance() {
    let mut pan_zoom = instance();
    let options = PanZoomUpdateOptions {
        selection_on_drag: true,
        ..PanZoomUpdateOptions::default()
    };
    pan_zoom.update(&options);
    assert!(pan_zoom.click_distance().is_infinite());
}

#[test]
fn xypanzoom_negative_click_distance_becomes_zero() {
    let mut pan_zoom = instance();
    let options = PanZoomUpdateOptions {
        pane_click_distance: -5.0,
        ..PanZoomUpdateOptions::default()
    };
    pan_zoom.update(&options);
    assert_eq!(pan_zoom.click_distance(), 0.0);
}

#[test]
fn xypanzoom_destroy_blocks_events() {
    let mut pan_zoom = instance();
    pan_zoom.update(&PanZoomUpdateOptions::default());
    pan_zoom.destroy();
    assert!(pan_zoom.destroyed());
    assert!(!pan_zoom.accepts(&PointerEvent::wheel(-1.0)));
    let output = pan_zoom.dispatch(&ZoomEvent::from_transform(Transform::IDENTITY));
    assert!(output.pan_zoom.is_none());
}

#[test]
fn xypanzoom_update_after_user_selection_destroys() {
    let mut pan_zoom = instance();
    let options = PanZoomUpdateOptions {
        user_selection_active: true,
        ..PanZoomUpdateOptions::default()
    };
    pan_zoom.update(&options);
    assert!(pan_zoom.destroyed());
}

#[test]
fn xypanzoom_start_records_viewport() {
    let mut pan_zoom = instance();
    pan_zoom.update(&PanZoomUpdateOptions::default());
    let event = ZoomEvent::from_transform(Transform::new(10.0, 20.0, 1.5))
        .with_source(PointerEvent::mouse_down(0));
    let result = pan_zoom.start(&event);
    assert!(result.applied);
    assert!(pan_zoom.is_zooming_or_panning());
    assert_eq!(pan_zoom.get_viewport().x, 10.0);
}

#[test]
fn xypanzoom_end_clears_state() {
    let mut pan_zoom = instance();
    pan_zoom.update(&PanZoomUpdateOptions::default());
    pan_zoom.start(&ZoomEvent::from_transform(Transform::new(1.0, 1.0, 1.0)));
    assert!(pan_zoom.is_zooming_or_panning());
    pan_zoom.end(&ZoomEvent::from_transform(Transform::new(1.0, 1.0, 1.0)));
    assert!(!pan_zoom.is_zooming_or_panning());
}

#[test]
fn xypanzoom_instant_set_viewport() {
    let mut pan_zoom = instance();
    let result = pan_zoom.set_viewport(Viewport::new(50.0, 60.0, 1.0), 0.0);
    assert_eq!(result.x, 50.0);
    assert!(!pan_zoom.advancing());
    assert_eq!(pan_zoom.get_viewport().x, 50.0);
}

#[test]
fn xypanzoom_animated_set_viewport_advances() {
    let mut pan_zoom = instance();
    pan_zoom.set_viewport(Viewport::new(100.0, 0.0, 1.0), 100.0);
    assert!(pan_zoom.advancing());
    let mid = pan_zoom.tick(50.0).unwrap();
    assert!(mid.x > 0.0 && mid.x < 100.0);
    let end = pan_zoom.finish_transition().unwrap();
    assert_eq!(end.x, 100.0);
    assert!(!pan_zoom.advancing());
}

#[test]
fn xypanzoom_scale_to_instant() {
    let mut pan_zoom = instance();
    let zoom = pan_zoom.scale_to(1.5, 0.0);
    assert_eq!(zoom, 1.5);
    assert_eq!(pan_zoom.get_viewport().zoom, 1.5);
}

#[test]
fn xypanzoom_scale_by_clamps_to_extent() {
    let mut pan_zoom = instance();
    pan_zoom.set_scale_extent(0.5, 2.0);
    pan_zoom.scale_by(100.0, 0.0);
    assert!(pan_zoom.get_viewport().zoom <= 2.0);
}

#[test]
fn xypanzoom_sync_viewport_noop_when_equal() {
    let mut pan_zoom = instance();
    let before = pan_zoom.get_viewport();
    pan_zoom.sync_viewport(before);
    assert_eq!(pan_zoom.get_viewport(), before);
}

#[test]
fn xypanzoom_sync_viewport_applies() {
    let mut pan_zoom = instance();
    pan_zoom.sync_viewport(Viewport::new(7.0, 8.0, 1.2));
    let after = pan_zoom.get_viewport();
    assert_eq!(after.x, 7.0);
    assert_eq!(after.y, 8.0);
}

#[test]
fn xypanzoom_wheel_scrolls_when_pan_on_scroll() {
    let mut pan_zoom = instance();
    let options = PanZoomUpdateOptions {
        pan_on_scroll: true,
        pan_on_scroll_mode: PanOnScrollMode::Free,
        ..PanZoomUpdateOptions::default()
    };
    pan_zoom.update(&options);
    let before = pan_zoom.get_viewport();
    let outcome = pan_zoom.wheel(&PointerEvent::wheel(-100.0), 0.0);
    assert!(outcome.pan_scroll.is_some());
    assert!(pan_zoom.get_viewport().y > before.y);
}

#[test]
fn xypanzoom_wheel_rejected_over_nopan() {
    let mut pan_zoom = instance();
    let options = PanZoomUpdateOptions {
        pan_on_scroll: true,
        ..PanZoomUpdateOptions::default()
    };
    pan_zoom.update(&options);
    let before = pan_zoom.get_viewport();
    let outcome = pan_zoom.wheel(&PointerEvent::wheel(-100.0).with_class("nopan"), 0.0);
    assert!(!outcome.outcome.allowed());
    assert_eq!(pan_zoom.get_viewport(), before);
}

#[test]
fn xypanzoom_right_click_pan_detection() {
    let mut pan_zoom = instance();
    let options = PanZoomUpdateOptions {
        pan_on_drag: PanOnDrag::Buttons(vec![0, 2]),
        ..PanZoomUpdateOptions::default()
    };
    pan_zoom.update(&options);
    assert!(pan_zoom.right_click_pan(2));
    assert!(!pan_zoom.right_click_pan(0));
}

#[test]
fn minimap_update_restores_after_destroy() {
    let mut minimap = XYMinimap::new();
    minimap.destroy();
    assert!(minimap.destroyed());
    minimap.update(MinimapUpdate::default());
    assert!(!minimap.destroyed());
}

#[test]
fn minimap_wheel_zooms_viewport() {
    let mut minimap = XYMinimap::new();
    minimap.set_scale_extent(0.1, 4.0);
    let viewport = Viewport::new(0.0, 0.0, 1.0);
    let next = minimap.wheel(viewport, -100.0, false, false).unwrap();
    assert!(next.zoom > 1.0);
}

#[test]
fn minimap_wheel_respects_scale_extent() {
    let mut minimap = XYMinimap::new();
    minimap.set_scale_extent(0.5, 2.0);
    let next = minimap
        .wheel(Viewport::new(0.0, 0.0, 2.0), -10000.0, false, false)
        .unwrap();
    assert!(next.zoom <= 2.0);
}

#[test]
fn minimap_wheel_disabled_when_not_zoomable() {
    let mut minimap = XYMinimap::new();
    let update = MinimapUpdate {
        zoomable: false,
        ..MinimapUpdate::default()
    };
    minimap.update(update);
    assert!(
        minimap
            .wheel(Viewport::IDENTITY, -100.0, false, false)
            .is_none()
    );
}

#[test]
fn minimap_pan_lifecycle() {
    let mut minimap = XYMinimap::new();
    minimap.update(MinimapUpdate::default());
    minimap.pan_start(XYPosition::new(10.0, 10.0));
    assert!(minimap.is_panning());
    let result = minimap
        .pan_move(
            Viewport::new(0.0, 0.0, 1.0),
            XYPosition::new(20.0, 10.0),
            0.5,
        )
        .unwrap();
    assert_eq!(result.delta, XYPosition::new(10.0, 0.0));
    assert!(result.viewport.x < 0.0);
    minimap.pan_end();
    assert_eq!(minimap.phase(), MinimapPhase::Idle);
    assert!(
        minimap
            .pan_move(Viewport::IDENTITY, XYPosition::new(0.0, 0.0), 1.0)
            .is_none()
    );
}

#[test]
fn minimap_pan_disabled_when_not_pannable() {
    let mut minimap = XYMinimap::new();
    let update = MinimapUpdate {
        pannable: false,
        ..MinimapUpdate::default()
    };
    minimap.update(update);
    minimap.pan_start(XYPosition::new(0.0, 0.0));
    assert!(!minimap.is_panning());
}

#[test]
fn minimap_inverse_pan_flips_direction() {
    let mut minimap = XYMinimap::new();
    let update = MinimapUpdate {
        inverse_pan: true,
        ..MinimapUpdate::default()
    };
    minimap.update(update);
    minimap.pan_start(XYPosition::new(0.0, 0.0));
    let result = minimap
        .pan_move(
            Viewport::new(0.0, 0.0, 1.0),
            XYPosition::new(10.0, 0.0),
            0.5,
        )
        .unwrap();
    assert!(result.viewport.x > 0.0);
}
