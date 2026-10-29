use std::sync::Arc;

use gpui_kit::{AnyElement, App, Bounds, Pixels, Window};

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeResizeInfo {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub type NodeResizeHandler = Arc<dyn Fn(&str, NodeResizeInfo) + Send + Sync>;

#[derive(Clone, Default)]
pub struct NodeResizerCallbacks {
    pub on_resize_start: Option<NodeResizeHandler>,
    pub on_resize: Option<NodeResizeHandler>,
    pub on_resize_end: Option<NodeResizeHandler>,
}

impl std::fmt::Debug for NodeResizerCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeResizerCallbacks")
            .finish_non_exhaustive()
    }
}

impl PartialEq for NodeResizerCallbacks {
    fn eq(&self, other: &Self) -> bool {
        fn same(left: &Option<NodeResizeHandler>, right: &Option<NodeResizeHandler>) -> bool {
            match (left, right) {
                (None, None) => true,
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                _ => false,
            }
        }
        same(&self.on_resize_start, &other.on_resize_start)
            && same(&self.on_resize, &other.on_resize)
            && same(&self.on_resize_end, &other.on_resize_end)
    }
}

/// Builds a real GPUI element rendered on top of one resize control.
///
/// 控制点元素只负责外观；缩放交互仍由画布命中测试驱动。
pub type ResizeControlRenderer = Arc<
    dyn Fn(&ResizeControl, Bounds<Pixels>, f64, &mut Window, &mut App) -> AnyElement + Send + Sync,
>;

#[derive(Clone)]
pub struct NodeResizerProps {
    pub node_ids: Vec<String>,
    pub is_visible: Option<bool>,
    pub min_width: f64,
    pub min_height: f64,
    pub max_width: f64,
    pub max_height: f64,
    pub keep_aspect_ratio: bool,
    pub handle_size: f64,
    pub line_width: f64,
    pub color: Option<gpui_kit::Rgba>,
    pub callbacks: NodeResizerCallbacks,
    pub control_renderer: Option<ResizeControlRenderer>,
}

impl std::fmt::Debug for NodeResizerProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeResizerProps")
            .field("node_ids", &self.node_ids)
            .field("is_visible", &self.is_visible)
            .field("min_width", &self.min_width)
            .field("min_height", &self.min_height)
            .field("max_width", &self.max_width)
            .field("max_height", &self.max_height)
            .field("keep_aspect_ratio", &self.keep_aspect_ratio)
            .field("handle_size", &self.handle_size)
            .field("line_width", &self.line_width)
            .field("color", &self.color)
            .field("callbacks", &self.callbacks)
            .field("control_renderer", &self.control_renderer.is_some())
            .finish()
    }
}

impl PartialEq for NodeResizerProps {
    fn eq(&self, other: &Self) -> bool {
        let same_renderer = match (&self.control_renderer, &other.control_renderer) {
            (None, None) => true,
            (Some(left), Some(right)) => Arc::ptr_eq(left, right),
            _ => false,
        };
        self.node_ids == other.node_ids
            && self.is_visible == other.is_visible
            && self.min_width == other.min_width
            && self.min_height == other.min_height
            && self.max_width == other.max_width
            && self.max_height == other.max_height
            && self.keep_aspect_ratio == other.keep_aspect_ratio
            && self.handle_size == other.handle_size
            && self.line_width == other.line_width
            && self.color == other.color
            && self.callbacks == other.callbacks
            && same_renderer
    }
}

impl Default for NodeResizerProps {
    fn default() -> Self {
        NodeResizerProps {
            node_ids: Vec::new(),
            is_visible: None,
            min_width: 10.0,
            min_height: 10.0,
            max_width: f64::MAX,
            max_height: f64::MAX,
            keep_aspect_ratio: false,
            handle_size: 10.0,
            line_width: 4.0,
            color: None,
            callbacks: NodeResizerCallbacks::default(),
            control_renderer: None,
        }
    }
}

impl NodeResizerProps {
    pub fn with_node_ids(mut self, node_ids: Vec<String>) -> Self {
        self.node_ids = node_ids;
        self
    }

    pub fn with_visibility(mut self, visible: Option<bool>) -> Self {
        self.is_visible = visible;
        self
    }

    pub fn with_min_size(mut self, width: f64, height: f64) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }

    pub fn with_max_size(mut self, width: f64, height: f64) -> Self {
        self.max_width = width;
        self.max_height = height;
        self
    }

    pub fn with_keep_aspect_ratio(mut self, keep: bool) -> Self {
        self.keep_aspect_ratio = keep;
        self
    }

    pub fn with_color(mut self, color: gpui_kit::Rgba) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_control_renderer(mut self, renderer: ResizeControlRenderer) -> Self {
        self.control_renderer = Some(renderer);
        self
    }

    pub fn boundaries(&self) -> Boundaries {
        Boundaries::new(
            self.min_width,
            self.min_height,
            self.max_width,
            self.max_height,
        )
    }
}

impl NodeResizeInfo {
    pub fn from_rect(rect: Rect) -> NodeResizeInfo {
        NodeResizeInfo {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
        }
    }
}

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

pub fn resize_change_to_node_changes<S>(
    node_id: &str,
    change: &crate::core::xyresizer::XYResizerChange,
    resizing: bool,
) -> Vec<crate::core::change::NodeChange<S>> {
    vec![
        crate::core::change::NodeChange::Position(crate::core::change::NodePositionChange {
            id: node_id.to_string(),
            position: Some(XYPosition::new(change.x, change.y)),
            position_absolute: None,
            dragging: None,
        }),
        crate::core::change::NodeChange::Dimensions(crate::core::change::NodeDimensionChange {
            id: node_id.to_string(),
            dimensions: Some(crate::core::geometry::Dimensions::new(
                change.width,
                change.height,
            )),
            resizing: Some(resizing),
            set_attributes: None,
        }),
    ]
}
