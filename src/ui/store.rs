use std::collections::{BTreeMap, BTreeSet};

use crate::core::change::{EdgeChange, NodeChange};
use crate::core::edge::Edge;
use crate::core::geometry::{
    ConnectionMode, CoordinateExtent, Dimensions, NodeOrigin, Rect, SnapGrid, Transform, Viewport,
    XYPosition,
};
use crate::core::graph::{get_edge_position, EdgePositionParams};
use crate::core::node::{InternalNode, Node, NodeHandleBounds, NodeLookup, ParentLookup};
use crate::core::store::{
    adopt_user_nodes, handle_expand_parent, update_absolute_positions, update_node_internals,
    AdoptOptions, NodeInternalsResult, NodeInternalsUpdate, ParentExpandChild,
};
use crate::core::utils::{get_internal_nodes_bounds, get_internal_nodes_bounds_filtered};

use crate::ui::connections::{ConnectionLookup, ConnectionState};

#[derive(Clone, Debug, PartialEq)]
pub struct FlowStore<NodeType = String, EdgeType = String> {
    pub rf_id: String,
    pub nodes: Vec<Node<NodeType>>,
    pub edges: Vec<Edge<EdgeType>>,
    pub node_lookup: NodeLookup<NodeType>,
    pub edge_lookup: BTreeMap<String, Edge<EdgeType>>,
    pub parent_lookup: ParentLookup<NodeType>,
    pub connection_lookup: ConnectionLookup,
    pub transform: Transform,
    pub width: f64,
    pub height: f64,
    pub min_zoom: f64,
    pub max_zoom: f64,
    pub translate_extent: CoordinateExtent,
    pub node_extent: CoordinateExtent,
    pub node_origin: NodeOrigin,
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
    pub connection: ConnectionState,
    pub aria_live_message: String,
}

impl<NodeType: Clone, EdgeType: Clone> Default for FlowStore<NodeType, EdgeType> {
    fn default() -> Self {
        FlowStore {
            rf_id: "1".to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
            node_lookup: BTreeMap::new(),
            edge_lookup: BTreeMap::new(),
            parent_lookup: BTreeMap::new(),
            connection_lookup: ConnectionLookup::new(),
            transform: Transform::IDENTITY,
            width: 0.0,
            height: 0.0,
            min_zoom: 0.5,
            max_zoom: 2.0,
            translate_extent: crate::core::constants::INFINITE_EXTENT,
            node_extent: crate::core::constants::INFINITE_EXTENT,
            node_origin: [0.0, 0.0],
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
            connection_radius: 20.0,
            reconnect_radius: 10.0,
            auto_pan_on_connect: true,
            auto_pan_on_node_drag: true,
            auto_pan_on_selection: true,
            auto_pan_on_node_focus: true,
            auto_pan_speed: 15.0,
            only_render_visible_elements: false,
            multi_selection_active: false,
            nodes_selection_active: false,
            user_selection_active: false,
            user_selection_rect: None,
            pane_dragging: false,
            connection: ConnectionState::default(),
            aria_live_message: String::new(),
        }
    }
}

impl<NodeType: Clone, EdgeType: Clone> FlowStore<NodeType, EdgeType> {
    pub fn viewport(&self) -> Viewport {
        Viewport::new(self.transform.0, self.transform.1, self.transform.2)
    }

    pub fn dimensions(&self) -> Dimensions {
        Dimensions::new(self.width, self.height)
    }

    pub fn set_nodes(&mut self, nodes: Vec<Node<NodeType>>) -> bool {
        self.nodes = nodes;
        self.adopt_nodes(true)
    }

    pub fn set_edges(&mut self, edges: Vec<Edge<EdgeType>>) {
        self.edges = edges;
        self.sync_edge_lookup();
    }

    pub fn adopt_nodes(&mut self, check_equality: bool) -> bool {
        let options = AdoptOptions {
            check_equality,
            node_origin: self.node_origin,
            node_extent: self.node_extent,
            elevate_nodes_on_select: self.elevate_nodes_on_select,
        };
        let result = adopt_user_nodes(&self.nodes, options);
        self.node_lookup = result.node_lookup;
        self.parent_lookup = result.parent_lookup;
        result.nodes_initialized
    }

    pub fn sync_edge_lookup(&mut self) {
        self.edge_lookup = self
            .edges
            .iter()
            .map(|edge| (edge.id.clone(), edge.clone()))
            .collect();
        self.connection_lookup =
            crate::ui::connections::build_connection_lookup(&self.edges);
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.transform = Transform(viewport.x, viewport.y, viewport.zoom);
    }

    pub fn update_dimensions(&mut self, dimensions: Dimensions) {
        self.width = dimensions.width;
        self.height = dimensions.height;
    }

    pub fn update_node_internals(
        &mut self,
        updates: Vec<NodeInternalsUpdate>,
    ) -> (Vec<NodeChange<NodeType>>, bool) {
        let result: NodeInternalsResult<NodeType> = update_node_internals(
            &updates,
            &mut self.node_lookup,
            &self.parent_lookup,
            self.node_origin,
            self.node_extent,
            self.z_index_mode,
        );

        (result.changes, result.updated_internals)
    }

    pub fn refresh_positions(&mut self) {
        update_absolute_positions(
            &mut self.node_lookup,
            &self.parent_lookup,
            self.node_origin,
        );
    }

