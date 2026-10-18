use std::collections::BTreeMap;

use crate::core::{
    connections::get_edge_id,
    edge::Edge,
    geometry::{CoordinateExtent, Rect, Transform, Viewport, XYPosition},
    graph::get_connected_edges,
    node::{Handle, HandleType, InternalNode, Node},
    utils::get_viewport_for_bounds,
};

#[derive(Clone, Debug, PartialEq)]
pub struct FlowHandleState<S = String> {
    pub nodes: Vec<Node<S>>,
    pub edges: Vec<Edge<S>>,
}

impl<S> Default for FlowHandleState<S> {
    fn default() -> Self {
        FlowHandleState {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

impl<S: Clone> FlowHandleState<S> {
    pub fn node(&self, id: &str) -> Option<&Node<S>> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn edge(&self, id: &str) -> Option<&Edge<S>> {
        self.edges.iter().find(|edge| edge.id == id)
    }

    pub fn node_ids(&self) -> Vec<String> {
        self.nodes.iter().map(|node| node.id.clone()).collect()
    }

    pub fn edge_ids(&self) -> Vec<String> {
        self.edges.iter().map(|edge| edge.id.clone()).collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewportApi {
    viewport: Viewport,
    size: (f64, f64),
    min_zoom: f64,
    max_zoom: f64,
}

impl Default for ViewportApi {
    fn default() -> Self {
        ViewportApi {
            viewport: Viewport::IDENTITY,
            size: (0.0, 0.0),
            min_zoom: 0.5,
            max_zoom: 2.0,
        }
    }
}

impl ViewportApi {
    pub fn new(viewport: Viewport, width: f64, height: f64) -> Self {
        ViewportApi {
            viewport,
            size: (width, height),
            min_zoom: 0.5,
            max_zoom: 2.0,
        }
    }

    pub fn viewport(&self) -> Viewport {
        self.viewport
    }

    pub fn transform(&self) -> Transform {
        Transform::new(self.viewport.x, self.viewport.y, self.viewport.zoom)
    }

    pub fn zoom(&self) -> f64 {
        self.viewport.zoom
    }

    pub fn set_zoom(&mut self, zoom: f64) {
        self.viewport.zoom = zoom.clamp(self.min_zoom, self.max_zoom);
    }

    pub fn set_scale_extent(&mut self, min_zoom: f64, max_zoom: f64) {
        self.min_zoom = min_zoom;
        self.max_zoom = max_zoom;
        self.viewport.zoom = self.viewport.zoom.clamp(min_zoom, max_zoom);
    }

    pub fn scale_to(&mut self, zoom: f64) {
        self.set_zoom(zoom);
    }

    pub fn scale_by(&mut self, factor: f64) {
        self.set_zoom(self.viewport.zoom * factor);
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.viewport = Viewport {
            x: viewport.x,
            y: viewport.y,
            zoom: viewport.zoom.clamp(self.min_zoom, self.max_zoom),
        };
    }

    pub fn set_center(&mut self, x: f64, y: f64, zoom: Option<f64>) {
        if let Some(zoom) = zoom {
            self.set_zoom(zoom);
        }
        let (width, height) = self.size;
        self.viewport.x = width / 2.0 - x * self.viewport.zoom;
        self.viewport.y = height / 2.0 - y * self.viewport.zoom;
    }

    pub fn pan_by(&mut self, delta: XYPosition) {
        self.viewport.x += delta.x;
        self.viewport.y += delta.y;
    }

    pub fn screen_to_flow(&self, screen: XYPosition) -> XYPosition {
        self.transform().invert(screen)
    }

    pub fn flow_to_screen(&self, flow: XYPosition) -> XYPosition {
        self.transform().apply(flow)
    }

    pub fn fit_bounds(&mut self, bounds: Rect, padding: f64) {
        let (width, height) = self.size;
        self.viewport = get_viewport_for_bounds(
            bounds,
            width,
            height,
            self.min_zoom,
            self.max_zoom,
            crate::core::geometry::Padding::all(padding),
        );
    }

    pub fn fit_view(&mut self, nodes: &[InternalNode<String>], padding: f64) {
        let rects: Vec<Rect> = nodes
            .iter()
            .filter(|node| !node.hidden)
            .map(|node| {
                let dimensions = node.size();
                Rect::new(
                    node.internals.position_absolute.x,
                    node.internals.position_absolute.y,
                    dimensions.width,
                    dimensions.height,
                )
            })
            .collect();
        if rects.is_empty() {
            return;
        }
        self.fit_bounds(
            crate::core::utils::get_bounds_of_rect_slice(&rects),
            padding,
        );
    }

    pub fn get_zoom(&self) -> f64 {
        self.viewport.zoom
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionInfo {
    pub edge_id: String,
    pub source: String,
    pub source_handle: Option<String>,
    pub target: String,
    pub target_handle: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeDataApi<S = String> {
    state: FlowHandleState<S>,
}

impl<S> Default for NodeDataApi<S> {
    fn default() -> Self {
        NodeDataApi {
            state: FlowHandleState::default(),
        }
    }
}

impl<S: Clone> NodeDataApi<S> {
    pub fn new(nodes: Vec<Node<S>>, edges: Vec<Edge<S>>) -> Self {
        NodeDataApi {
            state: FlowHandleState { nodes, edges },
        }
    }

    pub fn state(&self) -> &FlowHandleState<S> {
        &self.state
    }
}

impl<S: Clone + PartialEq> NodeDataApi<S> {
    pub fn get_nodes(&self) -> &[Node<S>] {
        &self.state.nodes
    }

    pub fn get_node(&self, id: &str) -> Option<&Node<S>> {
        self.state.node(id)
    }

    pub fn get_edges(&self) -> &[Edge<S>] {
        &self.state.edges
    }

    pub fn get_edge(&self, id: &str) -> Option<&Edge<S>> {
        self.state.edge(id)
    }

    pub fn get_edge_id(
        &self,
        source: &str,
        target: &str,
        source_handle: Option<&str>,
        target_handle: Option<&str>,
    ) -> Option<String> {
        self.state
            .edges
            .iter()
            .find(|edge| {
                edge.source == source
                    && edge.target == target
                    && edge.source_handle.as_deref() == source_handle
                    && edge.target_handle.as_deref() == target_handle
            })
            .map(|edge| edge.id.clone())
    }

    pub fn get_node_connections(&self, node_id: &str) -> Vec<ConnectionInfo> {
        self.state
            .edges
            .iter()
            .filter(|edge| edge.source == node_id || edge.target == node_id)
            .map(|edge| ConnectionInfo {
                edge_id: edge.id.clone(),
                source: edge.source.clone(),
                source_handle: edge.source_handle.clone(),
                target: edge.target.clone(),
                target_handle: edge.target_handle.clone(),
            })
            .collect()
    }

    pub fn get_handle_connections(
        &self,
        node_id: &str,
        handle_id: Option<&str>,
    ) -> Vec<ConnectionInfo> {
        self.state
            .edges
            .iter()
            .filter(|edge| {
                (edge.source == node_id && edge.source_handle.as_deref() == handle_id)
                    || (edge.target == node_id && edge.target_handle.as_deref() == handle_id)
            })
            .map(|edge| ConnectionInfo {
                edge_id: edge.id.clone(),
                source: edge.source.clone(),
                source_handle: edge.source_handle.clone(),
                target: edge.target.clone(),
                target_handle: edge.target_handle.clone(),
            })
            .collect()
    }

    pub fn get_connected_node_ids(&self, node_id: &str) -> Vec<String> {
        let mut ids: Vec<String> = Vec::new();
        for edge in &self.state.edges {
            if edge.source == node_id && !ids.contains(&edge.target) {
                ids.push(edge.target.clone());
            }
            if edge.target == node_id && !ids.contains(&edge.source) {
                ids.push(edge.source.clone());
            }
        }
        ids
    }

    pub fn node_rect(
        &self,
        node: &Node<S>,
        node_origin: crate::core::geometry::NodeOrigin,
    ) -> crate::core::geometry::Rect {
        let lookup = self.build_lookup(node_origin);
        if let Some(parent_id) = &node.parent_id {
            let position = crate::core::utils::evaluate_absolute_position(
                node.position,
                node.size(),
                parent_id,
                &lookup,
                node_origin,
            );
            crate::core::utils_extra::get_node_rect_with_position(node, position)
        } else {
            crate::core::utils_extra::get_node_rect_with_position(node, node.position)
        }
    }

    fn build_lookup(
        &self,
        node_origin: crate::core::geometry::NodeOrigin,
    ) -> crate::core::node::NodeLookup<S> {
        let mut lookup = crate::core::node::NodeLookup::new();
        let mut parent_lookup = crate::core::node::ParentLookup::new();
        let options = crate::core::store::AdoptOptions {
            node_origin,
            ..Default::default()
        };
        crate::core::store::adopt_user_nodes(
            &self.state.nodes,
            &mut lookup,
            &mut parent_lookup,
            &options,
        );
        lookup
    }

    pub fn get_intersecting_nodes(
        &self,
        target: &Node<S>,
        partially: bool,
        node_origin: crate::core::geometry::NodeOrigin,
    ) -> Vec<Node<S>> {
        let rect = self.node_rect(target, node_origin);
        let lookup = self.build_lookup(node_origin);
        let internals: Vec<crate::core::node::InternalNode<S>> = lookup.values().cloned().collect();

        let matches = crate::core::utils_extra::get_nodes_intersecting_rect(
            &internals,
            rect,
            partially,
            Some(&target.id),
        );
        let ids: Vec<String> = matches.into_iter().map(|node| node.id.clone()).collect();
        self.state
            .nodes
            .iter()
            .filter(|node| ids.contains(&node.id))
            .cloned()
            .collect()
    }

    pub fn is_node_intersecting(
        &self,
        target: &Node<S>,
        area: crate::core::geometry::Rect,
        partially: bool,
        node_origin: crate::core::geometry::NodeOrigin,
    ) -> bool {
        let lookup = self.build_lookup(node_origin);
        match lookup.get(&target.id) {
            Some(node) => crate::core::utils_extra::is_node_intersecting(node, area, partially),
            None => false,
        }
    }

    pub fn update_node_data<F>(&mut self, id: &str, update: F) -> bool
    where
        F: FnOnce(&mut S),
    {
        match self.state.nodes.iter_mut().find(|node| node.id == id) {
            Some(node) => {
                update(&mut node.data);
                true
            }
            None => false,
        }
    }

    pub fn update_edge_data<F>(&mut self, id: &str, update: F) -> bool
    where
        F: FnOnce(&mut Option<S>),
    {
        match self.state.edges.iter_mut().find(|edge| edge.id == id) {
            Some(edge) => {
                update(&mut edge.data);
                true
            }
            None => false,
        }
    }

    pub fn to_object(&self) -> BTreeMap<String, Vec<String>> {
        let mut object: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for node in &self.state.nodes {
            object
                .entry(node.id.clone())
                .or_insert_with(|| self.get_connected_node_ids(&node.id));
        }
        object
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionApi<S = String> {
    state: FlowHandleState<S>,
    connection_radius: f64,
    connection_mode: crate::core::graph::ConnectionMode,
}

impl<S> Default for ConnectionApi<S> {
    fn default() -> Self {
        ConnectionApi {
            state: FlowHandleState::default(),
            connection_radius: 20.0,
            connection_mode: crate::core::graph::ConnectionMode::Strict,
        }
    }
}

impl<S: Clone> ConnectionApi<S> {
    pub fn new(state: FlowHandleState<S>) -> Self {
        ConnectionApi {
            state,
            connection_radius: 20.0,
            connection_mode: crate::core::graph::ConnectionMode::Strict,
        }
    }

    pub fn with_radius(mut self, radius: f64) -> Self {
        self.connection_radius = radius;
        self
    }

    pub fn with_mode(mut self, mode: crate::core::graph::ConnectionMode) -> Self {
        self.connection_mode = mode;
        self
    }

    pub fn get_connections(&self, node_id: &str) -> Vec<ConnectionInfo> {
        self.state
            .edges
            .iter()
            .filter(|edge| edge.source == node_id || edge.target == node_id)
            .map(|edge| ConnectionInfo {
                edge_id: edge.id.clone(),
                source: edge.source.clone(),
                source_handle: edge.source_handle.clone(),
                target: edge.target.clone(),
                target_handle: edge.target_handle.clone(),
            })
            .collect()
    }

    pub fn has_connection(&self, source: &str, target: &str) -> bool {
        self.state
            .edges
            .iter()
            .any(|edge| edge.source == source && edge.target == target)
    }

    pub fn can_connect(&self, source: &str, target: &str) -> bool {
        source != target && !self.has_connection(source, target)
    }

    pub fn outgoing(&self, node_id: &str) -> Vec<&Edge<S>> {
        self.state
            .edges
            .iter()
            .filter(|edge| edge.source == node_id)
            .collect()
    }

    pub fn incoming(&self, node_id: &str) -> Vec<&Edge<S>> {
        self.state
            .edges
            .iter()
            .filter(|edge| edge.target == node_id)
            .collect()
    }

    pub fn connection_radius(&self) -> f64 {
        self.connection_radius
    }

    pub fn connection_mode(&self) -> crate::core::graph::ConnectionMode {
        self.connection_mode
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowHandle<S = String> {
    pub state: FlowHandleState<S>,
    pub viewport: ViewportApi,
    pub node_data: NodeDataApi<S>,
    pub connections: ConnectionApi<S>,
    pub extent: CoordinateExtent,
}

impl<S: Clone> Default for FlowHandle<S> {
    fn default() -> Self {
        FlowHandle {
            state: FlowHandleState::default(),
            viewport: ViewportApi::default(),
            node_data: NodeDataApi::default(),
            connections: ConnectionApi::default(),
            extent: crate::core::constants::INFINITE_EXTENT,
        }
    }
}

impl<S: Clone> FlowHandle<S> {
    pub fn new(
        nodes: Vec<Node<S>>,
        edges: Vec<Edge<S>>,
        viewport: Viewport,
        width: f64,
        height: f64,
    ) -> Self {
        let state = FlowHandleState {
            nodes: nodes.clone(),
            edges: edges.clone(),
        };
        FlowHandle {
            viewport: ViewportApi::new(viewport, width, height),
            node_data: NodeDataApi::new(nodes, edges),
            connections: ConnectionApi::new(state.clone()),
            state,
            extent: crate::core::constants::INFINITE_EXTENT,
        }
    }

    pub fn sync(&mut self) {
        self.node_data = NodeDataApi::new(self.state.nodes.clone(), self.state.edges.clone());
        self.connections = ConnectionApi::new(self.state.clone());
    }

    pub fn handle_ids(&self) -> Vec<String> {
        self.state
            .nodes
            .iter()
            .flat_map(|node| {
                node.handles
                    .iter()
                    .map(|handle| handle.id.clone().unwrap_or_default())
            })
            .filter(|id| !id.is_empty())
            .collect()
    }

    pub fn connected_edges_of(&self, node_id: &str) -> Vec<String> {
        get_connected_edges(&[node_id], &self.state.edges)
            .into_iter()
            .map(|edge| edge.id.clone())
            .collect()
    }

    pub fn edge_id_for(
        &self,
        source: &str,
        source_handle: Option<&str>,
        target: &str,
        target_handle: Option<&str>,
    ) -> String {
        get_edge_id(source, source_handle, target, target_handle)
    }

    pub fn selected_node_ids(&self) -> Vec<String> {
        self.state
            .nodes
            .iter()
            .filter(|node| node.selected)
            .map(|node| node.id.clone())
            .collect()
    }

    pub fn selected_edge_ids(&self) -> Vec<String> {
        self.state
            .edges
            .iter()
            .filter(|edge| edge.selected)
            .map(|edge| edge.id.clone())
            .collect()
    }
}

pub fn handles_of<S: Clone>(node: &InternalNode<S>, handle_type: HandleType) -> Vec<&Handle> {
    match node.internals.handle_bounds.as_ref() {
        Some(bounds) => match handle_type {
            HandleType::Source => bounds.source.iter().collect(),
            HandleType::Target => bounds.target.iter().collect(),
        },
        None => Vec::new(),
    }
}
