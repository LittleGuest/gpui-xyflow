use crate::core::{
    constants::{INFINITE_EXTENT, ZOOM_IN_FACTOR, ZOOM_OUT_FACTOR},
    geometry::{
        CoordinateExtent, Dimensions, PanOnScrollMode, Rect, Transform, Viewport, XYPosition,
    },
    utils::{clamp, is_numeric},
};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum PanOnDrag {
    Disabled,
    #[default]
    Enabled,
    Buttons(Vec<u32>),
}

impl PanOnDrag {
    pub fn from_bool(value: bool) -> Self {
        if value {
            PanOnDrag::Enabled
        } else {
            PanOnDrag::Disabled
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, PanOnDrag::Disabled)
    }

    pub fn allows_button(&self, button: u32) -> bool {
        match self {
            PanOnDrag::Disabled => false,
            PanOnDrag::Enabled => button <= 1,
            PanOnDrag::Buttons(buttons) => buttons.contains(&button),
        }
    }

    pub fn buttons(&self) -> Vec<u32> {
        match self {
            PanOnDrag::Disabled => Vec::new(),
            PanOnDrag::Enabled => vec![0, 1],
            PanOnDrag::Buttons(buttons) => buttons.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanZoomOptions {
    pub min_zoom: f64,
    pub max_zoom: f64,
    pub translate_extent: CoordinateExtent,
    pub zoom_on_scroll: bool,
    pub zoom_on_pinch: bool,
    pub zoom_on_double_click: bool,
    pub pan_on_drag: PanOnDrag,
    pub pan_on_scroll: bool,
    pub pan_on_scroll_mode: PanOnScrollMode,
    pub pan_on_scroll_speed: f64,
    pub prevent_scrolling: bool,
    pub user_selection_active: bool,
    pub connection_in_progress: bool,
    pub selection_on_drag: bool,
    pub pane_click_distance: f64,
    pub pan_activation_key_pressed: bool,
    pub zoom_activation_key_pressed: bool,
}

impl Default for PanZoomOptions {
    fn default() -> Self {
        PanZoomOptions {
            min_zoom: 0.5,
            max_zoom: 2.0,
            translate_extent: INFINITE_EXTENT,
            zoom_on_scroll: true,
            zoom_on_pinch: true,
            zoom_on_double_click: true,
            pan_on_drag: PanOnDrag::Enabled,
            pan_on_scroll: false,
            pan_on_scroll_mode: PanOnScrollMode::Free,
            pan_on_scroll_speed: 0.5,
            prevent_scrolling: true,
            user_selection_active: false,
            connection_in_progress: false,
            selection_on_drag: false,
            pane_click_distance: 1.0,
            pan_activation_key_pressed: false,
            zoom_activation_key_pressed: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanZoomEvent {
    Start,
    Zoom,
    End,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PanZoomState {
    pub transform: Transform,
    pub is_zooming_or_panning: bool,
    pub is_pan_scrolling: bool,
    pub used_right_mouse_button: bool,
    pub mouse_button: u32,
    pub prev_viewport: Viewport,
}

pub fn constrain_transform(
    transform: Transform,
    extent: Rect,
    translate_extent: CoordinateExtent,
) -> Transform {
    let dx0 = transform.invert(XYPosition::new(extent.x, extent.y)).x - translate_extent[0][0];
    let dx1 = transform
        .invert(XYPosition::new(extent.max_x(), extent.y))
        .x
        - translate_extent[1][0];
    let dy0 = transform.invert(XYPosition::new(extent.x, extent.y)).y - translate_extent[0][1];
    let dy1 = transform
        .invert(XYPosition::new(extent.x, extent.max_y()))
        .y
        - translate_extent[1][1];

    let translate_x = if dx1 > dx0 {
        (dx0 + dx1) / 2.0
    } else {
        let min_zero = 0.0_f64.min(dx0);
        if min_zero != 0.0 {
            min_zero
        } else {
            0.0_f64.max(dx1)
        }
    };

    let translate_y = if dy1 > dy0 {
        (dy0 + dy1) / 2.0
    } else {
        let min_zero = 0.0_f64.min(dy0);
        if min_zero != 0.0 {
            min_zero
        } else {
            0.0_f64.max(dy1)
        }
    };

    Transform(
        transform.x() + transform.zoom() * translate_x,
        transform.y() + transform.zoom() * translate_y,
        transform.zoom(),
    )
}

pub fn scale_transform(transform: Transform, k: f64, scale_extent: [f64; 2]) -> Transform {
    let k = clamp(k, scale_extent[0], scale_extent[1]);
    if k == transform.zoom() {
        transform
    } else {
        Transform(transform.x(), transform.y(), k)
    }
}

pub fn translate_transform(transform: Transform, p0: XYPosition, p1: XYPosition) -> Transform {
    let x = p0.x - p1.x * transform.zoom();
    let y = p0.y - p1.y * transform.zoom();
    if x == transform.x() && y == transform.y() {
        transform
    } else {
        Transform(x, y, transform.zoom())
    }
}

pub fn viewport_to_transform(viewport: Viewport) -> Transform {
    Transform(viewport.x, viewport.y, viewport.zoom)
}

pub fn transform_to_viewport(transform: Transform) -> Viewport {
    Viewport::new(transform.x(), transform.y(), transform.zoom())
}

pub fn wheel_delta(delta_y: f64, delta_mode: u32, ctrl_key: bool, is_mac: bool) -> f64 {
    let factor = if ctrl_key && is_mac { 10.0 } else { 1.0 };
    let unit = match delta_mode {
        1 => 0.05,
        0 => 0.002,
        _ => 1.0,
    };
    -delta_y * unit * factor
}

pub fn zoom_factor_for_wheel(delta_y: f64, delta_mode: u32, ctrl_key: bool, is_mac: bool) -> f64 {
    2.0_f64.powf(wheel_delta(delta_y, delta_mode, ctrl_key, is_mac))
}

pub fn is_right_click_pan(pan_on_drag: &PanOnDrag, used_button: u32) -> bool {
    used_button == 2 && pan_on_drag.buttons().contains(&2)
}

pub fn transition_ease_cubic_in_out(t: f64) -> f64 {
    if !t.is_finite() {
        return 0.0;
    }
    let t = t * 2.0;
    if t <= 1.0 {
        t * t * t / 2.0
    } else {
        let t = t - 2.0;
        (t * t * t + 2.0) / 2.0
    }
}

#[derive(Clone, Debug)]
pub struct PanZoom {
    transform: Transform,
    options: PanZoomOptions,
    extent: Dimensions,
    state: PanZoomState,
    drag: Option<DragSession>,
}

#[derive(Clone, Debug)]
struct DragSession {
    start: XYPosition,
    moved: bool,
}

impl PanZoom {
    pub fn new(viewport: Viewport, options: PanZoomOptions) -> Self {
        let transform = viewport_to_transform(Viewport::new(
            viewport.x,
            viewport.y,
            clamp(viewport.zoom, options.min_zoom, options.max_zoom),
        ));
        PanZoom {
            transform,
            options,
            extent: Dimensions::ZERO,
            state: PanZoomState {
                transform,
                prev_viewport: viewport,
                ..PanZoomState::default()
            },
            drag: None,
        }
    }

    pub fn options(&self) -> &PanZoomOptions {
        &self.options
    }

    pub fn set_options(&mut self, options: PanZoomOptions) {
        self.options = options;
    }

    pub fn set_extent(&mut self, width: f64, height: f64) {
        self.extent = Dimensions::new(width, height);
    }

    pub fn extent(&self) -> Dimensions {
        self.extent
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub fn viewport(&self) -> Viewport {
        transform_to_viewport(self.transform)
    }

    pub fn state(&self) -> &PanZoomState {
        &self.state
    }

    pub fn scale_extent(&self) -> [f64; 2] {
        [self.options.min_zoom, self.options.max_zoom]
    }

    pub fn set_scale_extent(&mut self, min_zoom: f64, max_zoom: f64) {
        self.options.min_zoom = min_zoom;
        self.options.max_zoom = max_zoom;
        let zoom = clamp(self.transform.zoom(), min_zoom, max_zoom);
        if zoom != self.transform.zoom() {
            self.transform = Transform(self.transform.x(), self.transform.y(), zoom);
        }
    }

    pub fn set_translate_extent(&mut self, translate_extent: CoordinateExtent) {
        self.options.translate_extent = translate_extent;
    }

    pub fn extent_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.extent.width, self.extent.height)
    }

    pub fn extent_of(&self) -> CoordinateExtent {
        [[0.0, 0.0], [self.extent.width, self.extent.height]]
    }

    pub fn set_transform(&mut self, transform: Transform) {
        self.transform = transform;
        self.state.transform = transform;
    }

    pub fn constrained_in(
        &self,
        viewport: Viewport,
        extent: CoordinateExtent,
        translate_extent: CoordinateExtent,
    ) -> Viewport {
        let rect = Rect::new(0.0, 0.0, extent[1][0], extent[1][1]);
        transform_to_viewport(constrain_transform(
            viewport_to_transform(viewport),
            rect,
            translate_extent,
        ))
    }

    pub fn constrained(&self, viewport: Viewport) -> Viewport {
        transform_to_viewport(constrain_transform(
            viewport_to_transform(viewport),
            self.extent_rect(),
            self.options.translate_extent,
        ))
    }

    pub fn set_viewport(&mut self, viewport: Viewport) -> (Transform, Vec<PanZoomEvent>) {
        self.set_viewport_constrained(viewport)
    }

    pub fn set_viewport_constrained(
        &mut self,
        viewport: Viewport,
    ) -> (Transform, Vec<PanZoomEvent>) {
        let next = constrain_transform(
            viewport_to_transform(viewport),
            self.extent_rect(),
            self.options.translate_extent,
        );
        let previous = self.transform;
        self.transform = next;
        self.state.transform = next;
        let events = if previous == next {
            Vec::new()
        } else {
            vec![PanZoomEvent::Zoom]
        };
        (next, events)
    }

    pub fn sync_viewport(&mut self, viewport: Viewport) {
        self.transform = viewport_to_transform(viewport);
        self.state.transform = self.transform;
    }

    pub fn scale_to(
        &mut self,
        zoom: f64,
        point: Option<XYPosition>,
    ) -> (Transform, Vec<PanZoomEvent>) {
        let current = self.transform;
        let p0 = point
            .unwrap_or_else(|| XYPosition::new(self.extent.width / 2.0, self.extent.height / 2.0));
        let p1 = current.invert(p0);
        let scaled = scale_transform(current, zoom, self.scale_extent());
        let translated = translate_transform(scaled, p0, p1);
        let constrained = constrain_transform(
            translated,
            self.extent_rect(),
            self.options.translate_extent,
        );
        let changed = constrained != current;
        self.transform = constrained;
        self.state.transform = constrained;
        (
            constrained,
            if changed {
                vec![PanZoomEvent::Zoom]
            } else {
                Vec::new()
            },
        )
    }

    pub fn scale_by(
        &mut self,
        factor: f64,
        point: Option<XYPosition>,
    ) -> (Transform, Vec<PanZoomEvent>) {
        let next = self.transform.zoom() * factor;
        self.scale_to(next, point)
    }

    pub fn zoom_in(&mut self) -> (Transform, Vec<PanZoomEvent>) {
        self.scale_by(ZOOM_IN_FACTOR, None)
    }

    pub fn zoom_out(&mut self) -> (Transform, Vec<PanZoomEvent>) {
        self.scale_by(ZOOM_OUT_FACTOR, None)
    }

    pub fn translate_by(&mut self, x: f64, y: f64) -> (Transform, Vec<PanZoomEvent>) {
        let next = constrain_transform(
            Transform(
                self.transform.x() + x,
                self.transform.y() + y,
                self.transform.zoom(),
            ),
            self.extent_rect(),
            self.options.translate_extent,
        );
        let changed = next != self.transform;
        self.transform = next;
        self.state.transform = next;
        (
            next,
            if changed {
                vec![PanZoomEvent::Zoom]
            } else {
                Vec::new()
            },
        )
    }

    pub fn pan_by(&mut self, delta: XYPosition) -> bool {
        if delta.is_zero() {
            return false;
        }
        let previous = self.transform;
        let (next, _) = self.translate_by(delta.x, delta.y);
        next != previous
    }

    pub fn zoom_scroll_enabled(&self) -> bool {
        self.options.zoom_on_scroll || self.options.zoom_activation_key_pressed
    }

    pub fn filter_wheel(&self, ctrl_key: bool) -> bool {
        if self.options.user_selection_active {
            return false;
        }
        if !self.options.zoom_on_pinch && ctrl_key {
            return false;
        }
        let pinch = self.options.zoom_on_pinch && ctrl_key;
        self.zoom_scroll_enabled() || self.options.pan_on_scroll || pinch
    }

    pub fn wheel(
        &mut self,
        position: XYPosition,
        delta_y: f64,
        delta_mode: u32,
        ctrl_key: bool,
        is_mac: bool,
        shift_key: bool,
    ) -> (Transform, Vec<PanZoomEvent>) {
        if !self.filter_wheel(ctrl_key) {
            return (self.transform, Vec::new());
        }

        if self.options.pan_on_scroll && !self.options.zoom_activation_key_pressed {
            if ctrl_key && self.options.zoom_on_pinch {
                let factor = zoom_factor_for_wheel(delta_y, delta_mode, ctrl_key, is_mac);
                return self.scale_by(factor, Some(position));
            }

            let normalize = if delta_mode == 1 { 20.0 } else { 1.0 };
            let mut delta_x = if self.options.pan_on_scroll_mode == PanOnScrollMode::Vertical {
                0.0
            } else {
                delta_y * normalize
            };
            let mut delta_yv = if self.options.pan_on_scroll_mode == PanOnScrollMode::Horizontal {
                0.0
            } else {
                delta_y * normalize
            };

            if !is_mac && shift_key && self.options.pan_on_scroll_mode != PanOnScrollMode::Vertical
            {
                delta_x = delta_y * normalize;
                delta_yv = 0.0;
            }

            let zoom = self.transform.zoom();
            let speed = self.options.pan_on_scroll_speed;
            return self.translate_by(-(delta_x / zoom) * speed, -(delta_yv / zoom) * speed);
        }

        let factor = zoom_factor_for_wheel(delta_y, delta_mode, ctrl_key, is_mac);
        let next_zoom = clamp(
            self.transform.zoom() * factor,
            self.options.min_zoom,
            self.options.max_zoom,
        );

        if next_zoom == self.transform.zoom() {
            return (self.transform, Vec::new());
        }

        let current = self.transform;
        let p1 = current.invert(position);
        let scaled = Transform(current.x(), current.y(), next_zoom);
        let translated = translate_transform(scaled, position, p1);
        let next = constrain_transform(
            translated,
            self.extent_rect(),
            self.options.translate_extent,
        );
        self.transform = next;
        self.state.transform = next;
        (next, vec![PanZoomEvent::Zoom])
    }

    pub fn filter_pan(&self, button: u32, ctrl_key: bool) -> bool {
        if self.options.user_selection_active {
            return false;
        }
        if !self.options.pan_on_drag.is_enabled() {
            return false;
        }
        if !self.options.pan_on_drag.allows_button(button) {
            return false;
        }
        !ctrl_key || self.options.pan_activation_key_pressed
    }

    pub fn pointer_down(
        &mut self,
        button: u32,
        position: XYPosition,
        ctrl_key: bool,
    ) -> Vec<PanZoomEvent> {
        if !self.filter_pan(button, ctrl_key) {
            return Vec::new();
        }
        self.drag = Some(DragSession {
            start: position,
            moved: false,
        });
        self.state.mouse_button = button;
        self.state.is_zooming_or_panning = true;
        self.state.prev_viewport = self.viewport();
        vec![PanZoomEvent::Start]
    }

    pub fn pointer_move(&mut self, position: XYPosition) -> Vec<PanZoomEvent> {
        let click_distance = if self.options.selection_on_drag {
            f64::INFINITY
        } else if !is_numeric(self.options.pane_click_distance)
            || self.options.pane_click_distance < 0.0
        {
            0.0
        } else {
            self.options.pane_click_distance
        };

        let Some(session) = self.drag.as_mut() else {
            return Vec::new();
        };

        let dx = position.x - session.start.x;
        let dy = position.y - session.start.y;
        if !session.moved {
            session.moved = dx * dx + dy * dy > click_distance * click_distance;
        }

        let current = self.transform;
        let anchor = current.invert(position);
        let next = constrain_transform(
            translate_transform(current, position, anchor),
            self.extent_rect(),
            self.options.translate_extent,
        );

        if next == current {
            return Vec::new();
        }

        self.transform = next;
        self.state.transform = next;
        vec![PanZoomEvent::Zoom]
    }

    pub fn pointer_up(&mut self) -> (Vec<PanZoomEvent>, bool) {
        let Some(session) = self.drag.take() else {
            return (Vec::new(), false);
        };
        self.state.is_zooming_or_panning = false;
        self.state.prev_viewport = self.viewport();
        (vec![PanZoomEvent::End], session.moved)
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.as_ref().is_some_and(|session| session.moved)
    }

    pub fn is_panning(&self) -> bool {
        self.drag.is_some()
    }

    pub fn double_click(
        &mut self,
        position: XYPosition,
        shift_key: bool,
    ) -> (Transform, Vec<PanZoomEvent>) {
        if !self.options.zoom_on_double_click || self.options.user_selection_active {
            return (self.transform, Vec::new());
        }
        let factor = if shift_key { 0.5 } else { 2.0 };
        let target = self.transform.zoom() * factor;
        self.scale_to(target, Some(position))
    }

    pub fn set_click_distance(&mut self, distance: f64) {
        self.options.pane_click_distance = if !is_numeric(distance) || distance < 0.0 {
            0.0
        } else {
            distance
        };
    }

    pub fn reset_state(&mut self) {
        self.state.is_zooming_or_panning = false;
        self.state.is_pan_scrolling = false;
        self.state.used_right_mouse_button = false;
        self.drag = None;
    }
}
