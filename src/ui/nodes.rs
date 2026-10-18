use crate::core::geometry::Position;
use crate::core::node::{Handle, HandleType, Node};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BuiltInNodeKind {
    #[default]
    Default,
    Input,
    Output,
    Group,
}

impl BuiltInNodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BuiltInNodeKind::Default => "default",
            BuiltInNodeKind::Input => "input",
            BuiltInNodeKind::Output => "output",
            BuiltInNodeKind::Group => "group",
        }
    }

    pub fn node_type(self) -> String {
        self.as_str().to_string()
    }

    pub fn from_str(value: &str) -> BuiltInNodeKind {
        match value {
            "input" => BuiltInNodeKind::Input,
            "output" => BuiltInNodeKind::Output,
            "group" => BuiltInNodeKind::Group,
            _ => BuiltInNodeKind::Default,
        }
    }

    pub fn has_source_handle(self) -> bool {
        !matches!(self, BuiltInNodeKind::Output | BuiltInNodeKind::Group)
    }

    pub fn has_target_handle(self) -> bool {
        !matches!(self, BuiltInNodeKind::Input | BuiltInNodeKind::Group)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeProps<S = String> {
    pub id: String,
    pub data: S,
    pub label: Option<String>,
    pub is_connectable: bool,
    pub target_position: Position,
    pub source_position: Position,
    pub selected: bool,
    pub dragging: bool,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

impl<S: Clone> NodeProps<S> {
    pub fn from_node(node: &Node<S>, label: Option<String>) -> Self {
        NodeProps {
            id: node.id.clone(),
            data: node.data.clone(),
            label,
            is_connectable: node.connectable.unwrap_or(true),
            target_position: node.target_position.unwrap_or(Position::Top),
            source_position: node.source_position.unwrap_or(Position::Bottom),
            selected: node.selected,
            dragging: node.dragging,
            width: node.width.or(node.initial_width),
            height: node.height.or(node.initial_height),
        }
    }
}

impl<S> Default for NodeProps<S>
where
    S: Default,
{
    fn default() -> Self {
        NodeProps {
            id: String::new(),
            data: S::default(),
            label: None,
            is_connectable: true,
            target_position: Position::Top,
            source_position: Position::Bottom,
            selected: false,
            dragging: false,
            width: None,
            height: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeHandleSpec {
    pub handle_type: HandleType,
    pub position: Position,
    pub is_connectable: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeComponentOutput {
    pub kind: BuiltInNodeKind,
    pub label: Option<String>,
    pub handles: Vec<NodeHandleSpec>,
}

pub fn default_node_handles<S>(props: &NodeProps<S>) -> NodeComponentOutput {
    let mut handles = vec![NodeHandleSpec {
        handle_type: HandleType::Target,
        position: props.target_position,
        is_connectable: props.is_connectable,
    }];
    handles.push(NodeHandleSpec {
        handle_type: HandleType::Source,
        position: props.source_position,
        is_connectable: props.is_connectable,
    });
    NodeComponentOutput {
        kind: BuiltInNodeKind::Default,
        label: props.label.clone(),
        handles,
    }
}

pub fn input_node_handles<S>(props: &NodeProps<S>) -> NodeComponentOutput {
    NodeComponentOutput {
        kind: BuiltInNodeKind::Input,
        label: props.label.clone(),
        handles: vec![NodeHandleSpec {
            handle_type: HandleType::Source,
            position: props.source_position,
            is_connectable: props.is_connectable,
        }],
    }
}

pub fn output_node_handles<S>(props: &NodeProps<S>) -> NodeComponentOutput {
    NodeComponentOutput {
        kind: BuiltInNodeKind::Output,
        label: props.label.clone(),
        handles: vec![NodeHandleSpec {
            handle_type: HandleType::Target,
            position: props.target_position,
            is_connectable: props.is_connectable,
        }],
    }
}

pub fn group_node_handles<S>(props: &NodeProps<S>) -> NodeComponentOutput {
    NodeComponentOutput {
        kind: BuiltInNodeKind::Group,
        label: props.label.clone(),
        handles: Vec::new(),
    }
}

pub fn built_in_node_output<S>(kind: BuiltInNodeKind, props: &NodeProps<S>) -> NodeComponentOutput {
    match kind {
        BuiltInNodeKind::Default => default_node_handles(props),
        BuiltInNodeKind::Input => input_node_handles(props),
        BuiltInNodeKind::Output => output_node_handles(props),
        BuiltInNodeKind::Group => group_node_handles(props),
    }
}

pub fn node_handles_for<S: Clone>(node: &Node<S>, label: Option<String>) -> Vec<Handle> {
    let kind = BuiltInNodeKind::from_str(node.node_type.as_deref().unwrap_or("default"));
    let props = NodeProps::from_node(node, label);
    let output = built_in_node_output(kind, &props);
    output
        .handles
        .into_iter()
        .map(|spec| Handle {
            id: None,
            node_id: node.id.clone(),
            x: 0.0,
            y: 0.0,
            position: spec.position,
            handle_type: spec.handle_type,
            width: 0.0,
            height: 0.0,
        })
        .collect()
}

pub fn node_label_default<S: Clone>(node: &Node<S>) -> Option<String> {
    node.node_type
        .as_deref()
        .filter(|node_type| *node_type != "default")
        .map(|node_type| BuiltInNodeKind::from_str(node_type).as_str().to_string())
}
