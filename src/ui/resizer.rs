use crate::{
    core::{
        geometry::{Rect, XYPosition},
        xyresizer::{
            Boundaries, ControlDirection, RESIZER_HANDLE_POSITIONS, RESIZER_LINE_POSITIONS,
            ResizeDimensions, ResizeStartValues, get_dimensions_after_resize, is_handle_position,
            is_line_position,
        },
    },
    ui::theme::FlowTheme,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeHandleKind {
    Handle,
    Line,
}

impl ResizeHandleKind {
    pub fn is_handle(self) -> bool {
        matches!(self, ResizeHandleKind::Handle)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResizeControl {
    pub position: String,
    pub kind: ResizeHandleKind,
    pub rect: Rect,
    pub cursor: &'static str,
}

impl ResizeControl {
    pub fn contains(&self, point: XYPosition) -> bool {
        point.x >= self.rect.x
            && point.x <= self.rect.x + self.rect.width
            && point.y >= self.rect.y
            && point.y <= self.rect.y + self.rect.height
    }
}

pub fn cursor_for_position(position: &str) -> &'static str {
    match position {
        "top" | "bottom" => "ns-resize",
        "left" | "right" => "ew-resize",
        "top-left" | "bottom-right" => "nwse-resize",
        "top-right" | "bottom-left" => "nesw-resize",
        _ => "default",
    }
}

pub fn resize_controls(rect: Rect, handle_size: f64, line_width: f64) -> Vec<ResizeControl> {
    let mut controls = Vec::new();
    for position in RESIZER_HANDLE_POSITIONS {
        if !is_handle_position(position) {
            continue;
        }
        let (x, y) = control_origin(rect, position, handle_size);
        controls.push(ResizeControl {
            position: position.to_string(),
            kind: ResizeHandleKind::Handle,
            rect: Rect::new(x, y, handle_size, handle_size),
            cursor: cursor_for_position(position),
        });
    }
    for position in RESIZER_LINE_POSITIONS {
        if !is_line_position(position) {
            continue;
        }
        let (x, y, width, height) = line_bounds(rect, position, line_width);
        controls.push(ResizeControl {
            position: position.to_string(),
            kind: ResizeHandleKind::Line,
            rect: Rect::new(x, y, width, height),
            cursor: cursor_for_position(position),
        });
    }
    controls
}

fn control_origin(rect: Rect, position: &str, size: f64) -> (f64, f64) {
    let half = size / 2.0;
    let center_x = rect.x + rect.width / 2.0 - half;
    let center_y = rect.y + rect.height / 2.0 - half;
    let left = rect.x - half;
    let top = rect.y - half;
    let right = rect.x + rect.width - half;
    let bottom = rect.y + rect.height - half;
    match position {
        "top-left" => (left, top),
        "top" => (center_x, top),
        "top-right" => (right, top),
        "right" => (right, center_y),
        "bottom-right" => (right, bottom),
        "bottom" => (center_x, bottom),
        "bottom-left" => (left, bottom),
        "left" => (left, center_y),
        _ => (center_x, center_y),
    }
}

fn line_bounds(rect: Rect, position: &str, width: f64) -> (f64, f64, f64, f64) {
    match position {
        "top" => (
            rect.x + width,
            rect.y - width / 2.0,
            rect.width - width * 2.0,
            width,
        ),
        "bottom" => (
            rect.x + width,
            rect.y + rect.height - width / 2.0,
            rect.width - width * 2.0,
            width,
        ),
        "left" => (
            rect.x - width / 2.0,
            rect.y + width,
            width,
            rect.height - width * 2.0,
        ),
        "right" => (
            rect.x + rect.width - width / 2.0,
            rect.y + width,
            width,
            rect.height - width * 2.0,
        ),
        _ => (rect.x, rect.y, rect.width, rect.height),
    }
}

pub fn find_resize_control(
    controls: &[ResizeControl],
    point: XYPosition,
) -> Option<&ResizeControl> {
    controls.iter().find(|control| control.contains(point))
}

pub fn resized_dimensions(
    control: ControlDirection,
    start: ResizeStartValues,
    pointer: XYPosition,
    boundaries: Boundaries,
    keep_aspect_ratio: bool,
    node_origin: crate::core::geometry::NodeOrigin,
    extent: Option<crate::core::geometry::CoordinateExtent>,
    child_extent: Option<crate::core::geometry::CoordinateExtent>,
) -> ResizeDimensions {
    get_dimensions_after_resize(
        start,
        control,
        pointer,
        boundaries,
        keep_aspect_ratio,
        node_origin,
        extent,
        child_extent,
    )
}

pub fn resize_direction(position: &str) -> Option<[f64; 2]> {
    let dx = match position {
        "left" | "top-left" | "bottom-left" => -1.0,
        "right" | "top-right" | "bottom-right" => 1.0,
        _ => 0.0,
    };
    let dy = match position {
        "top" | "top-left" | "top-right" => -1.0,
        "bottom" | "bottom-left" | "bottom-right" => 1.0,
        _ => 0.0,
    };
    if dx == 0.0 && dy == 0.0 {
        None
    } else {
        Some([dx, dy])
    }
}

pub fn resize_handle_fill(theme: &FlowTheme) -> gpui_kit::Rgba {
    theme.resize_background
}

pub fn resize_border_fill(theme: &FlowTheme) -> gpui_kit::Rgba {
    theme.selection_border
}
