use std::sync::Arc;

use gpui_kit::{AnyElement, App, Window};

use crate::core::{
    edge::Edge,
    geometry::{Alignment, Position, Rect, Viewport},
    graph::get_internal_nodes_bounds,
    node::{InternalNode, Node, NodeLookup},
    utils_extra::{
        ToolbarAlignX, ToolbarAlignY, get_edge_toolbar_transform, get_node_toolbar_transform,
    },
};

/// Builds the interactive content rendered inside a [`NodeToolbarProps`] overlay.
pub type NodeToolbarContent =
    Arc<dyn Fn(&Node<String>, &mut Window, &mut App) -> AnyElement + Send + Sync>;

/// Builds the interactive content rendered inside an [`EdgeToolbarProps`] overlay.
pub type EdgeToolbarContent =
    Arc<dyn Fn(&Edge<String>, &mut Window, &mut App) -> AnyElement + Send + Sync>;

/// Resolves which nodes a node toolbar should be attached to.
///
/// Explicit `node_ids` win; otherwise the current selection is used. When `is_visible` is unset,
/// only selected nodes are shown, matching React Flow's default behaviour.
pub fn node_toolbar_targets<S: Clone>(
    props: &NodeToolbarProps,
    node_lookup: &NodeLookup<S>,
    selected_node_ids: &[String],
) -> Vec<String> {
    let requested = if props.node_ids.is_empty() {
        selected_node_ids.to_vec()
    } else {
        props.node_ids.clone()
    };

    requested
        .into_iter()
        .filter(|id| {
            node_lookup
                .get(id)
                .map(|node| props.is_visible.unwrap_or(node.selected))
                .unwrap_or(false)
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeToolbarProps {
    pub node_ids: Vec<String>,
    pub is_visible: Option<bool>,
    pub position: Position,
    pub offset: f64,
    pub align: Alignment,
}

impl Default for NodeToolbarProps {
    fn default() -> Self {
        NodeToolbarProps {
            node_ids: Vec::new(),
            is_visible: None,
            position: Position::Top,
            offset: 10.0,
            align: Alignment::Center,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeToolbarState {
    pub active: bool,
    pub transform: Option<String>,
    pub z_index: f64,
    pub data_id: String,
    pub bounds: Option<Rect>,
}

impl Default for NodeToolbarState {
    fn default() -> Self {
        NodeToolbarState {
            active: false,
            transform: None,
            z_index: 0.0,
            data_id: String::new(),
            bounds: None,
        }
    }
}

pub fn node_toolbar<S: Clone>(
    props: &NodeToolbarProps,
    node_lookup: &NodeLookup<S>,
    viewport: Viewport,
    selected_nodes_count: usize,
) -> NodeToolbarState {
    let nodes: Vec<&InternalNode<S>> = props
        .node_ids
        .iter()
        .filter_map(|id| node_lookup.get(id))
        .collect();

    let is_active = match props.is_visible {
        Some(visible) => visible,
        None => nodes.len() == 1 && nodes[0].selected && selected_nodes_count == 1,
    };

    if !is_active || nodes.is_empty() {
        return NodeToolbarState::default();
    }

    let bounds = get_internal_nodes_bounds(nodes.iter().copied());
    let z_index = nodes
        .iter()
        .map(|node| node.internals.z + 1.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let transform =
        get_node_toolbar_transform(bounds, viewport, props.position, props.offset, props.align);
    let data_id = nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    NodeToolbarState {
        active: true,
        transform: Some(transform),
        z_index,
        data_id,
        bounds: Some(bounds),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeToolbarProps {
    pub edge_id: String,
    pub x: f64,
    pub y: f64,
    pub is_visible: Option<bool>,
    pub align_x: ToolbarAlignX,
    pub align_y: ToolbarAlignY,
}

impl Default for EdgeToolbarProps {
    fn default() -> Self {
        EdgeToolbarProps {
            edge_id: String::new(),
            x: 0.0,
            y: 0.0,
            is_visible: None,
            align_x: ToolbarAlignX::Center,
            align_y: ToolbarAlignY::Center,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeToolbarState {
    pub active: bool,
    pub transform: Option<String>,
    pub z_index: f64,
}

impl Default for EdgeToolbarState {
    fn default() -> Self {
        EdgeToolbarState {
            active: false,
            transform: None,
            z_index: 0.0,
        }
    }
}

pub fn edge_toolbar<S: Clone>(
    props: &EdgeToolbarProps,
    edge: Option<&Edge<S>>,
    zoom: f64,
) -> EdgeToolbarState {
    let is_active = match props.is_visible {
        Some(visible) => visible,
        None => edge.map(|edge| edge.selected).unwrap_or(false),
    };

    if !is_active {
        return EdgeToolbarState::default();
    }

    let z_index = edge.and_then(|edge| edge.z_index).unwrap_or(0.0) + 1.0;
    let transform =
        get_edge_toolbar_transform(props.x, props.y, zoom, props.align_x, props.align_y);

    EdgeToolbarState {
        active: true,
        transform: Some(transform),
        z_index,
    }
}
