use crate::core::edge::Edge;
use crate::core::node::Node;

#[derive(Clone, Debug, PartialEq)]
pub struct NodeDimensionChange {
    pub id: String,
    pub dimensions: Option<crate::core::geometry::Dimensions>,
    pub resizing: Option<bool>,
    pub set_attributes: Option<SetAttributes>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetAttributes {
    Both,
    Width,
    Height,
}

impl SetAttributes {
    pub fn sets_width(self) -> bool {
        matches!(self, SetAttributes::Both | SetAttributes::Width)
    }

    pub fn sets_height(self) -> bool {
        matches!(self, SetAttributes::Both | SetAttributes::Height)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodePositionChange {
    pub id: String,
    pub position: Option<crate::core::geometry::XYPosition>,
    pub position_absolute: Option<crate::core::geometry::XYPosition>,
    pub dragging: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionChange {
    pub id: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RemoveChange {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AddNodeChange<S> {
    pub item: Node<S>,
    pub index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplaceNodeChange<S> {
    pub id: String,
    pub item: Node<S>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AddEdgeChange<S> {
    pub item: Edge<S>,
    pub index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReplaceEdgeChange<S> {
    pub id: String,
    pub item: Edge<S>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeChange<S> {
    Dimensions(NodeDimensionChange),
    Position(NodePositionChange),
    Select(SelectionChange),
    Remove(RemoveChange),
    Add(AddNodeChange<S>),
    Replace(ReplaceNodeChange<S>),
}

impl<S> NodeChange<S> {
    pub fn dimensions(id: impl Into<String>, width: f64, height: f64) -> Self {
        NodeChange::Dimensions(NodeDimensionChange {
            id: id.into(),
            dimensions: Some(crate::core::geometry::Dimensions::new(width, height)),
            resizing: None,
            set_attributes: None,
        })
    }

    pub fn position(id: impl Into<String>, x: f64, y: f64, dragging: bool) -> Self {
        NodeChange::Position(NodePositionChange {
            id: id.into(),
            position: Some(crate::core::geometry::XYPosition::new(x, y)),
            position_absolute: None,
            dragging: Some(dragging),
        })
    }

    pub fn select(id: impl Into<String>, selected: bool) -> Self {
        NodeChange::Select(SelectionChange {
            id: id.into(),
            selected,
        })
    }

    pub fn remove(id: impl Into<String>) -> Self {
        NodeChange::Remove(RemoveChange { id: id.into() })
    }

    pub fn add(item: Node<S>, index: Option<usize>) -> Self {
        NodeChange::Add(AddNodeChange { item, index })
    }

    pub fn replace(id: impl Into<String>, item: Node<S>) -> Self {
        NodeChange::Replace(ReplaceNodeChange {
            id: id.into(),
            item,
        })
    }

    pub fn id(&self) -> &str {
        match self {
            NodeChange::Dimensions(change) => &change.id,
            NodeChange::Position(change) => &change.id,
            NodeChange::Select(change) => &change.id,
            NodeChange::Remove(change) => &change.id,
            NodeChange::Add(change) => &change.item.id,
            NodeChange::Replace(change) => &change.id,
        }
    }

    pub fn kind(&self) -> ChangeKind {
        match self {
            NodeChange::Dimensions(_) => ChangeKind::Dimensions,
            NodeChange::Position(_) => ChangeKind::Position,
            NodeChange::Select(_) => ChangeKind::Select,
            NodeChange::Remove(_) => ChangeKind::Remove,
            NodeChange::Add(_) => ChangeKind::Add,
            NodeChange::Replace(_) => ChangeKind::Replace,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EdgeChange<S> {
    Select(SelectionChange),
    Remove(RemoveChange),
    Add(AddEdgeChange<S>),
    Replace(ReplaceEdgeChange<S>),
}

impl<S> EdgeChange<S> {
    pub fn select(id: impl Into<String>, selected: bool) -> Self {
        EdgeChange::Select(SelectionChange {
            id: id.into(),
            selected,
        })
    }

    pub fn remove(id: impl Into<String>) -> Self {
        EdgeChange::Remove(RemoveChange { id: id.into() })
    }

    pub fn add(item: Edge<S>, index: Option<usize>) -> Self {
        EdgeChange::Add(AddEdgeChange { item, index })
    }

    pub fn replace(id: impl Into<String>, item: Edge<S>) -> Self {
        EdgeChange::Replace(ReplaceEdgeChange {
            id: id.into(),
            item,
        })
    }

    pub fn id(&self) -> &str {
        match self {
            EdgeChange::Select(change) => &change.id,
            EdgeChange::Remove(change) => &change.id,
            EdgeChange::Add(change) => &change.item.id,
            EdgeChange::Replace(change) => &change.id,
        }
    }

    pub fn kind(&self) -> ChangeKind {
        match self {
            EdgeChange::Select(_) => ChangeKind::Select,
            EdgeChange::Remove(_) => ChangeKind::Remove,
            EdgeChange::Add(_) => ChangeKind::Add,
            EdgeChange::Replace(_) => ChangeKind::Replace,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    Dimensions,
    Position,
    Select,
    Remove,
    Add,
    Replace,
}

pub fn apply_node_changes<S: Clone>(changes: &[NodeChange<S>], nodes: &[Node<S>]) -> Vec<Node<S>> {
    let mut updated: Vec<Node<S>> = Vec::with_capacity(nodes.len());
    let mut changes_map: std::collections::BTreeMap<String, Vec<&NodeChange<S>>> =
        std::collections::BTreeMap::new();
    let mut add_changes: Vec<&AddNodeChange<S>> = Vec::new();

    for change in changes {
        match change {
            NodeChange::Add(add) => add_changes.push(add),
            NodeChange::Remove(_) | NodeChange::Replace(_) => {
                changes_map.insert(change.id().to_string(), vec![change]);
            }
            _ => {
                changes_map
                    .entry(change.id().to_string())
                    .or_default()
                    .push(change);
            }
        }
    }

    for node in nodes {
        let Some(node_changes) = changes_map.get(&node.id) else {
            updated.push(node.clone());
            continue;
        };

        match node_changes.first() {
            Some(NodeChange::Remove(_)) => continue,
            Some(NodeChange::Replace(replace)) => {
                updated.push(replace.item.clone());
                continue;
            }
            _ => {}
        }

        let mut next = node.clone();
        for change in node_changes {
            apply_single_node_change(change, &mut next);
        }
        updated.push(next);
    }

    for add in add_changes {
        let item = add.item.clone();
        match add.index {
            Some(index) => {
                let index = index.min(updated.len());
                updated.insert(index, item);
            }
            None => updated.push(item),
        }
    }

    updated
}

fn apply_single_node_change<S: Clone>(change: &NodeChange<S>, node: &mut Node<S>) {
    match change {
        NodeChange::Select(change) => node.selected = change.selected,
        NodeChange::Position(change) => {
            if let Some(position) = change.position {
                node.position = position;
            }
            if let Some(dragging) = change.dragging {
                node.dragging = dragging;
            }
        }
        NodeChange::Dimensions(change) => {
            if let Some(dimensions) = change.dimensions {
                node.measured = crate::core::node::MeasuredDimensions::new(
                    dimensions.width,
                    dimensions.height,
                );
                if let Some(set_attributes) = change.set_attributes {
                    if set_attributes.sets_width() {
                        node.width = Some(dimensions.width);
                    }
                    if set_attributes.sets_height() {
                        node.height = Some(dimensions.height);
                    }
                }
            }
        }
        NodeChange::Remove(_) | NodeChange::Add(_) | NodeChange::Replace(_) => {}
    }
}

pub fn apply_edge_changes<S: Clone>(changes: &[EdgeChange<S>], edges: &[Edge<S>]) -> Vec<Edge<S>> {
    let mut updated: Vec<Edge<S>> = Vec::with_capacity(edges.len());
    let mut changes_map: std::collections::BTreeMap<String, Vec<&EdgeChange<S>>> =
        std::collections::BTreeMap::new();
    let mut add_changes: Vec<&AddEdgeChange<S>> = Vec::new();

    for change in changes {
        match change {
            EdgeChange::Add(add) => add_changes.push(add),
            EdgeChange::Remove(_) | EdgeChange::Replace(_) => {
                changes_map.insert(change.id().to_string(), vec![change]);
            }
            _ => {
                changes_map
                    .entry(change.id().to_string())
                    .or_default()
                    .push(change);
            }
        }
    }

    for edge in edges {
        let Some(edge_changes) = changes_map.get(&edge.id) else {
            updated.push(edge.clone());
            continue;
        };

        match edge_changes.first() {
            Some(EdgeChange::Remove(_)) => continue,
            Some(EdgeChange::Replace(replace)) => {
                updated.push(replace.item.clone());
                continue;
            }
            _ => {}
        }

        let mut next = edge.clone();
        for change in edge_changes {
            if let EdgeChange::Select(select) = change {
                next.selected = select.selected;
            }
        }
        updated.push(next);
    }

    for add in add_changes {
        let item = add.item.clone();
        match add.index {
            Some(index) => {
                let index = index.min(updated.len());
                updated.insert(index, item);
            }
            None => updated.push(item),
        }
    }

    updated
}

pub fn create_selection_change(id: impl Into<String>, selected: bool) -> SelectionChange {
    SelectionChange {
        id: id.into(),
        selected,
    }
}

pub fn get_selection_changes<S: Clone>(
    items: &[&Node<S>],
    selected_ids: &std::collections::BTreeSet<String>,
) -> Vec<NodeChange<S>> {
    let mut changes = Vec::new();
    for node in items {
        let will_be_selected = selected_ids.contains(&node.id);
        if !(node.selected == false && !will_be_selected) && node.selected != will_be_selected {
            changes.push(NodeChange::select(node.id.clone(), will_be_selected));
        }
    }
    changes
}

pub fn get_edge_selection_changes(
    items: &[&Edge<String>],
    selected_ids: &std::collections::BTreeSet<String>,
) -> Vec<EdgeChange<String>> {
    let mut changes = Vec::new();
    for edge in items {
        let will_be_selected = selected_ids.contains(&edge.id);
        if edge.selected != will_be_selected {
            changes.push(EdgeChange::select(edge.id.clone(), will_be_selected));
        }
    }
    changes
}
