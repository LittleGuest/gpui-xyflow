use crate::core::panzoom::PanOnDrag;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerEventKind {
    MouseDown,
    MouseMove,
    MouseUp,
    Wheel,
    TouchStart,
    TouchMove,
    TouchEnd,
    DblClick,
    Unknown,
}

impl PointerEventKind {
    pub fn is_wheel(self) -> bool {
        matches!(self, PointerEventKind::Wheel)
    }

    pub fn is_start(self) -> bool {
        matches!(
            self,
            PointerEventKind::MouseDown | PointerEventKind::TouchStart
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerEventKind,
    pub button: u32,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub meta_key: bool,
    pub delta_x: f64,
    pub delta_y: f64,
    pub delta_mode: u32,
    pub touches: usize,
    pub classes: Vec<String>,
    pub default_prevented: bool,
}

impl Default for PointerEvent {
    fn default() -> Self {
        PointerEvent {
            kind: PointerEventKind::Unknown,
            button: 0,
            ctrl_key: false,
            shift_key: false,
            meta_key: false,
            delta_x: 0.0,
            delta_y: 0.0,
            delta_mode: 0,
            touches: 0,
            classes: Vec::new(),
            default_prevented: false,
        }
    }
}

impl PointerEvent {
    pub fn wheel(delta_y: f64) -> Self {
        PointerEvent {
            kind: PointerEventKind::Wheel,
            delta_y,
            ..PointerEvent::default()
        }
    }

    pub fn mouse_down(button: u32) -> Self {
        PointerEvent {
            kind: PointerEventKind::MouseDown,
            button,
            ..PointerEvent::default()
        }
    }

    pub fn touch_start(touches: usize) -> Self {
        PointerEvent {
            kind: PointerEventKind::TouchStart,
            touches,
            ..PointerEvent::default()
        }
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl_key = true;
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.shift_key = true;
        self
    }

    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.classes.push(class.into());
        self
    }

    pub fn with_touches(mut self, touches: usize) -> Self {
        self.touches = touches;
        self
    }

    pub fn is_wrapped_with_class(&self, class_name: &str) -> bool {
        if class_name.is_empty() {
            return false;
        }
        self.classes.iter().any(|class| class.contains(class_name))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FilterParams {
    pub pan_activation_key_pressed: bool,
    pub zoom_activation_key_pressed: bool,
    pub zoom_on_scroll: bool,
    pub zoom_on_pinch: bool,
    pub pan_on_drag: PanOnDrag,
    pub pan_on_scroll: bool,
    pub zoom_on_double_click: bool,
    pub user_selection_active: bool,
    pub no_wheel_class_name: String,
    pub no_pan_class_name: String,
    pub lib: String,
    pub connection_in_progress: bool,
}

impl Default for FilterParams {
    fn default() -> Self {
        FilterParams {
            pan_activation_key_pressed: false,
            zoom_activation_key_pressed: false,
            zoom_on_scroll: true,
            zoom_on_pinch: true,
            pan_on_drag: PanOnDrag::Enabled,
            pan_on_scroll: false,
            zoom_on_double_click: true,
            user_selection_active: false,
            no_wheel_class_name: "nowheel".to_string(),
            no_pan_class_name: "nopan".to_string(),
            lib: "gpui".to_string(),
            connection_in_progress: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterOutcome {
    Allow,
    Reject,
    RejectAndPreventDefault,
}

impl FilterOutcome {
    pub fn allowed(self) -> bool {
        matches!(self, FilterOutcome::Allow)
    }

    pub fn prevented(self) -> bool {
        matches!(self, FilterOutcome::RejectAndPreventDefault)
    }
}

pub fn create_filter(params: &FilterParams, event: &PointerEvent) -> FilterOutcome {
    let zoom_scroll = params.zoom_activation_key_pressed || params.zoom_on_scroll;
    let pinch_zoom = params.zoom_on_pinch && event.ctrl_key;
    let is_wheel_event = event.kind.is_wheel();

    if event.button == 1
        && event.kind == PointerEventKind::MouseDown
        && (event.is_wrapped_with_class(&format!("{}-flow__node", params.lib))
            || event.is_wrapped_with_class(&format!("{}-flow__edge", params.lib))
            || event.is_wrapped_with_class(&format!("{}-flow__selection", params.lib))
            || event.is_wrapped_with_class(&format!("{}-flow__nodesselection", params.lib)))
    {
        return FilterOutcome::Allow;
    }

    if params.pan_on_drag == PanOnDrag::Disabled
        && !zoom_scroll
        && !params.pan_on_scroll
        && !params.zoom_on_double_click
        && !params.zoom_on_pinch
    {
        return FilterOutcome::Reject;
    }

    if params.user_selection_active {
        return FilterOutcome::Reject;
    }

    if params.connection_in_progress && !is_wheel_event {
        return FilterOutcome::Reject;
    }

    if event.is_wrapped_with_class(&params.no_wheel_class_name) && is_wheel_event {
        return FilterOutcome::Reject;
    }

    if event.is_wrapped_with_class(&params.no_pan_class_name)
        && (!is_wheel_event
            || (params.pan_on_scroll && is_wheel_event && !params.zoom_activation_key_pressed))
    {
        return FilterOutcome::Reject;
    }

    if !params.zoom_on_pinch && event.ctrl_key && is_wheel_event {
        return FilterOutcome::Reject;
    }

    if !params.zoom_on_pinch && event.kind == PointerEventKind::TouchStart && event.touches > 1 {
        return FilterOutcome::RejectAndPreventDefault;
    }

    if !zoom_scroll && !params.pan_on_scroll && !pinch_zoom && is_wheel_event {
        return FilterOutcome::Reject;
    }

    if params.pan_on_drag == PanOnDrag::Disabled && event.kind.is_start() {
        return FilterOutcome::Reject;
    }

    if let PanOnDrag::Buttons(buttons) = &params.pan_on_drag
        && !buttons.contains(&event.button)
        && event.kind == PointerEventKind::MouseDown
    {
        return FilterOutcome::Reject;
    }

    let button_allowed = match &params.pan_on_drag {
        PanOnDrag::Buttons(buttons) => buttons.contains(&event.button),
        _ => event.button == 0 || event.button <= 1,
    };

    if (!event.ctrl_key || is_wheel_event || params.pan_activation_key_pressed) && button_allowed {
        FilterOutcome::Allow
    } else {
        FilterOutcome::Reject
    }
}

pub fn create_native_filter(params: &FilterParams) -> impl Fn(&PointerEvent) -> bool + '_ {
    move |event| create_filter(params, event).allowed()
}
