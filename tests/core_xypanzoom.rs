use gpui_xyflow::core::{
    geometry::{PanOnScrollMode, Rect, Transform, Viewport},
    panzoom::{PanOnDrag, transition_ease_cubic_in_out},
    xypanzoom::{
        eventhandler::{
            PAN_SCROLL_END_DELAY_MS, PanOnScrollParams, PanScrollTimeoutResult, PanZoomEndParams,
            PanZoomParams, ZoomEvent, ZoomPanValues, create_pan_on_scroll_handler,
            create_pan_zoom_end_handler, create_pan_zoom_handler, create_pan_zoom_start_handler,
            poll_pan_scroll_timeout,
        },
        filter::{FilterOutcome, FilterParams, PointerEvent, PointerEventKind, create_filter},
        utils::{
            Ease, ViewportTransition, default_ease, interpolate_linear, interpolate_zoom,
            is_right_click_pan, is_wrapped_with_class, minimize_transform, wheel_delta,
        },
    },
};

fn params() -> FilterParams {
    FilterParams::default()
}

#[test]
fn filter_allows_plain_wheel() {
    let event = PointerEvent::wheel(-100.0);
    assert_eq!(create_filter(&params(), &event), FilterOutcome::Allow);
}

#[test]
fn filter_rejects_when_everything_disabled() {
    let mut p = params();
    p.pan_on_drag = PanOnDrag::Disabled;
    p.zoom_on_scroll = false;
    p.pan_on_scroll = false;
    p.zoom_on_pinch = false;
    p.zoom_on_double_click = false;
    assert_eq!(
        create_filter(&p, &PointerEvent::wheel(-1.0)),
        FilterOutcome::Reject
    );
    assert_eq!(
        create_filter(&p, &PointerEvent::mouse_down(0)),
        FilterOutcome::Reject
    );
}

#[test]
fn filter_rejects_during_user_selection() {
    let mut p = params();
    p.user_selection_active = true;
    assert_eq!(
        create_filter(&p, &PointerEvent::wheel(-1.0)),
        FilterOutcome::Reject
    );
}

#[test]
fn filter_rejects_mouse_during_connection_but_allows_wheel() {
    let mut p = params();
    p.connection_in_progress = true;
    assert_eq!(
        create_filter(&p, &PointerEvent::mouse_down(0)),
        FilterOutcome::Reject
    );
    assert_eq!(
        create_filter(&p, &PointerEvent::wheel(-1.0)),
        FilterOutcome::Allow
    );
}

#[test]
fn filter_rejects_wheel_over_nowheel() {
    let event = PointerEvent::wheel(-1.0).with_class("nowheel");
    assert_eq!(create_filter(&params(), &event), FilterOutcome::Reject);
}

#[test]
fn filter_keeps_mousedown_over_nowheel() {
    let event = PointerEvent::mouse_down(0).with_class("nowheel");
    assert_eq!(create_filter(&params(), &event), FilterOutcome::Allow);
}

#[test]
fn filter_rejects_pan_over_nopan() {
    let event = PointerEvent::mouse_down(0).with_class("nopan");
    assert_eq!(create_filter(&params(), &event), FilterOutcome::Reject);
}

#[test]
fn filter_allows_wheel_over_nopan_when_not_panning_on_scroll() {
    let event = PointerEvent::wheel(-1.0).with_class("nopan");
    let mut p = params();
    p.pan_on_scroll = false;
    assert_eq!(create_filter(&p, &event), FilterOutcome::Allow);
}

#[test]
fn filter_rejects_wheel_over_nopan_when_panning_on_scroll() {
    let event = PointerEvent::wheel(-1.0).with_class("nopan");
    let mut p = params();
    p.pan_on_scroll = true;
    assert_eq!(create_filter(&p, &event), FilterOutcome::Reject);
}

#[test]
fn filter_allows_wheel_over_nopan_with_zoom_activation_key() {
    let event = PointerEvent::wheel(-1.0).with_class("nopan").with_ctrl();
    let mut p = params();
    p.pan_on_scroll = true;
    p.zoom_activation_key_pressed = true;
    assert_eq!(create_filter(&p, &event), FilterOutcome::Allow);
}