    pub fn expand_parents(&mut self, children: Vec<ParentExpandChild>) -> Vec<NodeChange<NodeType>> {
        self.refresh_positions();
        handle_expand_parent(&children, &self.parent_lookup, self.node_origin)
    }

    pub fn internal_node(&self, id: &str) -> Option<&InternalNode<NodeType>> {
        self.node_lookup.get(id)
    }

    pub fn user_node(&self, id: &str) -> Option<&Node<NodeType>> {
        self.node_lookup.get(id).map(|node| &node.internals.user_node)
    }

    pub fn edge_position(&self, edge_id: &str) -> Option<crate::core::geometry::EdgePosition> {
        let edge = self.edge_lookup.get(edge_id)?;
        let source = self.node_lookup.get(&edge.source)?;
        let target = self.node_lookup.get(&edge.target)?;

        get_edge_position(EdgePositionParams {
            id: edge_id,
            source_node: source,
            source_handle: edge.source_handle.as_deref(),
            target_node: target,
            target_handle: edge.target_handle.as_deref(),
            connection_mode: self.connection_mode,
        })
    }

    pub fn selected_node_ids(&self) -> BTreeSet<String> {
        self.node_lookup
            .values()
            .filter(|node| node.selected)
            .map(|node| node.id.clone())
            .collect()
    }

    pub fn selected_edge_ids(&self) -> BTreeSet<String> {
        self.edge_lookup
            .values()
            .filter(|edge| edge.selected)
            .map(|edge| edge.id.clone())
            .collect()
    }

    pub fn nodes_inside(&self, rect: Rect, partially: bool) -> Vec<InternalNode<NodeType>> {
        crate::core::graph::get_nodes_inside(
            &self.node_lookup,
            rect,
            self.transform,
            partially,
            true,
        )
    }

    pub fn internal_bounds(&self) -> Rect {
        get_internal_nodes_bounds(&self.node_lookup)
    }

    pub fn internal_bounds_filtered(
        &self,
        filter: impl Fn(&InternalNode<NodeType>) -> bool,
    ) -> Rect {
        get_internal_nodes_bounds_filtered(&self.node_lookup, filter)
    }

    pub fn handle_bounds(&self, node_id: &str) -> Option<&NodeHandleBounds> {
        self.node_lookup
            .get(node_id)
            .and_then(|node| node.internals.handle_bounds.as_ref())
    }

    pub fn connection_key(&self, node_id: &str, handle_id: Option<&str>) -> String {
        match handle_id {
            Some(id) => format!("{node_id}-{id}"),
            None => node_id.to_string(),
        }
    }

    pub fn apply_node_changes(&mut self, changes: &[NodeChange<NodeType>]) -> Vec<Node<NodeType>> {
        crate::core::change::apply_node_changes(changes, &self.nodes)
    }

    pub fn apply_edge_changes(&mut self, changes: &[EdgeChange<EdgeType>]) -> Vec<Edge<EdgeType>> {
        crate::core::change::apply_edge_changes(changes, &self.edges)
    }

    pub fn select_nodes(&mut self, ids: &[String]) {
        let mut changes = Vec::new();
        for id in ids {
            changes.push(NodeChange::Selection {
                id: id.clone(),
                selected: true,
            });
        }
        let next = self.apply_node_changes(&changes);
        self.set_nodes(next);
    }

    pub fn select_edges(&mut self, ids: &[String]) {
        let mut changes = Vec::new();
        for id in ids {
            changes.push(EdgeChange::Selection {
                id: id.clone(),
                selected: true,
            });
        }
        let next = self.apply_edge_changes(&changes);
        self.set_edges(next);
    }

    pub fn clear_selection(&mut self) {
        let node_changes: Vec<NodeChange<NodeType>> = self
            .node_lookup
            .values()
            .filter(|node| node.selected)
            .map(|node| NodeChange::Selection {
                id: node.id.clone(),
                selected: false,
            })
            .collect();
        let edge_changes: Vec<EdgeChange<EdgeType>> = self
            .edge_lookup
            .values()
            .filter(|edge| edge.selected)
            .map(|edge| EdgeChange::Selection {
                id: edge.id.clone(),
                selected: false,
            })
            .collect();

        if !node_changes.is_empty() {
            let next = self.apply_node_changes(&node_changes);
            self.set_nodes(next);
        }

        if !edge_changes.is_empty() {
            let next = self.apply_edge_changes(&edge_changes);
            self.set_edges(next);
        }
    }

    pub fn update_node_position(&mut self, id: &str, position: XYPosition, dragging: bool) {
        if let Some(node) = self.node_lookup.get_mut(id) {
            node.position = position;
            node.internals.user_node.position = position;
            if dragging {
                node.dragging = true;
                node.internals.user_node.dragging = true;
            }
        }
    }

    pub fn update_node_dimensions(&mut self, id: &str, dimensions: Dimensions) {
        if let Some(node) = self.node_lookup.get_mut(id) {
            node.measured.width = Some(dimensions.width);
            node.measured.height = Some(dimensions.height);
            node.internals.user_node.measured.width = Some(dimensions.width);
            node.internals.user_node.measured.height = Some(dimensions.height);
        }
    }

    pub fn reset(&mut self) {
        let rf_id = self.rf_id.clone();
        let min_zoom = self.min_zoom;
        let max_zoom = self.max_zoom;
        *self = FlowStore::default();
        self.rf_id = rf_id;
        self.min_zoom = min_zoom;
        self.max_zoom = max_zoom;
    }
}
