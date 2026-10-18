use crate::core::{
    edge::{Edge, EdgeGraphics},
    geometry::{
        EdgePosition, NodeOrigin, Padding, Position, Rect, Transform, Viewport, XYPosition,
        ZIndexMode,
    },
    node::{Handle, HandleType, InternalNode, Node, NodeLookup},
    utils::{
        box_to_rect, get_bounds_of_boxes, get_rects_overlapping_area, internal_node_to_box,
        node_to_box,
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectionMode {
    #[default]
    Strict,
    Loose,
}

impl ConnectionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ConnectionMode::Strict => "strict",
            ConnectionMode::Loose => "loose",
        }
    }

    pub fn from_str(value: &str) -> Option<ConnectionMode> {
        match value {
            "strict" => Some(ConnectionMode::Strict),
            "loose" => Some(ConnectionMode::Loose),
            _ => None,
        }
    }
}

pub fn get_outgoers<'a, S>(
    node_id: &str,
    nodes: &'a [Node<S>],
    edges: &[Edge],
) -> Vec<&'a Node<S>> {
    if node_id.is_empty() {
        return Vec::new();
    }
    let outgoer_ids: std::collections::BTreeSet<&str> = edges
        .iter()
        .filter(|edge| edge.source == node_id)
        .map(|edge| edge.target.as_str())
        .collect();
    nodes
        .iter()
        .filter(|node| outgoer_ids.contains(node.id.as_str()))
        .collect()
}

pub fn get_incomers<'a, S>(
    node_id: &str,
    nodes: &'a [Node<S>],
    edges: &[Edge],
) -> Vec<&'a Node<S>> {
    if node_id.is_empty() {
        return Vec::new();
    }
    let incomer_ids: std::collections::BTreeSet<&str> = edges
        .iter()
        .filter(|edge| edge.target == node_id)
        .map(|edge| edge.source.as_str())
        .collect();
    nodes
        .iter()
        .filter(|node| incomer_ids.contains(node.id.as_str()))
        .collect()
}

pub fn get_connected_edges<'a, S>(node_ids: &[&str], edges: &'a [Edge<S>]) -> Vec<&'a Edge<S>> {
    let ids: std::collections::BTreeSet<&str> = node_ids.iter().copied().collect();
    edges
        .iter()
        .filter(|edge| ids.contains(edge.source.as_str()) || ids.contains(edge.target.as_str()))
        .collect()
}

pub fn get_nodes_bounds<'a, S: Clone + 'a>(
    nodes: impl IntoIterator<Item = &'a Node<S>>,
    node_origin: NodeOrigin,
) -> Rect {
    let mut bounds = crate::core::geometry::Box2::EMPTY;
    let mut has_node = false;

    for node in nodes {
        has_node = true;
        bounds = get_bounds_of_boxes(bounds, node_to_box(node, node_origin));
    }

    if has_node {
        box_to_rect(bounds)
    } else {
        Rect::ZERO
    }
}

pub fn get_internal_nodes_bounds<'a, S: Clone + 'a>(
    nodes: impl IntoIterator<Item = &'a InternalNode<S>>,
) -> Rect {
    let mut bounds = crate::core::geometry::Box2::EMPTY;
    let mut has_node = false;

    for node in nodes {
        has_node = true;
        bounds = get_bounds_of_boxes(bounds, internal_node_to_box(node));
    }

    if has_node {
        box_to_rect(bounds)
    } else {
        Rect::ZERO
    }
}

pub fn get_nodes_inside<S: Clone>(
    node_lookup: &NodeLookup<S>,
    rect: Rect,
    transform: Transform,
    partially: bool,
    exclude_non_selectable_nodes: bool,
) -> Vec<InternalNode<S>> {
    let pane = Rect::new(
        (rect.x - transform.x()) / transform.zoom(),
        (rect.y - transform.y()) / transform.zoom(),
        rect.width / transform.zoom(),
        rect.height / transform.zoom(),
    );

    let mut visible = Vec::new();

    for node in node_lookup.values() {
        if (exclude_non_selectable_nodes && !node.selectable.unwrap_or(true)) || node.hidden {
            continue;
        }

        let dimensions = node.size();
        let node_rect = Rect::new(
            node.internals.position_absolute.x,
            node.internals.position_absolute.y,
            dimensions.width,
            dimensions.height,
        );
        let overlapping_area = get_rects_overlapping_area(pane, node_rect);
        let area = node_rect.width * node_rect.height;

        let partially_visible = partially && overlapping_area > 0.0;
        let force_initial_render = node.internals.handle_bounds.is_none();
        let is_visible = force_initial_render || partially_visible || overlapping_area >= area;

        if is_visible || node.dragging {
            visible.push(node.clone());
        }
    }

    visible
}