#[test]
fn filter_rejects_ctrl_wheel_when_pinch_disabled() {
    let event = PointerEvent::wheel(-1.0).with_ctrl();
    let mut p = params();
    p.zoom_on_pinch = false;
    assert_eq!(create_filter(&p, &event), FilterOutcome::Reject);
}

#[test]
fn filter_rejects_multitouch_when_pinch_disabled() {
    let event = PointerEvent::touch_start(0).with_touches(2);
    let mut p = params();
    p.zoom_on_pinch = false;
    assert_eq!(
        create_filter(&p, &event),
        FilterOutcome::RejectAndPreventDefault
    );
}

#[test]
fn filter_allows_fullscreen_middle_click_on_node() {
    let event = PointerEvent::mouse_down(1)
        .with_class("gpui-flow__node")
        .with_class("nowheel")
        .with_class("nopan");
    let mut p = params();
    p.user_selection_active = true;
    assert_eq!(create_filter(&p, &event), FilterOutcome::Allow);
}

#[test]
fn filter_rejects_mouse_when_pan_on_drag_disabled() {
    let mut p = params();
    p.pan_on_drag = PanOnDrag::Disabled;
    assert_eq!(
        create_filter(&p, &PointerEvent::mouse_down(0)),
        FilterOutcome::Reject
    );
}

#[test]
fn filter_rejects_disallowed_button() {
    let mut p = params();
    p.pan_on_drag = PanOnDrag::Buttons(vec![2]);
    assert_eq!(
        create_filter(&p, &PointerEvent::mouse_down(2)),
        FilterOutcome::Allow
    );
    assert_eq!(
        create_filter(&p, &PointerEvent::mouse_down(0)),
        FilterOutcome::Reject
    );
}

#[test]
fn filter_rejects_ctrl_drag_when_not_pan_key() {
    let event = PointerEvent::mouse_down(0).with_ctrl();
    assert_eq!(create_filter(&params(), &event), FilterOutcome::Reject);
}

#[test]
fn filter_allows_ctrl_drag_with_pan_activation_key() {
    let event = PointerEvent::mouse_down(0).with_ctrl();
    let mut p = params();
    p.pan_activation_key_pressed = true;
    assert_eq!(create_filter(&p, &event), FilterOutcome::Allow);
}

#[test]
fn filter_rejects_wheel_when_no_scroll_handling() {
    let mut p = params();
    p.zoom_on_scroll = false;
    p.pan_on_scroll = false;
    p.zoom_on_pinch = true;
    assert_eq!(
        create_filter(&p, &PointerEvent::wheel(-1.0)),
        FilterOutcome::Reject
    );
}

#[test]
fn wrapper_class_detects_substring() {
    let classes = vec!["gpui-flow__node".to_string()];
    assert!(is_wrapped_with_class(&classes, "gpui-flow__node"));
    assert!(!is_wrapped_with_class(&classes, "nowheel"));
    assert!(!is_wrapped_with_class(&classes, ""));
}

#[test]
fn right_click_pan_requires_button_list() {
    assert!(is_right_click_pan(&PanOnDrag::Buttons(vec![2]), 2));
    assert!(!is_right_click_pan(&PanOnDrag::Enabled, 2));
    assert!(!is_right_click_pan(&PanOnDrag::Buttons(vec![2]), 0));
}

#[test]
fn wheel_delta_matches_d3_formula() {
    assert!((wheel_delta(-100.0, 0, false, false) - 0.2).abs() < 1e-9);
    assert!((wheel_delta(-100.0, 1, false, false) - 5.0).abs() < 1e-9);
    assert!((wheel_delta(-100.0, 2, false, false) - 100.0).abs() < 1e-9);
    assert!((wheel_delta(-100.0, 0, true, true) - 2.0).abs() < 1e-9);
    assert!((wheel_delta(-100.0, 0, true, false) - 0.2).abs() < 1e-9);
}

