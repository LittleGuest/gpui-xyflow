use crate::core::geometry::{Dimensions, NodeOrigin, Position, XYPosition};

#[derive(Clone, Debug, PartialEq)]
pub struct Node<S = String> {
    pub id: String,
    pub position: XYPosition,
    pub data: S,
    pub node_type: Option<String>,
    pub source_position: Option<Position>,
    pub target_position: Option<Position>,
    pub hidden: bool,
    pub selected: bool,
    pub dragging: bool,
    pub draggable: Option<bool>,
    pub selectable: Option<bool>,
    pub connectable: Option<bool>,
    pub deletable: Option<bool>,
    pub focusable: Option<bool>,
    pub drag_handle: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub initial_width: Option<f64>,
    pub initial_height: Option<f64>,
    pub parent_id: Option<String>,
    pub z_index: Option<f64>,
    pub extent: Option<NodeExtent>,
    pub expand_parent: bool,
    pub aria_label: Option<String>,
    pub origin: Option<NodeOrigin>,
    pub handles: Vec<Handle>,
    pub measured: MeasuredDimensions,
}

impl<S: Default> Default for Node<S> {
    fn default() -> Self {
        Self::new(String::new(), XYPosition::ZERO, S::default())
    }
}

impl<S> Node<S> {
    pub fn new(id: impl Into<String>, position: XYPosition, data: S) -> Self {
        Node {
            id: id.into(),
            position,
            data,
            node_type: None,
            source_position: None,
            target_position: None,
            hidden: false,
            selected: false,
            dragging: false,
            draggable: None,
            selectable: None,
            connectable: None,
            deletable: None,
            focusable: None,
            drag_handle: None,
            width: None,
            height: None,
            initial_width: None,
            initial_height: None,
            parent_id: None,
            z_index: None,
            extent: None,
            expand_parent: false,
            aria_label: None,
            origin: None,
            handles: Vec::new(),
            measured: MeasuredDimensions::default(),
        }
    }

    pub fn with_type(mut self, node_type: impl Into<String>) -> Self {
        self.node_type = Some(node_type.into());
        self
    }

    pub fn with_size(mut self, width: f64, height: f64) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    pub fn with_initial_size(mut self, width: f64, height: f64) -> Self {
        self.initial_width = Some(width);
        self.initial_height = Some(height);
        self
    }

