use crate::core::geometry::{Rect, Transform, Viewport, XYPosition};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ViewportRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        ViewportRect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_rect(rect: Rect) -> Self {
        ViewportRect {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }

    pub fn to_rect(self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }

    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    pub fn contains(self, point: XYPosition) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    pub fn intersect(self, other: ViewportRect) -> Option<ViewportRect> {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        if right <= left || bottom <= top {
            return None;
        }
        Some(ViewportRect::new(left, top, right - left, bottom - top))
    }

    pub fn union(self, other: ViewportRect) -> ViewportRect {
        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        ViewportRect::new(left, top, right - left, bottom - top)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenMapping {
    pub transform: Transform,
}

impl ScreenMapping {
    pub fn new(transform: Transform) -> Self {
        ScreenMapping { transform }
    }

    pub fn identity() -> Self {
        ScreenMapping {
            transform: Transform::IDENTITY,
        }
    }

    pub fn flow_to_screen(&self, point: XYPosition) -> XYPosition {
        self.transform.apply(point)
    }

    pub fn screen_to_flow(&self, point: XYPosition) -> XYPosition {
        self.transform.invert(point)
    }

    pub fn flow_rect_to_screen(&self, rect: Rect) -> Rect {
        let top_left = self.flow_to_screen(XYPosition::new(rect.x, rect.y));
        let zoom = self.transform.zoom();
        Rect::new(
            top_left.x,
            top_left.y,
            rect.width * zoom,
            rect.height * zoom,
        )
    }

    pub fn screen_rect_to_flow(&self, rect: Rect) -> Rect {
        let top_left = self.screen_to_flow(XYPosition::new(rect.x, rect.y));
        let zoom = self.transform.zoom();
        let safe_zoom = if zoom == 0.0 { 1.0 } else { zoom };
        Rect::new(
            top_left.x,
            top_left.y,
            rect.width / safe_zoom,
            rect.height / safe_zoom,
        )
    }

    pub fn visible_flow_rect(&self, width: f64, height: f64) -> Rect {
        self.screen_rect_to_flow(Rect::new(0.0, 0.0, width, height))
    }

    pub fn is_rect_visible(&self, rect: Rect, width: f64, height: f64) -> bool {
        let screen = self.flow_rect_to_screen(rect);
        let viewport = ViewportRect::new(0.0, 0.0, width, height);
        ViewportRect::from_rect(screen)
            .intersect(viewport)
            .is_some()
    }
}

pub fn viewport_origin(viewport: Viewport) -> XYPosition {
    XYPosition::new(viewport.x, viewport.y)
}

pub fn viewport_zoom(viewport: Viewport) -> f64 {
    if viewport.zoom == 0.0 {
        1.0
    } else {
        viewport.zoom
    }
}

pub fn viewport_center(viewport: Viewport, width: f64, height: f64) -> XYPosition {
    let zoom = viewport_zoom(viewport);
    XYPosition::new(
        (width / 2.0 - viewport.x) / zoom,
        (height / 2.0 - viewport.y) / zoom,
    )
}

pub fn zoom_clamped(zoom: f64, min_zoom: f64, max_zoom: f64) -> f64 {
    zoom.max(min_zoom).min(max_zoom)
}