#[test]
fn default_ease_matches_d3_cubic() {
    assert!((default_ease(0.0)).abs() < 1e-9);
    assert!((default_ease(0.5) - 0.5).abs() < 1e-9);
    assert!((default_ease(1.0) - 1.0).abs() < 1e-9);
    assert!((default_ease(0.25) - 0.0625).abs() < 1e-9);
    assert!((default_ease(0.75) - 0.9375).abs() < 1e-9);
}

#[test]
fn ease_is_monotonic() {
    let mut previous = transition_ease_cubic_in_out(0.0);
    for step in 1..=100 {
        let value = transition_ease_cubic_in_out(step as f64 / 100.0);
        assert!(value >= previous - 1e-12);
        previous = value;
    }
}

#[test]
fn interpolate_linear_hits_endpoints() {
    let from = Viewport::new(0.0, 0.0, 1.0);
    let to = Viewport::new(100.0, 50.0, 2.0);
    assert_eq!(interpolate_linear(from, to, 0.0), from);
    assert_eq!(interpolate_linear(from, to, 1.0), to);
}

#[test]
fn interpolate_zoom_hits_endpoints() {
    let from = Viewport::new(0.0, 0.0, 1.0);
    let to = Viewport::new(100.0, 50.0, 2.0);
    let start = interpolate_zoom(from, to, 0.0);
    assert!(start.zoom.is_finite());
    let end = interpolate_zoom(from, to, 1.0);
    assert!((end.zoom - 2.0).abs() < 1e-9);
}

#[test]
fn transition_advances_to_target() {
    let from = Viewport::new(0.0, 0.0, 1.0);
    let to = Viewport::new(10.0, 20.0, 2.0);
    let mut transition = ViewportTransition::new(from, to, 100.0);
    assert!(transition.has_duration());
    transition.advance(50.0);
    assert!((transition.progress() - 0.5).abs() < 1e-9);
    let end = transition.advance(60.0);
    assert_eq!(end, to);
    assert!(transition.finished());
}

#[test]
fn transition_with_zero_duration_is_instant() {
    let from = Viewport::new(0.0, 0.0, 1.0);
    let to = Viewport::new(1.0, 1.0, 1.0);
    let transition = ViewportTransition::new(from, to, 0.0);
    assert!(!transition.has_duration());
    assert_eq!(transition.current(), to);
}

#[test]
fn transition_respects_linear_ease() {
    let from = Viewport::new(0.0, 0.0, 1.0);
    let to = Viewport::new(100.0, 0.0, 1.0);
    let mut transition = ViewportTransition::new(from, to, 100.0)
        .with_ease(Ease::Linear)
        .with_interpolate(gpui_xyflow::core::xypanzoom::utils::Interpolate::Linear);
    let mid = transition.advance(50.0);
    assert!((mid.x - 50.0).abs() < 1e-9);
}

#[test]
fn minimize_transform_clamps_into_translate_extent() {
    let transform = Transform::new(-1000.0, -1000.0, 1.0);
    let extent = Rect::new(0.0, 0.0, 800.0, 600.0);
    let translate = Rect::new(0.0, 0.0, 1000.0, 1000.0);
    let result = minimize_transform(transform, extent, translate);
    assert!(result.x() >= -1000.0);
    assert!(result.zoom() == 1.0);
}

#[test]
fn minimize_transform_zero_extent_passthrough() {
    let transform = Transform::new(5.0, 6.0, 1.0);
    let result = minimize_transform(
        transform,
        Rect::new(0.0, 0.0, 1.0, 1.0),
        Rect::new(0.0, 0.0, 0.0, 0.0),
    );
    assert_eq!(result, transform);
}

#[test]
fn pan_scroll_starts_then_continues() {
    let mut values = ZoomPanValues::default();
    let p = PanOnScrollParams::default();
    let event = PointerEvent::wheel(-10.0);
    let first = create_pan_on_scroll_handler(&p, &mut values, &event, 1.0, 0.0);
    assert!(first.start);
    assert!(!first.continued);
    assert!(values.is_pan_scrolling);
    let second = create_pan_on_scroll_handler(&p, &mut values, &event, 1.0, 10.0);
    assert!(!second.start);
    assert!(second.continued);
}

