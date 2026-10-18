use crate::{
    core::{
        connect::{ConnectionLine, ConnectionState},
        edge::{ConnectionLineType, PathOptions},
        geometry::XYPosition,
    },
    ui::{
        edge_renderer::connection_line_path,
        paths::{SvgPath, parse_svg_path},
        theme::FlowTheme,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionVisual {
    pub path: String,
    pub valid: Option<bool>,
    pub is_connecting: bool,
}

impl ConnectionVisual {
    pub fn svg_path(&self) -> SvgPath {
        parse_svg_path(&self.path)
    }

    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }
}

pub fn connection_visual<S: Clone>(
    state: &ConnectionState<S>,
    line_type: ConnectionLineType,
    pointer: XYPosition,
    transform: crate::core::geometry::Transform,
    path_options: Option<&PathOptions>,
) -> Option<ConnectionVisual> {
    let line = match state {
        ConnectionState::InProgress(line) => line,
        ConnectionState::Idle(_) => return None,
    };
    let renderer_pointer = transform.invert(pointer);
    let from = XYPosition::new(line.from.x, line.from.y);
    let to = match &line.to_handle {
        Some(handle) => {
            if let Some(target) = &line.to_node {
                let dimensions = target.size();
                let position = target.internals.position_absolute;
                XYPosition::new(
                    position.x + dimensions.width * handle.x + handle.width / 2.0,
                    position.y + dimensions.height * handle.y + handle.height / 2.0,
                )
            } else {
                renderer_pointer
            }
        }
        None => renderer_pointer,
    };
    let connection_line = ConnectionLine {
        source_x: from.x,
        source_y: from.y,
        source_position: line.from_position,
        target_x: to.x,
        target_y: to.y,
        target_position: line.to_position_or_opposite(),
    };
    let graphics = connection_line_path(&connection_line, line_type, path_options);
    Some(ConnectionVisual {
        path: graphics.path,
        valid: line.is_valid,
        is_connecting: true,
    })
}

pub fn connection_stroke(theme: &FlowTheme, valid: Option<bool>) -> gpui_kit::Rgba {
    match valid {
        Some(true) => theme.connection_line_stroke,
        Some(false) => theme.handle_border,
        None => theme.connection_line_stroke,
    }
}

pub fn connection_stroke_width(theme: &FlowTheme) -> f32 {
    theme.connection_line_stroke_width
}

pub fn is_connecting<S: Clone>(state: &ConnectionState<S>) -> bool {
    matches!(state, ConnectionState::InProgress(_))
}

pub fn connection_anchor<S: Clone>(state: &ConnectionState<S>) -> Option<XYPosition> {
    match state {
        ConnectionState::InProgress(line) => Some(XYPosition::new(line.from.x, line.from.y)),
        ConnectionState::Idle(_) => None,
    }
}
