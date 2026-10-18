use std::collections::BTreeMap;

use crate::core::change::{EdgeChange, NodeChange};
use crate::core::edge::{Edge, EdgeLookup};
use crate::core::geometry::{
    CoordinateExtent, Dimensions, EdgePosition, NodeOrigin, Rect, SnapGrid, Transform, Viewport,
    XYPosition, ZIndexMode,
};
use crate::core::graph::ConnectionMode;
use crate::core::graph::{get_edge_position, get_nodes_inside, EdgePositionParams};
use crate::core::node::{InternalNode, Node, NodeLookup, ParentLookup};
use crate::core::store::{
    adopt_user_nodes, AdoptOptions, AdoptResult, NodeInternalsResult, NodeInternalsUpdate,
};
use crate::core::utils::{
    get_bounds_of_box_list, get_viewport_for_bounds, internal_node_to_box, parse_paddings,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewportSize {
    pub width: f64,
    pub height: f64,
}

impl ViewportSize {
    pub fn new(width: f64, height: f64) -> Self {
        ViewportSize { width, height }
    }

    pub fn dimensions(self) -> Dimensions {
        Dimensions::new(self.width, self.height)
    }

    pub fn rect(self) -> Rect {
        Rect::new(0.0, 0.0, self.width, self.height)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionViewState {
    pub in_progress: bool,
    pub is_valid: Option<bool>,
    pub from: XYPosition,
    pub from_node: Option<String>,
    pub from_handle: Option<String>,
    pub from_position: Option<crate::core::geometry::Position>,
    pub to: XYPosition,
    pub to_node: Option<String>,
    pub to_handle: Option<String>,
    pub to_position: Option<crate::core::geometry::Position>,
    pub pointer: XYPosition,
}

impl Default for ConnectionViewState {
    fn default() -> Self {
        ConnectionViewState {
            in_progress: false,
            is_valid: None,
            from: XYPosition::ZERO,
            from_node: None,
            from_handle: None,
            from_position: None,
            to: XYPosition::ZERO,
            to_node: None,
            to_handle: None,
            to_position: None,
            pointer: XYPosition::ZERO,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowStore<NodeType = String, EdgeType = String> {
    pub rf_id: String,
    pub nodes: Vec<Node<NodeType>>,
    pub edges: Vec<Edge<EdgeType>>,
    pub node_lookup: NodeLookup<NodeType>,
    pub edge_lookup: EdgeLookup<EdgeType>,
    pub parent_lookup: ParentLookup<NodeType>,
    pub transform: Transform,
    pub width: f64,
    pub height: f64,
    pub min_zoom: f64,
    pub max_zoom: f64,
    pub translate_extent: CoordinateExtent,
    pub node_extent: CoordinateExtent,
    pub node_origin: NodeOrigin,
    pub z_index_mode: ZIndexMode,
    pub snap_grid: SnapGrid,
    pub snap_to_grid: bool,
    pub connection_mode: ConnectionMode,
    pub nodes_draggable: bool,
    pub nodes_connectable: bool,
    pub nodes_focusable: bool,
    pub edges_focusable: bool,
    pub edges_reconnectable: bool,
    pub elements_selectable: bool,
    pub elevate_nodes_on_select: bool,
    pub elevate_edges_on_select: bool,
    pub select_nodes_on_drag: bool,
    pub node_drag_threshold: f64,
    pub connection_drag_threshold: f64,
    pub connection_radius: f64,
    pub reconnect_radius: f64,
    pub auto_pan_on_connect: bool,
    pub auto_pan_on_node_drag: bool,
    pub auto_pan_on_selection: bool,
    pub auto_pan_on_node_focus: bool,
    pub auto_pan_speed: f64,
    pub only_render_visible_elements: bool,
    pub multi_selection_active: bool,
    pub nodes_selection_active: bool,
    pub user_selection_active: bool,
    pub user_selection_rect: Option<Rect>,
    pub pane_dragging: bool,
    pub connection: ConnectionViewState,
}

impl<NodeType: Clone, EdgeType: Clone> Default for FlowStore<NodeType, EdgeType> {
    fn default() -> Self {
        FlowStore {
            rf_id: "1".to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
            node_lookup: NodeLookup::default(),
            edge_lookup: EdgeLookup::default(),
            parent_lookup: ParentLookup::default(),
            transform: Transform::IDENTITY,
            width: 0.0,
            height: 0.0,
            min_zoom: 0.5,
            max_zoom: 2.0,
            translate_extent: crate::core::constants::INFINITE_EXTENT,
            node_extent: crate::core::constants::INFINITE_EXTENT,
            node_origin: crate::core::constants::DEFAULT_NODE_ORIGIN,
            z_index_mode: ZIndexMode::Basic,
            snap_grid: [15.0, 15.0],
            snap_to_grid: false,
            connection_mode: ConnectionMode::Strict,
            nodes_draggable: true,
            nodes_connectable: true,
            nodes_focusable: true,
            edges_focusable: true,
            edges_reconnectable: true,
            elements_selectable: true,
            elevate_nodes_on_select: true,
            elevate_edges_on_select: false,
            select_nodes_on_drag: true,
            node_drag_threshold: 1.0,
            connection_drag_threshold: 1.0,
            connection_radius: crate::core::constants::DEFAULT_CONNECTION_RADIUS,
            reconnect_radius: crate::core::constants::DEFAULT_RECONNECT_RADIUS,
            auto_pan_on_connect: true,
            auto_pan_on_node_drag: true,
            auto_pan_on_selection: true,
            auto_pan_on_node_focus: true,
            auto_pan_speed: crate::core::constants::DEFAULT_AUTO_PAN_SPEED,
            only_render_visible_elements: false,
            multi_selection_active: false,
            nodes_selection_active: false,
            user_selection_active: false,
            user_selection_rect: None,
            pane_dragging: false,
            connection: ConnectionViewState::default(),
        }
    }
}

impl<NodeType: Clone + PartialEq, EdgeType: Clone> FlowStore<NodeType, EdgeType> {
    pub fn viewport(&self) -> Viewport {
        Viewport::new(self.transform.0, self.transform.1, self.transform.2)
    }

    pub fn size(&self) -> ViewportSize {
        ViewportSize::new(self.width, self.height)
    }

    pub fn adopt_options(&self, check_equality: bool) -> AdoptOptions {
        AdoptOptions {
            node_origin: self.node_origin,
            node_extent: self.node_extent,
            elevate_nodes_on_select: self.elevate_nodes_on_select,
            z_index_mode: self.z_index_mode,
            check_equality,
        }
    }

    pub fn set_nodes(&mut self, nodes: Vec<Node<NodeType>>) -> AdoptResult {
        self.nodes = nodes;
        self.adopt(true)
    }

    pub fn adopt(&mut self, check_equality: bool) -> AdoptResult {
        let options = self.adopt_options(check_equality);
        adopt_user_nodes(
            &self.nodes,
            &mut self.node_lookup,
            &mut self.parent_lookup,
            &options,
        )
    }

    pub fn set_edges(&mut self, edges: Vec<Edge<EdgeType>>) {
        self.edges = edges;
        self.sync_edges();
    }

    pub fn sync_edges(&mut self) {
        self.edge_lookup = self
            .edges
            .iter()
            .map(|edge| (edge.id.clone(), edge.clone()))
            .collect();
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.transform = Transform::new(viewport.x, viewport.y, viewport.zoom);
    }

    pub fn set_size(&mut self, size: ViewportSize) {
        self.width = size.width;
        self.height = size.height;
    }

    pub fn apply_internals(&mut self, updates: &[NodeInternalsUpdate]) -> NodeInternalsResult<NodeType> {
        let options = self.adopt_options(false);
        crate::core::store::update_node_internals(
            updates,
            &mut self.node_lookup,
            &mut self.parent_lookup,
            &options,
        )
    }

    pub fn node(&self, id: &str) -> Option<&InternalNode<NodeType>> {
        self.node_lookup.get(id)
    }

    pub fn edge(&self, id: &str) -> Option<&Edge<EdgeType>> {
        self.edge_lookup.get(id)
    }

    pub fn edge_position(&self, id: &str) -> Option<EdgePosition> {
        let edge = self.edge_lookup.get(id)?;
        let source_node = self.node_lookup.get(&edge.source)?;
        let target_node = self.node_lookup.get(&edge.target)?;

        get_edge_position(EdgePositionParams {
            id: &edge.id,
            source_node,
            source_handle: edge.source_handle.as_deref(),
            target_node,
            target_handle: edge.target_handle.as_deref(),
            connection_mode: self.connection_mode,
        })
    }

    pub fn visible_nodes(&self) -> Vec<InternalNode<NodeType>> {
        if !self.only_render_visible_elements {
            return self.node_lookup.values().cloned().collect();
        }

        get_nodes_inside(
            &self.node_lookup,
            self.size().rect(),
            self.transform,
            true,
            true,
        )
    }

    pub fn bounds(&self) -> Rect {
        let boxes: Vec<crate::core::geometry::Box2> = self
            .node_lookup
            .values()
            .filter(|node| !node.hidden)
            .map(internal_node_to_box)
            .collect();

        if boxes.is_empty() {
            return Rect::ZERO;
        }

        crate::core::utils::box_to_rect(get_bounds_of_box_list(&boxes))
    }

    pub fn selected_nodes(&self) -> Vec<InternalNode<NodeType>> {
        self.node_lookup
            .values()
            .filter(|node| node.selected)
            .cloned()
            .collect()
    }

    pub fn selected_edges(&self) -> Vec<Edge<EdgeType>> {
        self.edge_lookup
            .values()
            .filter(|edge| edge.selected)
            .cloned()
            .collect()
    }

    pub fn selected_bounds(&self) -> Option<Rect> {
        let boxes: Vec<crate::core::geometry::Box2> = self
            .node_lookup
            .values()
            .filter(|node| node.selected)
            .map(internal_node_to_box)
            .collect();

        if boxes.is_empty() {
            None
        } else {
            Some(crate::core::utils::box_to_rect(get_bounds_of_box_list(&boxes)))
        }
    }

    pub fn nodes_inside(&self, rect: Rect, partially: bool) -> Vec<InternalNode<NodeType>> {
        get_nodes_inside(&self.node_lookup, rect, self.transform, partially, true)
    }

    pub fn node_changes_to_nodes(&self, changes: &[NodeChange<NodeType>]) -> Vec<Node<NodeType>> {
        crate::core::change::apply_node_changes(changes, &self.nodes)
    }

    pub fn edge_changes_to_edges(&self, changes: &[EdgeChange<EdgeType>]) -> Vec<Edge<EdgeType>> {
        crate::core::change::apply_edge_changes(changes, &self.edges)
    }

    pub fn commit_node_changes(&mut self, changes: &[NodeChange<NodeType>]) -> AdoptResult {
        let next = self.node_changes_to_nodes(changes);
        self.set_nodes(next)
    }

    pub fn commit_edge_changes(&mut self, changes: &[EdgeChange<EdgeType>]) {
        let next = self.edge_changes_to_edges(changes);
        self.set_edges(next);
    }

    pub fn select_nodes(&mut self, ids: &[String]) {
        let changes: Vec<NodeChange<NodeType>> = ids
            .iter()
            .map(|id| NodeChange::select(id.clone(), true))
            .collect();

        if !changes.is_empty() {
            self.commit_node_changes(&changes);
        }
    }

    pub fn select_edges(&mut self, ids: &[String]) {
        let changes: Vec<EdgeChange<EdgeType>> = ids
            .iter()
            .map(|id| EdgeChange::select(id.clone(), true))
            .collect();

        if !changes.is_empty() {
            self.commit_edge_changes(&changes);
        }
    }

    pub fn clear_node_selection(&mut self) {
        let changes: Vec<NodeChange<NodeType>> = self
            .node_lookup
            .values()
            .filter(|node| node.selected)
            .map(|node| NodeChange::select(node.id.clone(), false))
            .collect();

        if !changes.is_empty() {
            self.commit_node_changes(&changes);
        }
    }

    pub fn clear_edge_selection(&mut self) {
        let changes: Vec<EdgeChange<EdgeType>> = self
            .edge_lookup
            .values()
            .filter(|edge| edge.selected)
            .map(|edge| EdgeChange::select(edge.id.clone(), false))
            .collect();

        if !changes.is_empty() {
            self.commit_edge_changes(&changes);
        }
    }

    pub fn clear_selection(&mut self) {
        self.clear_node_selection();
        self.clear_edge_selection();
    }

    pub fn bounding_viewport(&self, padding: crate::core::geometry::Padding) -> Option<Viewport> {
        let boxes: Vec<crate::core::geometry::Box2> = self
            .node_lookup
            .values()
            .filter(|node| !node.hidden)
            .map(internal_node_to_box)
            .collect();

        if boxes.is_empty() {
            return None;
        }

        let rect = crate::core::utils::box_to_rect(get_bounds_of_box_list(&boxes));

        if rect.width <= 0.0 || rect.height <= 0.0 {
            return None;
        }

        Some(get_viewport_for_bounds(
            rect,
            self.width,
            self.height,
            self.min_zoom,
            self.max_zoom,
            padding,
        ))
    }

    pub fn padding_of(padding: crate::core::geometry::Padding) -> crate::core::utils::ResolvedPadding {
        parse_paddings(padding, 0.0, 0.0)
    }

    pub fn node_ids(&self) -> Vec<String> {
        self.node_lookup.keys().cloned().collect()
    }

    pub fn edge_ids(&self) -> Vec<String> {
        self.edge_lookup.keys().cloned().collect()
    }

    pub fn lookup_map(&self) -> BTreeMap<String, InternalNode<NodeType>> {
        self.node_lookup.clone()
    }

    pub fn reset_keep_config(&mut self) {
        let rf_id = self.rf_id.clone();
        let min_zoom = self.min_zoom;
        let max_zoom = self.max_zoom;
        let width = self.width;
        let height = self.height;
        *self = FlowStore::default();
        self.rf_id = rf_id;
        self.min_zoom = min_zoom;
        self.max_zoom = max_zoom;
        self.width = width;
        self.height = height;
    }
}
