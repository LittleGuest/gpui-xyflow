use crate::core::{
    geometry::{PanOnScrollMode, Transform, Viewport},
    panzoom::PanOnDrag,
    xypanzoom::{
        filter::{PointerEvent, PointerEventKind},
        utils::{is_right_click_pan, is_wrapped_with_class, wheel_delta},
    },
};

pub const PAN_SCROLL_END_DELAY_MS: f64 = 150.0;

#[derive(Clone, Debug, PartialEq)]
pub struct ZoomPanValues {
    pub is_zooming_or_panning: bool,
    pub used_right_mouse_button: bool,
    pub prev_viewport: Viewport,
    pub mouse_button: u32,
    pub timer_id: Option<u64>,
    pub pan_scroll_timeout: Option<u64>,
    pub is_pan_scrolling: bool,
    pub pan_scroll_deadline_ms: Option<f64>,
}

impl Default for ZoomPanValues {
    fn default() -> Self {
        ZoomPanValues {
            is_zooming_or_panning: false,
            used_right_mouse_button: false,
            prev_viewport: Viewport {
                x: 0.0,
                y: 0.0,
                zoom: 0.0,
            },
            mouse_button: 0,
            timer_id: None,
            pan_scroll_timeout: None,
            is_pan_scrolling: false,
            pan_scroll_deadline_ms: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZoomEvent {
    pub source: PointerEvent,
    pub transform: Transform,
    pub internal: bool,
    pub sync: bool,
}

impl ZoomEvent {
    pub fn from_transform(transform: Transform) -> Self {
        ZoomEvent {
            source: PointerEvent::default(),
            transform,
            internal: false,
            sync: false,
        }
    }

    pub fn with_source(mut self, source: PointerEvent) -> Self {
        self.source = source;
        self
    }

    pub fn internal(mut self) -> Self {
        self.internal = true;
        self
    }

    pub fn sync(mut self) -> Self {
        self.sync = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PanScrollResult {
    pub handled: bool,
    pub prevented: bool,
    pub translated: Option<(f64, f64)>,
    pub scaled_to: Option<f64>,
    pub start: bool,
    pub continued: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanOnScrollParams {
    pub no_wheel_class_name: String,
    pub pan_on_scroll_mode: PanOnScrollMode,
    pub pan_on_scroll_speed: f64,
    pub zoom_on_pinch: bool,
    pub is_mac_os: bool,
}

impl Default for PanOnScrollParams {
    fn default() -> Self {
        PanOnScrollParams {
            no_wheel_class_name: "nowheel".to_string(),
            pan_on_scroll_mode: PanOnScrollMode::Free,
            pan_on_scroll_speed: 0.5,
            zoom_on_pinch: true,
            is_mac_os: super::utils::is_mac_os(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerPosition {
    pub x: f64,
    pub y: f64,
}

pub fn create_pan_on_scroll_handler(
    params: &PanOnScrollParams,
    values: &mut ZoomPanValues,
    event: &PointerEvent,
    current_zoom: f64,
    now_ms: f64,
) -> PanScrollResult {
    if is_wrapped_with_class(&event.classes, &params.no_wheel_class_name) {
        return PanScrollResult {
            handled: false,
            prevented: event.ctrl_key,
            ..PanScrollResult::default()
        };
    }

    if event.ctrl_key && params.zoom_on_pinch {
        let pinch_delta = wheel_delta(
            event.delta_y,
            event.delta_mode,
            event.ctrl_key,
            params.is_mac_os,
        );
        let zoom = current_zoom * 2f64.powf(pinch_delta);
        return PanScrollResult {
            handled: true,
            prevented: true,
            scaled_to: Some(zoom),
            ..PanScrollResult::default()
        };
    }

    let delta_normalize = if event.delta_mode == 1 { 20.0 } else { 1.0 };
    let mut delta_x = if params.pan_on_scroll_mode == PanOnScrollMode::Vertical {
        0.0
    } else {
        event.delta_x * delta_normalize
    };
    let mut delta_y = if params.pan_on_scroll_mode == PanOnScrollMode::Horizontal {
        0.0
    } else {
        event.delta_y * delta_normalize
    };

    if !params.is_mac_os
        && event.shift_key
        && params.pan_on_scroll_mode != PanOnScrollMode::Vertical
    {
        delta_x = event.delta_y * delta_normalize;
        delta_y = 0.0;
    }

    let translate_x = -(delta_x / current_zoom) * params.pan_on_scroll_speed;
    let translate_y = -(delta_y / current_zoom) * params.pan_on_scroll_speed;

    let start = !values.is_pan_scrolling;
    if start {
        values.is_pan_scrolling = true;
    }

    values.pan_scroll_deadline_ms = Some(now_ms + PAN_SCROLL_END_DELAY_MS);

    PanScrollResult {
        handled: true,
        prevented: true,
        translated: Some((translate_x, translate_y)),
        scaled_to: None,
        start,
        continued: !start,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZoomOnScrollParams {
    pub no_wheel_class_name: String,
    pub prevent_scrolling: bool,
}

impl Default for ZoomOnScrollParams {
    fn default() -> Self {
        ZoomOnScrollParams {
            no_wheel_class_name: "nowheel".to_string(),
            prevent_scrolling: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomOnScrollResult {
    pub pass_through: bool,
    pub prevented: bool,
}

pub fn create_zoom_on_scroll_handler(
    params: &ZoomOnScrollParams,
    event: &PointerEvent,
) -> ZoomOnScrollResult {
    let is_wheel = event.kind == PointerEventKind::Wheel;
    let prevent_zoom = !params.prevent_scrolling && is_wheel && !event.ctrl_key;
    let has_no_wheel_class = is_wrapped_with_class(&event.classes, &params.no_wheel_class_name);

    if event.ctrl_key && is_wheel && has_no_wheel_class {
        return ZoomOnScrollResult {
            pass_through: false,
            prevented: true,
        };
    }

    if prevent_zoom || has_no_wheel_class {
        return ZoomOnScrollResult {
            pass_through: false,
            prevented: false,
        };
    }

    ZoomOnScrollResult {
        pass_through: true,
        prevented: true,
    }
}

#[derive(Clone, Debug, Default)]
pub struct PanZoomStartParams {
    pub on_dragging_change: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanZoomStartResult {
    pub applied: bool,
    pub viewport: Viewport,
    pub dragging: bool,
}

pub fn create_pan_zoom_start_handler(
    values: &mut ZoomPanValues,
    event: &ZoomEvent,
) -> PanZoomStartResult {
    if event.internal || event.source.classes.iter().any(|c| c == "__internal") {
        return PanZoomStartResult {
            applied: false,
            viewport: values.prev_viewport,
            dragging: false,
        };
    }

    let viewport = event.transform.viewport();
    values.mouse_button = event.source.button;
    values.is_zooming_or_panning = true;
    values.prev_viewport = viewport;

    let dragging = event.source.kind == PointerEventKind::MouseDown;

    PanZoomStartResult {
        applied: true,
        viewport,
        dragging,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanZoomParams {
    pub pan_on_drag: PanOnDrag,
    pub on_pane_context_menu: bool,
}

impl Default for PanZoomParams {
    fn default() -> Self {
        PanZoomParams {
            pan_on_drag: PanOnDrag::Enabled,
            on_pane_context_menu: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanZoomResult {
    pub transform: Transform,
    pub viewport: Viewport,
    pub emit_transform_change: bool,
    pub emit_pan_zoom: bool,
    pub used_right_mouse_button: bool,
}

pub fn create_pan_zoom_handler(
    values: &mut ZoomPanValues,
    params: &PanZoomParams,
    event: &ZoomEvent,
) -> PanZoomResult {
    values.used_right_mouse_button =
        params.on_pane_context_menu && is_right_click_pan(&params.pan_on_drag, values.mouse_button);

    PanZoomResult {
        transform: event.transform,
        viewport: event.transform.viewport(),
        emit_transform_change: !event.sync,
        emit_pan_zoom: !event.internal,
        used_right_mouse_button: values.used_right_mouse_button,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanZoomEndParams {
    pub pan_on_drag: PanOnDrag,
    pub pan_on_scroll: bool,
    pub on_pane_context_menu: bool,
}

impl Default for PanZoomEndParams {
    fn default() -> Self {
        PanZoomEndParams {
            pan_on_drag: PanOnDrag::Enabled,
            pan_on_scroll: false,
            on_pane_context_menu: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanZoomEndResult {
    pub applied: bool,
    pub viewport: Viewport,
    pub viewport_changed: bool,
    pub dragging: bool,
    pub fire_pane_context_menu: bool,
}

pub fn create_pan_zoom_end_handler(
    values: &mut ZoomPanValues,
    params: &PanZoomEndParams,
    event: &ZoomEvent,
) -> PanZoomEndResult {
    if event.internal {
        return PanZoomEndResult {
            applied: false,
            viewport: values.prev_viewport,
            viewport_changed: false,
            dragging: false,
            fire_pane_context_menu: false,
        };
    }

    values.is_zooming_or_panning = false;

    let fire_pane_context_menu = params.on_pane_context_menu
        && is_right_click_pan(&params.pan_on_drag, values.mouse_button)
        && !values.used_right_mouse_button;

    values.used_right_mouse_button = false;

    let viewport = event.transform.viewport();
    let viewport_changed = viewport != values.prev_viewport;
    values.prev_viewport = viewport;

    PanZoomEndResult {
        applied: true,
        viewport,
        viewport_changed,
        dragging: false,
        fire_pane_context_menu,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PanScrollTimeoutResult {
    Idle,
    Fired { viewport: Viewport },
}

pub fn poll_pan_scroll_timeout(
    values: &mut ZoomPanValues,
    now_ms: f64,
    viewport: Viewport,
) -> PanScrollTimeoutResult {
    let Some(deadline) = values.pan_scroll_deadline_ms else {
        return PanScrollTimeoutResult::Idle;
    };
    if now_ms < deadline {
        return PanScrollTimeoutResult::Idle;
    }
    values.pan_scroll_deadline_ms = None;
    values.pan_scroll_timeout = None;
    values.is_pan_scrolling = false;
    PanScrollTimeoutResult::Fired { viewport }
}