#[test]
fn pan_scroll_translation_uses_speed_and_zoom() {
    let mut values = ZoomPanValues::default();
    let p = PanOnScrollParams {
        pan_on_scroll_speed: 0.5,
        ..PanOnScrollParams::default()
    };
    let event = PointerEvent::wheel(-100.0);
    let result = create_pan_on_scroll_handler(&p, &mut values, &event, 2.0, 0.0);
    let (dx, dy) = result.translated.unwrap();
    assert!(dx.abs() < 1e-12);
    assert!((dy - 25.0).abs() < 1e-9);
}

#[test]
fn pan_scroll_vertical_mode_zeroes_delta_x() {
    let mut values = ZoomPanValues::default();
    let p = PanOnScrollParams {
        pan_on_scroll_mode: PanOnScrollMode::Vertical,
        ..PanOnScrollParams::default()
    };
    let mut event = PointerEvent::wheel(-10.0);
    event.delta_x = 50.0;
    let result = create_pan_on_scroll_handler(&p, &mut values, &event, 1.0, 0.0);
    let (dx, _) = result.translated.unwrap();
    assert_eq!(dx, 0.0);
}

#[test]
fn pan_scroll_horizontal_mode_zeroes_delta_y() {
    let mut values = ZoomPanValues::default();
    let p = PanOnScrollParams {
        pan_on_scroll_mode: PanOnScrollMode::Horizontal,
        ..PanOnScrollParams::default()
    };
    let result =
        create_pan_on_scroll_handler(&p, &mut values, &PointerEvent::wheel(-10.0), 1.0, 0.0);
    let (_, dy) = result.translated.unwrap();
    assert_eq!(dy, 0.0);
}

#[test]
fn pan_scroll_pinch_scales_instead_of_translating() {
    let mut values = ZoomPanValues::default();
    let p = PanOnScrollParams::default();
    let event = PointerEvent::wheel(-100.0).with_ctrl();
    let result = create_pan_on_scroll_handler(&p, &mut values, &event, 1.0, 0.0);
    assert!(result.scaled_to.is_some());
    assert!(result.translated.is_none());
    assert!(result.scaled_to.unwrap() > 1.0);
}

#[test]
fn pan_scroll_skips_nowheel() {
    let mut values = ZoomPanValues::default();
    let p = PanOnScrollParams::default();
    let event = PointerEvent::wheel(-10.0).with_class("nowheel");
    let result = create_pan_on_scroll_handler(&p, &mut values, &event, 1.0, 0.0);
    assert!(!result.handled);
    assert!(!values.is_pan_scrolling);
}

#[test]
fn pan_scroll_timeout_fires_after_delay() {
    let mut values = ZoomPanValues::default();
    let p = PanOnScrollParams::default();
    create_pan_on_scroll_handler(&p, &mut values, &PointerEvent::wheel(-10.0), 1.0, 0.0);
    let early = poll_pan_scroll_timeout(
        &mut values,
        PAN_SCROLL_END_DELAY_MS - 1.0,
        Viewport::IDENTITY,
    );
    assert_eq!(early, PanScrollTimeoutResult::Idle);
    let late = poll_pan_scroll_timeout(
        &mut values,
        PAN_SCROLL_END_DELAY_MS + 1.0,
        Viewport::IDENTITY,
    );
    assert!(matches!(late, PanScrollTimeoutResult::Fired { .. }));
    assert!(!values.is_pan_scrolling);
}

#[test]
fn pan_zoom_start_ignores_internal() {
    let mut values = ZoomPanValues::default();
    let event = ZoomEvent::from_transform(Transform::new(1.0, 2.0, 1.0)).internal();
    let result = create_pan_zoom_start_handler(&mut values, &event);
    assert!(!result.applied);
    assert!(!values.is_zooming_or_panning);
}