    pub fn with_measured(mut self, width: f64, height: f64) -> Self {
        self.measured = MeasuredDimensions::new(width, height);
        self
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    pub fn with_handles(mut self, handles: Vec<Handle>) -> Self {
        self.handles = handles;
        self
    }

    pub fn with_extent(mut self, extent: NodeExtent) -> Self {
        self.extent = Some(extent);
        self
    }

    pub fn with_origin(mut self, origin: NodeOrigin) -> Self {
        self.origin = Some(origin);
        self
    }

    pub fn with_z_index(mut self, z_index: f64) -> Self {
        self.z_index = Some(z_index);
        self
    }

    pub fn with_source_position(mut self, position: Position) -> Self {
        self.source_position = Some(position);
        self
    }

    pub fn with_target_position(mut self, position: Position) -> Self {
        self.target_position = Some(position);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn with_expand_parent(mut self, expand_parent: bool) -> Self {
        self.expand_parent = expand_parent;
        self
    }

    pub fn dimensions(&self) -> MeasuredDimensions {
        MeasuredDimensions {
            width: Some(
                self.measured
                    .width
                    .or(self.width)
                    .or(self.initial_width)
                    .unwrap_or(0.0),
            ),
            height: Some(
                self.measured
                    .height
                    .or(self.height)
                    .or(self.initial_height)
                    .unwrap_or(0.0),
            ),
        }
    }

    pub fn size(&self) -> Dimensions {
        let dimensions = self.dimensions();
        Dimensions::new(
            dimensions.width.unwrap_or(0.0),
            dimensions.height.unwrap_or(0.0),
        )
    }

    pub fn has_dimensions(&self) -> bool {
        self.measured
            .width
            .or(self.width)
            .or(self.initial_width)
            .is_some()
            && self
                .measured
                .height
                .or(self.height)
                .or(self.initial_height)
                .is_some()
    }

    pub fn is_draggable(&self, nodes_draggable: bool) -> bool {
        self.draggable.unwrap_or(nodes_draggable)
    }

    pub fn is_selectable(&self, elements_selectable: bool) -> bool {
        self.selectable.unwrap_or(elements_selectable)
    }

    pub fn is_connectable(&self, nodes_connectable: bool) -> bool {
        self.connectable.unwrap_or(nodes_connectable)
    }

    pub fn is_deletable(&self) -> bool {
        self.deletable.unwrap_or(true)
    }

    pub fn is_focusable(&self, nodes_focusable: bool) -> bool {
        self.focusable.unwrap_or(nodes_focusable)
    }

    pub fn type_name(&self) -> &str {
        self.node_type.as_deref().unwrap_or("default")
    }

    pub fn origin_or(&self, fallback: NodeOrigin) -> NodeOrigin {
        self.origin.unwrap_or(fallback)
    }

    pub fn position_with_origin(&self, fallback_origin: NodeOrigin) -> XYPosition {
        let dimensions = self.size();
        let origin = self.origin_or(fallback_origin);
        XYPosition::new(
            self.position.x - dimensions.width * origin[0],
            self.position.y - dimensions.height * origin[1],
        )
    }
}

impl Node<String> {
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.data = label.into();
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MeasuredDimensions {
    pub width: Option<f64>,
    pub height: Option<f64>,
}

impl MeasuredDimensions {
    pub fn new(width: f64, height: f64) -> Self {
        MeasuredDimensions {
            width: Some(width),
            height: Some(height),
        }
    }

    pub fn is_measured(&self) -> bool {
        self.width.is_some() && self.height.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeExtent {
    Parent,
    Coordinates([[f64; 2]; 2]),
}

impl NodeExtent {
    pub fn coordinates(self) -> Option<[[f64; 2]; 2]> {
        match self {
            NodeExtent::Parent => None,
            NodeExtent::Coordinates(extent) => Some(extent),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleType {
    Source,
    Target,
}

impl HandleType {
    pub fn as_str(self) -> &'static str {
        match self {
            HandleType::Source => "source",
            HandleType::Target => "target",
        }
    }

    pub fn from_str(value: &str) -> Option<HandleType> {
        match value {
            "source" => Some(HandleType::Source),
            "target" => Some(HandleType::Target),
            _ => None,
        }
    }

    pub fn opposite(self) -> HandleType {
        match self {
            HandleType::Source => HandleType::Target,
            HandleType::Target => HandleType::Source,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Handle {
    pub id: Option<String>,
    pub node_id: String,
    pub x: f64,
    pub y: f64,
    pub position: Position,
    pub handle_type: HandleType,
    pub width: f64,
    pub height: f64,
}

impl Handle {
    pub fn new(
        node_id: impl Into<String>,
        handle_type: HandleType,
        position: Position,
        x: f64,
        y: f64,
    ) -> Self {
        Handle {
            id: None,
            node_id: node_id.into(),
            x,
            y,
            position,
            handle_type,
            width: 1.0,
            height: 1.0,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn with_size(mut self, width: f64, height: f64) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeHandleBounds {
    pub source: Vec<Handle>,
    pub target: Vec<Handle>,
}

impl NodeHandleBounds {
    pub fn is_empty(&self) -> bool {
        self.source.is_empty() && self.target.is_empty()
    }

    pub fn all(&self) -> impl Iterator<Item = &Handle> {
        self.source.iter().chain(self.target.iter())
    }

    pub fn handles_of(&self, handle_type: HandleType) -> &[Handle] {
        match handle_type {
            HandleType::Source => &self.source,
            HandleType::Target => &self.target,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeBounds {
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Internals<S = String> {
    pub position_absolute: XYPosition,
    pub z: f64,
    pub root_parent_index: Option<usize>,
    pub user_node: Node<S>,
    pub handle_bounds: Option<NodeHandleBounds>,
    pub bounds: Option<NodeBounds>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InternalNode<S = String> {
    pub id: String,
    pub position: XYPosition,
    pub data: S,
    pub node_type: Option<String>,
    pub source_position: Option<Position>,
    pub target_position: Option<Position>,
    pub hidden: bool,
    pub selected: bool,
    pub dragging: bool,
    pub draggable: Option<bool>,
    pub selectable: Option<bool>,
    pub connectable: Option<bool>,
    pub deletable: Option<bool>,
    pub focusable: Option<bool>,
    pub drag_handle: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub initial_width: Option<f64>,
    pub initial_height: Option<f64>,
    pub parent_id: Option<String>,
    pub z_index: Option<f64>,
    pub extent: Option<NodeExtent>,
    pub expand_parent: bool,
    pub aria_label: Option<String>,
    pub origin: Option<NodeOrigin>,
    pub measured: MeasuredDimensions,
    pub internals: Internals<S>,
}

impl<S: Clone> InternalNode<S> {
    pub fn from_user_node(
        node: Node<S>,
        position_absolute: XYPosition,
        z: f64,
        handle_bounds: Option<NodeHandleBounds>,
    ) -> Self {
        InternalNode {
            internals: Internals {
                position_absolute,
                z,
                root_parent_index: None,
                user_node: node.clone(),
                handle_bounds,
                bounds: None,
            },
            id: node.id,
            position: node.position,
            data: node.data,
            node_type: node.node_type,
            source_position: node.source_position,
            target_position: node.target_position,
            hidden: node.hidden,
            selected: node.selected,
            dragging: node.dragging,
            draggable: node.draggable,
            selectable: node.selectable,
            connectable: node.connectable,
            deletable: node.deletable,
            focusable: node.focusable,
            drag_handle: node.drag_handle,
            width: node.width,
            height: node.height,
            initial_width: node.initial_width,
            initial_height: node.initial_height,
            parent_id: node.parent_id,
            z_index: node.z_index,
            extent: node.extent,
            expand_parent: node.expand_parent,
            aria_label: node.aria_label,
            origin: node.origin,
            measured: node.measured,
        }
    }

    pub fn dimensions(&self) -> MeasuredDimensions {
        MeasuredDimensions {
            width: Some(
                self.measured
                    .width
                    .or(self.width)
                    .or(self.initial_width)
                    .unwrap_or(0.0),
            ),
            height: Some(
                self.measured
                    .height
                    .or(self.height)
                    .or(self.initial_height)
                    .unwrap_or(0.0),
            ),
        }
    }

    pub fn size(&self) -> Dimensions {
        let dimensions = self.dimensions();
        Dimensions::new(
            dimensions.width.unwrap_or(0.0),
            dimensions.height.unwrap_or(0.0),
        )
    }

    pub fn type_name(&self) -> &str {
        self.node_type.as_deref().unwrap_or("default")
    }

    pub fn origin_or(&self, fallback: NodeOrigin) -> NodeOrigin {
        self.origin.unwrap_or(fallback)
    }

    pub fn position_with_origin(&self, fallback_origin: NodeOrigin) -> XYPosition {
        let dimensions = self.size();
        let origin = self.origin_or(fallback_origin);
        XYPosition::new(
            self.position.x - dimensions.width * origin[0],
            self.position.y - dimensions.height * origin[1],
        )
    }

    pub fn handle_bounds_or(&self, handles: &[Handle]) -> NodeHandleBounds {
        self.internals
            .handle_bounds
            .clone()
            .unwrap_or_else(|| bounds_from_handles(handles))
    }
}

pub fn bounds_from_handles(handles: &[Handle]) -> NodeHandleBounds {
    let mut bounds = NodeHandleBounds::default();
    for handle in handles {
        match handle.handle_type {
            HandleType::Source => bounds.source.push(handle.clone()),
            HandleType::Target => bounds.target.push(handle.clone()),
        }
    }
    bounds
}

pub type NodeLookup<S = String> = std::collections::BTreeMap<String, InternalNode<S>>;
pub type ParentLookup<S = String> =
    std::collections::BTreeMap<String, std::collections::BTreeMap<String, InternalNode<S>>>;

#[derive(Clone, Debug, PartialEq)]
pub struct NodeDragItem {
    pub id: String,
    pub position: XYPosition,
    pub distance: XYPosition,
    pub measured: MeasuredDimensions,
    pub position_absolute: XYPosition,
    pub extent: Option<NodeExtent>,
    pub parent_id: Option<String>,
    pub origin: Option<NodeOrigin>,
    pub expand_parent: bool,
    pub dragging: bool,
}

impl NodeDragItem {
    pub fn from_internal<S: Clone>(node: &InternalNode<S>, pointer: XYPosition) -> Self {
        NodeDragItem {
            id: node.id.clone(),
            position: node.position,
            distance: XYPosition::new(
                pointer.x - node.internals.position_absolute.x,
                pointer.y - node.internals.position_absolute.y,
            ),
            measured: MeasuredDimensions {
                width: Some(node.size().width),
                height: Some(node.size().height),
            },
            position_absolute: node.internals.position_absolute,
            extent: node.extent,
            parent_id: node.parent_id.clone(),
            origin: node.origin,
            expand_parent: node.expand_parent,
            dragging: false,
        }
    }

    pub fn dimensions(&self) -> MeasuredDimensions {
        self.measured
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeProps<T> {
    pub id: String,
    pub data: T,
    pub node_type: String,
    pub dragging: bool,
    pub z_index: f64,
    pub selectable: bool,
    pub deletable: bool,
    pub selected: bool,
    pub draggable: bool,
    pub is_connectable: bool,
    pub position_absolute_x: f64,
    pub position_absolute_y: f64,
    pub width: f64,
    pub height: f64,
    pub source_position: Option<Position>,
    pub target_position: Option<Position>,
    pub drag_handle: Option<String>,
    pub parent_id: Option<String>,
}

impl<T: Default> Default for NodeProps<T> {
    fn default() -> Self {
        NodeProps {
            id: String::new(),
            data: T::default(),
            node_type: "default".to_string(),
            dragging: false,
            z_index: 0.0,
            selectable: true,
            deletable: true,
            selected: false,
            draggable: true,
            is_connectable: true,
            position_absolute_x: 0.0,
            position_absolute_y: 0.0,
            width: 0.0,
            height: 0.0,
            source_position: None,
            target_position: None,
            drag_handle: None,
            parent_id: None,
        }
    }
}
