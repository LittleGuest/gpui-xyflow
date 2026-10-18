use crate::{
    core::{
        geometry::{Position, Rect, Transform, XYPosition},
        node::{Handle, HandleType, InternalNode, Node},
    },
    ui::theme::FlowTheme,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeVariant {
    Default,
    Input,
    Output,
    Group,
}

impl NodeVariant {
    pub fn from_str(value: &str) -> NodeVariant {
        match value {
            "input" => NodeVariant::Input,
            "output" => NodeVariant::Output,
            "group" => NodeVariant::Group,
            _ => NodeVariant::Default,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            NodeVariant::Default => "default",
            NodeVariant::Input => "input",
            NodeVariant::Output => "output",
            NodeVariant::Group => "group",
        }
    }

    pub fn has_source_handle(self) -> bool {
        !matches!(self, NodeVariant::Output)
    }

    pub fn has_target_handle(self) -> bool {
        !matches!(self, NodeVariant::Input)
    }

    pub fn is_group(self) -> bool {
        matches!(self, NodeVariant::Group)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeBox {
    pub id: String,
    pub rect: Rect,
    pub z: f64,
    pub selected: bool,
    pub dragging: bool,
    pub variant: NodeVariant,
    pub label: String,
    pub source_handles: Vec<Handle>,
    pub target_handles: Vec<Handle>,
}

impl NodeBox {
    pub fn from_internal<S: Clone + ToString>(
        node: &InternalNode<S>,
        origin: crate::core::geometry::NodeOrigin,
    ) -> Self {
        let dimensions = node.size();
        let position = node.internals.position_absolute;
        let rect = Rect::new(
            position.x - dimensions.width * origin[0],
            position.y - dimensions.height * origin[1],
            dimensions.width,
            dimensions.height,
        );
        let variant = node
            .node_type
            .as_deref()
            .map(NodeVariant::from_str)
            .unwrap_or(NodeVariant::Default);
        let (source_handles, target_handles) = match &node.internals.handle_bounds {
            Some(bounds) => (bounds.source.clone(), bounds.target.clone()),
            None => (Vec::new(), Vec::new()),
        };
        NodeBox {
            id: node.id.clone(),
            rect,
            z: node.internals.z,
            selected: node.selected,
            dragging: node.dragging,
            variant,
            label: node.data.to_string(),
            source_handles,
            target_handles,
        }
    }

    pub fn center(&self) -> XYPosition {
        XYPosition::new(
            self.rect.x + self.rect.width / 2.0,
            self.rect.y + self.rect.height / 2.0,
        )
    }

    pub fn contains(&self, point: XYPosition) -> bool {
        point.x >= self.rect.x
            && point.x <= self.rect.x + self.rect.width
            && point.y >= self.rect.y
            && point.y <= self.rect.y + self.rect.height
    }

    pub fn handle_positions(&self) -> Vec<(HandleType, Position, XYPosition, f64, f64)> {
        let mut out = Vec::new();
        for handle in &self.target_handles {
            out.push((
                HandleType::Target,
                handle.position,
                handle_point(&self.rect, handle),
                handle.x,
                handle.y,
            ));
        }
        for handle in &self.source_handles {
            out.push((
                HandleType::Source,
                handle.position,
                handle_point(&self.rect, handle),
                handle.x,
                handle.y,
            ));
        }
        out
    }
}

pub fn handle_point(rect: &Rect, handle: &Handle) -> XYPosition {
    XYPosition::new(
        rect.x + handle.x + handle.width / 2.0,
        rect.y + handle.y + handle.height / 2.0,
    )
}

pub fn default_handle_x(position: Position, width: f64) -> f64 {
    let half = 3.0;
    match position {
        Position::Top | Position::Bottom => width / 2.0 - half,
        Position::Left => -half,
        Position::Right => width - half,
    }
}

pub fn default_handle_y(position: Position, height: f64) -> f64 {
    let half = 3.0;
    match position {
        Position::Top => -half,
        Position::Bottom => height - half,
        Position::Left | Position::Right => height / 2.0 - half,
    }
}

pub fn default_handles<S: Clone>(node: &Node<S>) -> Vec<Handle> {
    let variant = node
        .node_type
        .as_deref()
        .map(NodeVariant::from_str)
        .unwrap_or(NodeVariant::Default);
    let dimensions = node.size();
    let mut handles = Vec::new();
    if variant.has_target_handle() {
        handles.push(Handle::new(
            node.id.clone(),
            HandleType::Target,
            node.target_position.unwrap_or(Position::Top),
            default_handle_x(
                node.target_position.unwrap_or(Position::Top),
                dimensions.width,
            ),
            default_handle_y(
                node.target_position.unwrap_or(Position::Top),
                dimensions.height,
            ),
        ));
    }
    if variant.has_source_handle() {
        handles.push(Handle::new(
            node.id.clone(),
            HandleType::Source,
            node.source_position.unwrap_or(Position::Bottom),
            default_handle_x(
                node.source_position.unwrap_or(Position::Bottom),
                dimensions.width,
            ),
            default_handle_y(
                node.source_position.unwrap_or(Position::Bottom),
                dimensions.height,
            ),
        ));
    }
    handles
}

pub fn node_fill(theme: &FlowTheme, variant: NodeVariant, selected: bool) -> gpui_kit::Rgba {
    if variant.is_group() {
        return theme.node_group_background;
    }
    let _ = selected;
    theme.node_background
}

pub fn node_border(theme: &FlowTheme, selected: bool) -> gpui_kit::Rgba {
    let _ = selected;
    theme.node_border
}

pub fn node_text_color(theme: &FlowTheme, _variant: NodeVariant) -> gpui_kit::Rgba {
    theme.node_color
}

pub fn node_screen_rect(rect: &Rect, transform: Transform) -> Rect {
    let top_left = transform.apply(XYPosition::new(rect.x, rect.y));
    let zoom = transform.zoom();
    Rect::new(
        top_left.x,
        top_left.y,
        rect.width * zoom,
        rect.height * zoom,
    )
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeRenderList {
    pub nodes: Vec<NodeBox>,
}

impl NodeRenderList {
    pub fn new(mut nodes: Vec<NodeBox>) -> Self {
        nodes.sort_by(|a, b| {
            a.z.partial_cmp(&b.z)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        NodeRenderList { nodes }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn find_at(&self, point: XYPosition) -> Option<&NodeBox> {
        self.nodes.iter().rev().find(|node| node.contains(point))
    }

    pub fn find(&self, id: &str) -> Option<&NodeBox> {
        self.nodes.iter().find(|node| node.id == id)
    }
}