#[test]
fn pan_zoom_start_records_button_and_state() {
    let mut values = ZoomPanValues::default();
    let event = ZoomEvent::from_transform(Transform::new(3.0, 4.0, 2.0))
        .with_source(PointerEvent::mouse_down(2));
    let result = create_pan_zoom_start_handler(&mut values, &event);
    assert!(result.applied);
    assert_eq!(values.mouse_button, 2);
    assert!(values.is_zooming_or_panning);
    assert_eq!(values.prev_viewport, Viewport::new(3.0, 4.0, 2.0));
    assert!(result.dragging);
}

#[test]
fn pan_zoom_start_wheel_does_not_flag_dragging() {
    let mut values = ZoomPanValues::default();
    let event =
        ZoomEvent::from_transform(Transform::IDENTITY).with_source(PointerEvent::wheel(-1.0));
    let result = create_pan_zoom_start_handler(&mut values, &event);
    assert!(!result.dragging);
}

#[test]
fn pan_zoom_emits_transform_unless_sync() {
    let mut values = ZoomPanValues::default();
    let p = PanZoomParams::default();
    let event = ZoomEvent::from_transform(Transform::new(1.0, 1.0, 1.0));
    let result = create_pan_zoom_handler(&mut values, &p, &event);
    assert!(result.emit_transform_change);
    assert!(result.emit_pan_zoom);

    let event = ZoomEvent::from_transform(Transform::new(1.0, 1.0, 1.0)).sync();
    let result = create_pan_zoom_handler(&mut values, &p, &event);
    assert!(!result.emit_transform_change);

    let event = ZoomEvent::from_transform(Transform::new(1.0, 1.0, 1.0)).internal();
    let result = create_pan_zoom_handler(&mut values, &p, &event);
    assert!(!result.emit_pan_zoom);
}

#[test]
fn pan_zoom_end_fires_context_menu_for_unused_right_pan() {
    let mut values = ZoomPanValues {
        mouse_button: 2,
        used_right_mouse_button: false,
        ..ZoomPanValues::default()
    };
    let params = PanZoomEndParams {
        pan_on_drag: PanOnDrag::Buttons(vec![2]),
        pan_on_scroll: false,
        on_pane_context_menu: true,
    };
    let event = ZoomEvent::from_transform(Transform::IDENTITY);
    let result = create_pan_zoom_end_handler(&mut values, &params, &event);
    assert!(result.fire_pane_context_menu);
    assert!(!values.used_right_mouse_button);
}

#[test]
fn pan_zoom_end_skips_context_menu_after_actual_pan() {
    let mut values = ZoomPanValues {
        mouse_button: 2,
        used_right_mouse_button: true,
        ..ZoomPanValues::default()
    };
    let params = PanZoomEndParams {
        pan_on_drag: PanOnDrag::Buttons(vec![2]),
        pan_on_scroll: false,
        on_pane_context_menu: true,
    };
    let result = create_pan_zoom_end_handler(
        &mut values,
        &params,
        &ZoomEvent::from_transform(Transform::IDENTITY),
    );
    assert!(!result.fire_pane_context_menu);
}

#[test]
fn pan_zoom_end_clears_zooming_state() {
    let mut values = ZoomPanValues {
        is_zooming_or_panning: true,
        ..ZoomPanValues::default()
    };
    let params = PanZoomEndParams::default();
    create_pan_zoom_end_handler(
        &mut values,
        &params,
        &ZoomEvent::from_transform(Transform::IDENTITY),
    );
    assert!(!values.is_zooming_or_panning);
}

#[test]
fn pointer_event_builders_are_composable() {
    let event = PointerEvent::wheel(-5.0)
        .with_ctrl()
        .with_shift()
        .with_class("nopan");
    assert_eq!(event.kind, PointerEventKind::Wheel);
    assert!(event.ctrl_key);
    assert!(event.shift_key);
    assert_eq!(event.delta_y, -5.0);
    assert_eq!(event.classes.len(), 1);
}
