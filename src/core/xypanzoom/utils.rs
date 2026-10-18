use crate::core::geometry::Viewport;
use crate::core::panzoom::{transition_ease_cubic_in_out, PanOnDrag};

pub fn is_wrapped_with_class(classes: &[String], class_name: &str) -> bool {
    if class_name.is_empty() {
        return false;
    }
    classes.iter().any(|class| class.contains(class_name))
}

pub fn is_right_click_pan(pan_on_drag: &PanOnDrag, used_button: u32) -> bool {
    used_button == 2 && matches!(pan_on_drag, PanOnDrag::Buttons(buttons) if buttons.contains(&2))
}

pub fn default_ease(t: f64) -> f64 {
    transition_ease_cubic_in_out(t)
}

pub fn wheel_delta(event_delta_y: f64, delta_mode: u32, ctrl_key: bool, is_mac_os: bool) -> f64 {
    let factor = if ctrl_key && is_mac_os { 10.0 } else { 1.0 };
    let mode_factor = if delta_mode == 1 {
        0.05
    } else if delta_mode != 0 {
        1.0
    } else {
        0.002
    };
    -event_delta_y * mode_factor * factor
}

pub fn is_mac_os() -> bool {
    cfg!(target_os = "macos")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ease {
    CubicInOut,
    Linear,
}

impl Ease {
    pub fn apply(self, t: f64) -> f64 {
        match self {
            Ease::CubicInOut => default_ease(t),
            Ease::Linear => t.clamp(0.0, 1.0),
        }
    }
}

impl Default for Ease {
    fn default() -> Self {
        Ease::CubicInOut
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interpolate {
    Zoom,
    Linear,
}

impl Default for Interpolate {
    fn default() -> Self {
        Interpolate::Zoom
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewportTransition {
    from: Viewport,
    to: Viewport,
    duration: f64,
    elapsed: f64,
    ease: Ease,
    interpolate: Interpolate,
    finished: bool,
}

pub fn interpolate_zoom(from: Viewport, to: Viewport, t: f64) -> Viewport {
    let scale = from.zoom * (to.zoom / from.zoom).powf(t);
    let x = (from.x - to.x * scale / to.zoom) * (1.0 - t) + from.x * t;
    let y = (from.y - to.y * scale / to.zoom) * (1.0 - t) + from.y * t;
    Viewport {
        x,
        y,
        zoom: scale,
    }
}

pub fn interpolate_linear(from: Viewport, to: Viewport, t: f64) -> Viewport {
    Viewport {
        x: from.x + (to.x - from.x) * t,
        y: from.y + (to.y - from.y) * t,
        zoom: from.zoom + (to.zoom - from.zoom) * t,
    }
}

impl ViewportTransition {
    pub fn new(from: Viewport, to: Viewport, duration: f64) -> Self {
        ViewportTransition {
            from,
            to,
            duration: duration.max(0.0),
            elapsed: 0.0,
            ease: Ease::CubicInOut,
            interpolate: Interpolate::Zoom,
            finished: false,
        }
    }

    pub fn with_ease(mut self, ease: Ease) -> Self {
        self.ease = ease;
        self
    }

    pub fn with_interpolate(mut self, interpolate: Interpolate) -> Self {
        self.interpolate = interpolate;
        self
    }

    pub fn has_duration(&self) -> bool {
        self.duration > 0.0
    }

    pub fn finished(&self) -> bool {
        self.finished
    }

    pub fn progress(&self) -> f64 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        }
    }

    pub fn current(&self) -> Viewport {
        if self.duration <= 0.0 {
            return self.to;
        }
        let t = self.ease.apply(self.progress());
        match self.interpolate {
            Interpolate::Zoom => interpolate_zoom(self.from, self.to, t),
            Interpolate::Linear => interpolate_linear(self.from, self.to, t),
        }
    }

    pub fn advance(&mut self, delta_ms: f64) -> Viewport {
        if self.finished {
            return self.to;
        }
        self.elapsed += delta_ms.max(0.0);
        if self.elapsed >= self.duration {
            self.finished = true;
            self.to
        } else {
            self.current()
        }
    }

    pub fn finish(&mut self) -> Viewport {
        self.finished = true;
        self.to
    }

    pub fn target(&self) -> Viewport {
        self.to
    }
}

pub fn get_d3_transition(
    from: Viewport,
    to: Viewport,
    duration: f64,
) -> Option<ViewportTransition> {
    if duration > 0.0 {
        Some(ViewportTransition::new(from, to, duration))
    } else {
        None
    }
}

pub fn apply_d3_transition(
    transition: Option<&ViewportTransition>,
    fallback: Viewport,
) -> Viewport {
    match transition {
        Some(transition) => transition.target(),
        None => fallback,
    }
}

pub fn minimize_transform(
    transform: crate::core::geometry::Transform,
    extent: crate::core::geometry::Rect,
    translate_extent: crate::core::geometry::Rect,
) -> crate::core::geometry::Transform {
    if translate_extent.width == 0.0 || translate_extent.height == 0.0 {
        return transform;
    }
    let k = transform.zoom();
    let x = transform.x().clamp(
        extent.x - translate_extent.max_x() * k,
        extent.max_x() - translate_extent.x * k,
    );
    let y = transform.y().clamp(
        extent.y - translate_extent.max_y() * k,
        extent.max_y() - translate_extent.y * k,
    );
    crate::core::geometry::Transform::new(x, y, k)
}
