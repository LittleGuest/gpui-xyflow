use gpui_kit::Pixels;

use crate::{
    core::{
        geometry::{Position, Rect, XYPosition},
        node::{Handle, HandleType, NodeHandleBounds},
    },
    ui::theme::FlowTheme,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleSide {
    Top,
    Right,
    Bottom,
    Left,
}

impl HandleSide {
    pub fn from_position(position: Position) -> HandleSide {
        match position {
            Position::Top => HandleSide::Top,
            Position::Right => HandleSide::Right,
            Position::Bottom => HandleSide::Bottom,
            Position::Left => HandleSide::Left,
        }
    }

    pub fn to_position(self) -> Position {
        match self {
            HandleSide::Top => Position::Top,
            HandleSide::Right => Position::Right,
            HandleSide::Bottom => Position::Bottom,
            HandleSide::Left => Position::Left,
        }
    }

    pub fn is_vertical(self) -> bool {
        matches!(self, HandleSide::Top | HandleSide::Bottom)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HandleGeometry {
    pub id: Option<String>,
    pub node_id: String,
    pub handle_type: HandleType,
    pub side: HandleSide,
    pub center: XYPosition,
    pub radius: f64,
}

impl HandleGeometry {
    pub fn from_handle(rect: &Rect, handle: &Handle, radius: f64) -> Self {
        HandleGeometry {
            id: handle.id.clone(),
            node_id: handle.node_id.clone(),
            handle_type: handle.handle_type,
            side: HandleSide::from_position(handle.position),
            center: XYPosition::new(
                rect.x + rect.width * handle.x + handle.width / 2.0,
                rect.y + rect.height * handle.y + handle.height / 2.0,
            ),
            radius,
        }
    }

    pub fn contains(&self, point: XYPosition, tolerance: f64) -> bool {
        let limit = self.radius + tolerance;
        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;
        dx * dx + dy * dy <= limit * limit
    }

    pub fn bounding_rect(&self) -> Rect {
        Rect::new(
            self.center.x - self.radius,
            self.center.y - self.radius,
            self.radius * 2.0,
            self.radius * 2.0,
        )
    }
}

pub fn handle_geometry_list(rect: &Rect, handles: &[Handle], radius: f64) -> Vec<HandleGeometry> {
    handles
        .iter()
        .map(|handle| HandleGeometry::from_handle(rect, handle, radius))
        .collect()
}

pub fn handle_fill(theme: &FlowTheme, handle_type: HandleType) -> gpui_kit::Rgba {
    match handle_type {
        HandleType::Source => theme.handle_background,
        HandleType::Target => theme.handle_background,
    }
}

pub fn handle_border(theme: &FlowTheme) -> gpui_kit::Rgba {
    theme.handle_border
}

pub fn handle_radius(theme: &FlowTheme) -> Pixels {
    theme.handle_radius()
}

pub fn find_handle_at(
    handles: &[HandleGeometry],
    point: XYPosition,
    tolerance: f64,
) -> Option<&HandleGeometry> {
    handles
        .iter()
        .find(|handle| handle.contains(point, tolerance))
}

pub fn find_handle_at_with_type(
    handles: &[HandleGeometry],
    point: XYPosition,
    tolerance: f64,
    handle_type: HandleType,
) -> Option<&HandleGeometry> {
    handles
        .iter()
        .filter(|handle| handle.handle_type == handle_type)
        .find(|handle| handle.contains(point, tolerance))
}

pub fn handle_offset(side: HandleSide, size: f64) -> (f64, f64) {
    match side {
        HandleSide::Top => (0.0, -size / 2.0),
        HandleSide::Bottom => (0.0, size / 2.0),
        HandleSide::Left => (-size / 2.0, 0.0),
        HandleSide::Right => (size / 2.0, 0.0),
    }
}

pub fn cursor_for_handle(handle_type: HandleType) -> &'static str {
    match handle_type {
        HandleType::Source => "crosshair",
        HandleType::Target => "crosshair",
    }
}

pub fn handle_bounds_from_node<S: Clone>(
    node: &crate::core::node::InternalNode<S>,
) -> NodeHandleBounds {
    node.handle_bounds_or(&node.internals.user_node.handles)
}
