use crate::core::{
    geometry::{CoordinateExtent, PanOnScrollMode, Transform, Viewport},
    panzoom::{PanOnDrag, PanZoom, PanZoomOptions},
    xypanzoom::{
        eventhandler::{
            PanOnScrollParams, PanScrollResult, PanScrollTimeoutResult, PanZoomEndParams,
            PanZoomEndResult, PanZoomParams, PanZoomResult, PanZoomStartResult, ZoomEvent,
            ZoomOnScrollParams, ZoomOnScrollResult, ZoomPanValues, create_pan_on_scroll_handler,
            create_pan_zoom_end_handler, create_pan_zoom_handler, create_pan_zoom_start_handler,
            create_zoom_on_scroll_handler, poll_pan_scroll_timeout,
        },
        filter::{FilterOutcome, FilterParams, PointerEvent, create_filter},
        utils::{ViewportTransition, get_d3_transition, is_right_click_pan},
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct PanZoomUpdateOptions {
    pub no_wheel_class_name: String,
    pub no_pan_class_name: String,
    pub on_pane_context_menu: bool,
    pub user_selection_active: bool,
    pub pan_on_scroll: bool,
    pub pan_on_drag: PanOnDrag,
    pub pan_on_scroll_mode: PanOnScrollMode,
    pub pan_on_scroll_speed: f64,
    pub prevent_scrolling: bool,
    pub zoom_on_pinch: bool,
    pub zoom_on_scroll: bool,
    pub zoom_on_double_click: bool,
    pub pan_activation_key_pressed: bool,
    pub zoom_activation_key_pressed: bool,
    pub lib: String,
    pub connection_in_progress: bool,
    pub pane_click_distance: f64,
    pub selection_on_drag: bool,
}

impl Default for PanZoomUpdateOptions {
    fn default() -> Self {
        PanZoomUpdateOptions {
            no_wheel_class_name: "nowheel".to_string(),
            no_pan_class_name: "nopan".to_string(),
            on_pane_context_menu: false,
            user_selection_active: false,
            pan_on_scroll: false,
            pan_on_drag: PanOnDrag::Enabled,
            pan_on_scroll_mode: PanOnScrollMode::Free,
            pan_on_scroll_speed: 0.5,
            prevent_scrolling: true,
            zoom_on_pinch: true,
            zoom_on_scroll: true,
            zoom_on_double_click: true,
            pan_activation_key_pressed: false,
            zoom_activation_key_pressed: false,
            lib: "gpui".to_string(),
            connection_in_progress: false,
            pane_click_distance: 1.0,
            selection_on_drag: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanZoomHandlers {
    pub is_pan_on_scroll: bool,
    pub pan_on_scroll: Option<PanOnScrollParams>,
    pub zoom_on_scroll: Option<ZoomOnScrollParams>,
    pub filter: FilterParams,
    pub pan_zoom: PanZoomParams,
    pub pan_zoom_end: PanZoomEndParams,
    pub zoom_on_double_click: bool,
    pub click_distance: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PanZoomEventOutput {
    pub pan_zoom_start: Option<PanZoomStartResult>,
    pub pan_zoom: Option<PanZoomResult>,
    pub pan_zoom_end: Option<PanZoomEndResult>,
    pub transform_changed: Option<Transform>,
}

#[derive(Clone, Debug)]
pub struct XYPanZoom {
    pan_zoom: PanZoom,
    values: ZoomPanValues,
    handlers: PanZoomHandlers,
    transition: Option<ViewportTransition>,
    destroyed: bool,
    extent: CoordinateExtent,
    click_distance: f64,
}

impl XYPanZoom {
    pub fn new(viewport: Viewport, options: PanZoomOptions, extent: CoordinateExtent) -> Self {
        let mut pan_zoom = PanZoom::new(viewport, options.clone());
        pan_zoom.set_extent(extent[1][0], extent[1][1]);
        let constrained = Viewport {
            x: viewport.x,
            y: viewport.y,
            zoom: viewport.zoom.clamp(options.min_zoom, options.max_zoom),
        };
        pan_zoom.set_viewport_constrained(constrained);
        let handlers = PanZoomHandlers {
            is_pan_on_scroll: false,
            pan_on_scroll: None,
            zoom_on_scroll: Some(ZoomOnScrollParams::default()),
            filter: FilterParams::default(),
            pan_zoom: PanZoomParams::default(),
            pan_zoom_end: PanZoomEndParams::default(),
            zoom_on_double_click: options.zoom_on_double_click,
            click_distance: 0.0,
        };
        XYPanZoom {
            pan_zoom,
            values: ZoomPanValues::default(),
            handlers,
            transition: None,
            destroyed: false,
            extent,
            click_distance: 0.0,
        }
    }

    pub fn update(&mut self, options: &PanZoomUpdateOptions) -> PanZoomHandlers {
        if options.user_selection_active && !self.values.is_zooming_or_panning {
            self.destroy();
        }

        let is_pan_on_scroll = options.pan_on_scroll
            && !options.zoom_activation_key_pressed
            && !options.user_selection_active;

        let click_distance = if options.selection_on_drag {
            f64::INFINITY
        } else if !options.pane_click_distance.is_finite() || options.pane_click_distance < 0.0 {
            0.0
        } else {
            options.pane_click_distance
        };
        self.click_distance = click_distance;
        self.pan_zoom.set_click_distance(click_distance);

        self.handlers = PanZoomHandlers {
            is_pan_on_scroll,
            pan_on_scroll: if is_pan_on_scroll {
                Some(PanOnScrollParams {
                    no_wheel_class_name: options.no_wheel_class_name.clone(),
                    pan_on_scroll_mode: options.pan_on_scroll_mode,
                    pan_on_scroll_speed: options.pan_on_scroll_speed,
                    zoom_on_pinch: options.zoom_on_pinch,
                    is_mac_os: crate::core::xypanzoom::utils::is_mac_os(),
                })
            } else {
                None
            },
            zoom_on_scroll: if is_pan_on_scroll {
                None
            } else {
                Some(ZoomOnScrollParams {
                    no_wheel_class_name: options.no_wheel_class_name.clone(),
                    prevent_scrolling: options.prevent_scrolling,
                })
            },
            filter: FilterParams {
                pan_activation_key_pressed: options.pan_activation_key_pressed,
                zoom_activation_key_pressed: options.zoom_activation_key_pressed,
                zoom_on_scroll: options.zoom_on_scroll,
                zoom_on_pinch: options.zoom_on_pinch,
                pan_on_drag: options.pan_on_drag.clone(),
                pan_on_scroll: options.pan_on_scroll,
                zoom_on_double_click: options.zoom_on_double_click,
                user_selection_active: options.user_selection_active,
                no_wheel_class_name: options.no_wheel_class_name.clone(),
                no_pan_class_name: options.no_pan_class_name.clone(),
                lib: options.lib.clone(),
                connection_in_progress: options.connection_in_progress,
            },
            pan_zoom: PanZoomParams {
                pan_on_drag: options.pan_on_drag.clone(),
                on_pane_context_menu: options.on_pane_context_menu,
            },
            pan_zoom_end: PanZoomEndParams {
                pan_on_drag: options.pan_on_drag.clone(),
                pan_on_scroll: options.pan_on_scroll,
                on_pane_context_menu: options.on_pane_context_menu,
            },
            zoom_on_double_click: options.zoom_on_double_click,
            click_distance,
        };

        self.handlers.clone()
    }

    pub fn destroy(&mut self) {
        self.destroyed = true;
    }

    pub fn restore(&mut self) {
        self.destroyed = false;
    }

    pub fn destroyed(&self) -> bool {
        self.destroyed
    }

    pub fn handlers(&self) -> &PanZoomHandlers {
        &self.handlers
    }

    pub fn values(&self) -> &ZoomPanValues {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut ZoomPanValues {
        &mut self.values
    }

    pub fn pan_zoom(&self) -> &PanZoom {
        &self.pan_zoom
    }

    pub fn extent(&self) -> CoordinateExtent {
        self.extent
    }

    pub fn click_distance(&self) -> f64 {
        self.click_distance
    }

    pub fn set_extent(&mut self, extent: CoordinateExtent) {
        self.extent = extent;
        self.pan_zoom.set_extent(extent[1][0], extent[1][1]);
    }

    pub fn filter(&self, event: &PointerEvent) -> FilterOutcome {
        create_filter(&self.handlers.filter, event)
    }

    pub fn accepts(&self, event: &PointerEvent) -> bool {
        !self.destroyed && self.filter(event).allowed()
    }

    pub fn wheel(&mut self, event: &PointerEvent, now_ms: f64) -> WheelOutcome {
        let outcome = self.filter(event);
        if self.destroyed || !outcome.allowed() {
            return WheelOutcome {
                outcome,
                pan_scroll: None,
                zoom_on_scroll: None,
                transform: None,
            };
        }

        if let Some(params) = &self.handlers.pan_on_scroll {
            let current_zoom = self.pan_zoom.transform().zoom();
            let result =
                create_pan_on_scroll_handler(params, &mut self.values, event, current_zoom, now_ms);
            let mut transform = None;
            if let Some(zoom) = result.scaled_to {
                self.pan_zoom.scale_to(zoom, None);
                transform = Some(self.pan_zoom.transform());
            } else if let Some((dx, dy)) = result.translated {
                self.pan_zoom.translate_by(dx, dy);
                transform = Some(self.pan_zoom.transform());
            }
            return WheelOutcome {
                outcome,
                pan_scroll: Some(result),
                zoom_on_scroll: None,
                transform,
            };
        }

        let result = self
            .handlers
            .zoom_on_scroll
            .as_ref()
            .map(|params| create_zoom_on_scroll_handler(params, event));
        if let Some(result) = result
            && result.pass_through
        {
            let zoom_factor = 2f64.powf(-event.delta_y * 0.002);
            self.pan_zoom.scale_by(zoom_factor, None);
        }
        WheelOutcome {
            outcome,
            pan_scroll: None,
            zoom_on_scroll: result,
            transform: Some(self.pan_zoom.transform()),
        }
    }

    pub fn start(&mut self, event: &ZoomEvent) -> PanZoomStartResult {
        let result = create_pan_zoom_start_handler(&mut self.values, event);
        if result.applied {
            self.pan_zoom.set_transform(event.transform);
        }
        result
    }

    pub fn zoom(&mut self, event: &ZoomEvent) -> PanZoomResult {
        let result = create_pan_zoom_handler(&mut self.values, &self.handlers.pan_zoom, event);
        self.pan_zoom.set_transform(event.transform);
        result
    }

    pub fn end(&mut self, event: &ZoomEvent) -> PanZoomEndResult {
        let result =
            create_pan_zoom_end_handler(&mut self.values, &self.handlers.pan_zoom_end, event);
        if result.applied {
            self.pan_zoom.set_transform(event.transform);
        }
        result
    }

    pub fn dispatch(&mut self, event: &ZoomEvent) -> PanZoomEventOutput {
        if self.destroyed {
            return PanZoomEventOutput::default();
        }
        let pan_zoom_start = Some(self.start(event));
        let pan_zoom = self.zoom(event);
        let transform_changed = if pan_zoom.emit_transform_change {
            Some(pan_zoom.transform)
        } else {
            None
        };
        PanZoomEventOutput {
            pan_zoom_start,
            transform_changed,
            pan_zoom: Some(pan_zoom),
            pan_zoom_end: Some(self.end(event)),
        }
    }

    pub fn poll_timeout(&mut self, now_ms: f64) -> PanScrollTimeoutResult {
        let viewport = self.pan_zoom.viewport();
        poll_pan_scroll_timeout(&mut self.values, now_ms, viewport)
    }

    pub fn is_pan_scrolling(&self) -> bool {
        self.values.is_pan_scrolling
    }

    pub fn is_zooming_or_panning(&self) -> bool {
        self.values.is_zooming_or_panning
    }

    pub fn set_viewport(&mut self, viewport: Viewport, duration: f64) -> Viewport {
        let next = viewport;
        if duration > 0.0 {
            self.transition = get_d3_transition(self.pan_zoom.viewport(), next, duration);
            self.pan_zoom.viewport()
        } else {
            self.transition = None;
            self.pan_zoom.set_viewport(next);
            next
        }
    }

    pub fn set_viewport_constrained(
        &mut self,
        viewport: Viewport,
        extent: CoordinateExtent,
        translate_extent: CoordinateExtent,
        duration: f64,
    ) -> Viewport {
        let constrained = self
            .pan_zoom
            .constrained_in(viewport, extent, translate_extent);
        self.set_viewport(constrained, duration);
        constrained
    }

    pub fn get_viewport(&self) -> Viewport {
        self.pan_zoom.viewport()
    }

    pub fn sync_viewport(&mut self, viewport: Viewport) {
        let current = self.pan_zoom.transform();
        let next = Transform::new(viewport.x, viewport.y, viewport.zoom);
        if current.x() != next.x() || current.y() != next.y() || current.zoom() != next.zoom() {
            self.pan_zoom.set_transform(next);
        }
    }

    pub fn scale_to(&mut self, zoom: f64, duration: f64) -> f64 {
        if duration > 0.0 {
            let mut target = self.pan_zoom.viewport();
            target.zoom = zoom;
            self.transition = get_d3_transition(self.pan_zoom.viewport(), target, duration);
            self.pan_zoom.viewport().zoom
        } else {
            self.transition = None;
            self.pan_zoom.scale_to(zoom, None);
            self.pan_zoom.viewport().zoom
        }
    }

    pub fn scale_by(&mut self, factor: f64, duration: f64) -> f64 {
        if duration > 0.0 {
            let current = self.pan_zoom.viewport();
            let mut target = current;
            target.zoom = (current.zoom * factor).clamp(
                self.pan_zoom.options().min_zoom,
                self.pan_zoom.options().max_zoom,
            );
            self.transition = get_d3_transition(current, target, duration);
            current.zoom
        } else {
            self.transition = None;
            self.pan_zoom.scale_by(factor, None);
            self.pan_zoom.viewport().zoom
        }
    }

    pub fn set_scale_extent(&mut self, min_zoom: f64, max_zoom: f64) {
        self.pan_zoom.set_scale_extent(min_zoom, max_zoom);
    }

    pub fn set_translate_extent(&mut self, translate_extent: CoordinateExtent) {
        self.pan_zoom.set_translate_extent(translate_extent);
    }

    pub fn set_click_distance(&mut self, distance: f64) {
        let valid = if !distance.is_finite() || distance < 0.0 {
            0.0
        } else {
            distance
        };
        self.click_distance = valid;
        self.pan_zoom.set_click_distance(valid);
    }

    pub fn advancing(&self) -> bool {
        self.transition.is_some()
    }

    pub fn tick(&mut self, delta_ms: f64) -> Option<Viewport> {
        let transition = self.transition.as_mut()?;
        let viewport = transition.advance(delta_ms);
        let finished = transition.finished();
        if finished {
            self.transition = None;
            self.pan_zoom.set_viewport(viewport);
        } else {
            self.pan_zoom.set_viewport(viewport);
        }
        Some(viewport)
    }

    pub fn finish_transition(&mut self) -> Option<Viewport> {
        let transition = self.transition.take()?;
        let viewport = transition.target();
        self.pan_zoom.set_viewport(viewport);
        Some(viewport)
    }

    pub fn right_click_pan(&self, button: u32) -> bool {
        is_right_click_pan(&self.handlers.pan_zoom.pan_on_drag, button)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WheelOutcome {
    pub outcome: FilterOutcome,
    pub pan_scroll: Option<PanScrollResult>,
    pub zoom_on_scroll: Option<ZoomOnScrollResult>,
    pub transform: Option<Transform>,
}