pub fn is_edge_visible<S: Clone>(
    source_node: &InternalNode<S>,
    target_node: &InternalNode<S>,
    width: f64,
    height: f64,
    transform: Transform,
) -> bool {
    let mut edge_box = get_bounds_of_boxes(
        internal_node_to_box(source_node),
        internal_node_to_box(target_node),
    );

    if edge_box.x == edge_box.x2 {
        edge_box.x2 += 1.0;
    }
    if edge_box.y == edge_box.y2 {
        edge_box.y2 += 1.0;
    }

    let view_rect = Rect::new(
        -transform.x() / transform.zoom(),
        -transform.y() / transform.zoom(),
        width / transform.zoom(),
        height / transform.zoom(),
    );

    get_rects_overlapping_area(view_rect, box_to_rect(edge_box)) > 0.0
}

pub fn get_handle_position<S: Clone>(
    node: &InternalNode<S>,
    handle: Option<&Handle>,
    fallback_position: Position,
    center: bool,
) -> XYPosition {
    let x = handle.map(|h| h.x).unwrap_or(0.0) + node.internals.position_absolute.x;
    let y = handle.map(|h| h.y).unwrap_or(0.0) + node.internals.position_absolute.y;
    let (width, height) = match handle {
        Some(handle) => (handle.width, handle.height),
        None => {
            let dimensions = node.size();
            (dimensions.width, dimensions.height)
        }
    };

    if center {
        return XYPosition::new(x + width / 2.0, y + height / 2.0);
    }

    match handle.map(|h| h.position).unwrap_or(fallback_position) {
        Position::Top => XYPosition::new(x + width / 2.0, y),
        Position::Right => XYPosition::new(x + width, y + height / 2.0),
        Position::Bottom => XYPosition::new(x + width / 2.0, y + height),
        Position::Left => XYPosition::new(x, y + height / 2.0),
    }
}

fn find_handle(bounds: &[Handle], handle_id: Option<&str>) -> Option<Handle> {
    match handle_id {
        None => bounds.first().cloned(),
        Some(id) => bounds
            .iter()
            .find(|handle| handle.id.as_deref() == Some(id))
            .cloned(),
    }
}

pub fn get_handle_by_id<S: Clone>(
    node: &InternalNode<S>,
    handle_type: HandleType,
    handle_id: Option<&str>,
    connection_mode: ConnectionMode,
) -> Option<Handle> {
    let bounds = node.internals.handle_bounds.as_ref()?;
    let candidates: Vec<Handle> = match connection_mode {
        ConnectionMode::Strict => bounds.handles_of(handle_type).to_vec(),
        ConnectionMode::Loose => bounds.all().cloned().collect(),
    };
    find_handle(&candidates, handle_id)
}

pub fn is_node_initialized_for_edge<S: Clone>(node: &InternalNode<S>) -> bool {
    (node.internals.handle_bounds.is_some() || !node.internals.user_node.handles.is_empty())
        && (node.measured.width.is_some() || node.width.is_some() || node.initial_width.is_some())
}

pub struct EdgePositionParams<'a, S> {
    pub id: &'a str,
    pub source_node: &'a InternalNode<S>,
    pub source_handle: Option<&'a str>,
    pub target_node: &'a InternalNode<S>,
    pub target_handle: Option<&'a str>,
    pub connection_mode: ConnectionMode,
}

pub fn get_edge_position<S: Clone>(params: EdgePositionParams<'_, S>) -> Option<EdgePosition> {
    if !is_node_initialized_for_edge(params.source_node)
        || !is_node_initialized_for_edge(params.target_node)
    {
        return None;
    }

    let source_handle = get_handle_by_id(
        params.source_node,
        HandleType::Source,
        params.source_handle,
        params.connection_mode,
    )?;
    let target_handle = get_handle_by_id(
        params.target_node,
        HandleType::Target,
        params.target_handle,
        params.connection_mode,
    )?;

    let source = get_handle_position(
        params.source_node,
        Some(&source_handle),
        source_handle.position,
        false,
    );
    let target = get_handle_position(
        params.target_node,
        Some(&target_handle),
        target_handle.position,
        false,
    );

    Some(EdgePosition::new(
        source.x,
        source.y,
        target.x,
        target.y,
        source_handle.position,
        target_handle.position,
    ))
}

pub fn get_edge_graphics(
    edge_type: &str,
    position: &EdgePosition,
    path_options: Option<&crate::core::edge::PathOptions>,
) -> EdgeGraphics {
    crate::core::edge_path::get_edge_path(
        edge_type,
        position.source_x,
        position.source_y,
        position.source_position,
        position.target_x,
        position.target_y,
        position.target_position,
        path_options,
    )
}

pub fn get_elevated_edge_z_index<S: Clone>(
    source_node: &InternalNode<S>,
    target_node: &InternalNode<S>,
    selected: bool,
    z_index: f64,
    elevate_on_select: bool,
    z_index_mode: ZIndexMode,
) -> f64 {
    if z_index_mode.is_manual() {
        return z_index;
    }

    let edge_z = if elevate_on_select && selected {
        z_index + 1000.0
    } else {
        z_index
    };

    let source_z = if source_node.parent_id.is_some() || (elevate_on_select && source_node.selected)
    {
        source_node.internals.z
    } else {
        0.0
    };
    let target_z = if target_node.parent_id.is_some() || (elevate_on_select && target_node.selected)
    {
        target_node.internals.z
    } else {
        0.0
    };

    edge_z + source_z.max(target_z)
}

pub fn get_selected_nodes_bounds<S: Clone>(node_lookup: &NodeLookup<S>) -> Rect {
    get_internal_nodes_bounds(node_lookup.values().filter(|node| node.selected))
}

pub fn get_all_nodes_bounds<S: Clone>(node_lookup: &NodeLookup<S>) -> Rect {
    get_internal_nodes_bounds(node_lookup.values())
}

pub fn get_node_position_with_origin<S: Clone>(
    node: &Node<S>,
    node_origin: NodeOrigin,
) -> XYPosition {
    let dimensions = node.size();
    let origin = node.origin.unwrap_or(node_origin);
    let offset_x = dimensions.width * origin[0];
    let offset_y = dimensions.height * origin[1];

    XYPosition::new(node.position.x - offset_x, node.position.y - offset_y)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FitViewOptions {
    pub nodes: Option<Vec<String>>,
    pub include_hidden_nodes: bool,
    pub min_zoom: Option<f64>,
    pub max_zoom: Option<f64>,
    pub padding: Option<f64>,
}

pub fn get_fit_view_nodes<S: Clone>(
    node_lookup: &NodeLookup<S>,
    options: &FitViewOptions,
) -> Vec<InternalNode<S>> {
    let option_ids: Option<std::collections::BTreeSet<String>> = options
        .nodes
        .as_ref()
        .map(|ids| ids.iter().cloned().collect());

    node_lookup
        .values()
        .filter(|node| {
            let is_visible = if options.include_hidden_nodes {
                let dimensions = node.size();
                dimensions.width > 0.0 && dimensions.height > 0.0
            } else {
                node.measured.width.is_some() && node.measured.height.is_some() && !node.hidden
            };

            is_visible
                && option_ids
                    .as_ref()
                    .map(|ids| ids.contains(&node.id))
                    .unwrap_or(true)
        })
        .cloned()
        .collect()
}

pub fn fit_viewport<S: Clone>(
    node_lookup: &NodeLookup<S>,
    width: f64,
    height: f64,
    min_zoom: f64,
    max_zoom: f64,
    options: &FitViewOptions,
) -> Option<Viewport> {
    let nodes_to_fit = get_fit_view_nodes(node_lookup, options);
    if nodes_to_fit.is_empty() {
        return None;
    }

    let bounds = get_internal_nodes_bounds(nodes_to_fit.iter());
    let padding: Padding = options.padding.unwrap_or(0.1).into();

    Some(crate::core::utils::get_viewport_for_bounds(
        bounds,
        width,
        height,
        options.min_zoom.unwrap_or(min_zoom),
        options.max_zoom.unwrap_or(max_zoom),
        padding,
    ))
}

pub fn get_elements_to_remove<S: Clone + PartialEq>(
    nodes_to_remove: &[Node<S>],
    edges_to_remove: &[Edge<S>],
    nodes: &[Node<S>],
    edges: &[Edge<S>],
) -> (Vec<Node<S>>, Vec<Edge<S>>) {
    let node_ids: std::collections::BTreeSet<&str> = nodes_to_remove
        .iter()
        .map(|node| node.id.as_str())
        .collect();
    let mut matching_nodes: Vec<Node<S>> = Vec::new();

    for node in nodes {
        if node.deletable == Some(false) {
            continue;
        }

        let is_included = node_ids.contains(node.id.as_str());
        let parent_hit = !is_included
            && node
                .parent_id
                .as_ref()
                .map(|parent_id| matching_nodes.iter().any(|n| &n.id == parent_id))
                .unwrap_or(false);

        if is_included || parent_hit {
            matching_nodes.push(node.clone());
        }
    }

    let edge_ids: std::collections::BTreeSet<&str> = edges_to_remove
        .iter()
        .map(|edge| edge.id.as_str())
        .collect();
    let deletable_edges: Vec<Edge<S>> = edges
        .iter()
        .filter(|edge| edge.deletable != Some(false))
        .cloned()
        .collect();

    let matching_node_ids: Vec<&str> = matching_nodes.iter().map(|n| n.id.as_str()).collect();
    let connected_edges = get_connected_edges(&matching_node_ids, &deletable_edges);
    let mut matching_edges: Vec<Edge<S>> = connected_edges.into_iter().cloned().collect();

    for edge in &deletable_edges {
        let is_included = edge_ids.contains(edge.id.as_str());
        if is_included && !matching_edges.iter().any(|e| e.id == edge.id) {
            matching_edges.push(edge.clone());
        }
    }

    (matching_nodes, matching_edges)
}
