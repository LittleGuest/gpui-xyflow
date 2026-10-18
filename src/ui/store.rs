use std::{cell::RefCell, collections::BTreeMap, sync::Arc};

use crate::{
    core::{
        change::{
            EdgeChange, NodeChange, NodePositionChange, apply_edge_changes, apply_node_changes,
        },
        connect::{
            CheckHandleParams, ConnectionLine, ConnectionResult, ConnectionState, NoConnection,
            StartConnectionParams, UpdateConnectionParams, check_click_connection,
            start_connection, update_connection,
        },
        constants::{DEFAULT_NODE_ORIGIN, INFINITE_EXTENT},
        drag::{DragStore, UpdateNodesParams, parent_expansion_rect, update_nodes},
        edge::{ConnectionLineType, Edge, EdgeGraphics, PathOptions},
        geometry::{
            CoordinateExtent, Dimensions, EdgePosition, NodeOrigin, Padding, PanOnScrollMode, Rect,
            SelectionMode, SnapGrid, Transform, Viewport, XYPosition, ZIndexMode,
        },
        graph::{
            ConnectionMode, EdgePositionParams, get_edge_graphics, get_edge_position,
            get_elevated_edge_z_index,
        },
        node::{HandleType, InternalNode, Node, NodeDragItem, NodeLookup, ParentLookup},
        panzoom::{
            PanOnDrag, PanZoom, PanZoomOptions, transform_to_viewport, viewport_to_transform,
        },
        store::{
            AdoptOptions, NodeInternalsUpdate, adopt_user_nodes, handle_expand_parent,
            update_node_internals,
        },
        utils::{box_to_rect, get_rects_overlapping_area, get_viewport_for_bounds, node_to_box},
        xyminimap::minimap_bounds,
    },
    ui::{callbacks::FlowCallbacks, theme::FlowTheme},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClickHandle {
    Node,
    Edge,
    Pane,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InteractionState {
    pub dragging: bool,
    pub selecting: bool,
    pub connecting: bool,
    pub pane_clicked: bool,
    pub node_dragging: Option<String>,
    pub selection_start: Option<XYPosition>,
    pub selection_end: Option<XYPosition>,
}

impl InteractionState {
    pub fn selection_rect(&self) -> Option<Rect> {
        let start = self.selection_start?;
        let end = self.selection_end?;
        Some(Rect::new(
            start.x.min(end.x),
            start.y.min(end.y),
            (end.x - start.x).abs(),
            (end.y - start.y).abs(),
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FlowProps<S = String> {
    pub nodes_draggable: bool,
    pub nodes_connectable: bool,
    pub nodes_focusable: bool,
    pub edges_focusable: bool,
    pub elements_selectable: bool,
    pub edges_reconnectable: bool,
    pub nodes_deletable: bool,
    pub edges_deletable: bool,
    pub elevate_nodes_on_select: bool,
    pub elevate_edges_on_select: bool,
    pub pan_on_drag: PanOnDrag,
    pub selection_on_drag: bool,
    pub selection_mode: SelectionMode,
    pub node_origin: NodeOrigin,
    pub node_extent: CoordinateExtent,
    pub translate_extent: CoordinateExtent,
    pub snap_to_grid: bool,
    pub snap_grid: SnapGrid,
    pub connection_mode: ConnectionMode,
    pub connection_radius: f64,
    pub connection_line_type: ConnectionLineType,
    pub connect_on_click: bool,
    pub reconnect_radius: f64,
    pub min_zoom: f64,
    pub max_zoom: f64,
    pub default_viewport: Viewport,
    pub zoom_on_double_click: bool,
    pub zoom_on_scroll: bool,
    pub zoom_on_pinch: bool,
    pub pan_on_scroll: bool,
    pub pan_on_scroll_mode: PanOnScrollMode,
    pub prevent_scrolling: bool,
    pub delete_key_code: String,
    pub z_index_mode: ZIndexMode,
    pub only_render_visible_elements: bool,
    pub auto_pan_on_node_drag: bool,
    pub fit_view_padding: f64,
    pub has_default_nodes: bool,
    pub has_default_edges: bool,
    pub theme: FlowTheme,
    pub rf_id: String,
    pub width: f64,
    pub height: f64,
    pub nodes_selection_active: bool,
    pub user_selection_active: bool,
    pub user_selection_rect: Option<Rect>,
    pub pane_dragging: bool,
    pub no_pan_class_name: String,
    pub node_drag_threshold: f64,
    pub connection_drag_threshold: f64,
    pub select_nodes_on_drag: bool,
    pub multi_selection_active: bool,
    pub fit_view_queued: bool,
    pub connection_click_start_handle: Option<String>,
    pub aria_live_message: String,
    pub auto_pan_on_connect: bool,
    pub auto_pan_on_node_focus: bool,
    pub auto_pan_speed: f64,
    pub lib: String,
    pub debug: bool,
    pub auto_pan_on_drag: bool,
    pub selection_key_code: Option<String>,
    pub pan_activation_key_code: Option<String>,
    pub zoom_activation_key_code: Option<String>,
    pub disable_keyboard_a11y: bool,
    pub default_edge_options: Option<Edge<String>>,
    pub callbacks: FlowCallbacks<S>,
}

impl<S> Default for FlowProps<S> {
    fn default() -> Self {
        FlowProps {
            nodes_draggable: true,
            nodes_connectable: true,
            nodes_focusable: false,
            edges_focusable: false,
            elements_selectable: true,
            edges_reconnectable: true,
            nodes_deletable: true,
            edges_deletable: true,
            elevate_nodes_on_select: false,
            elevate_edges_on_select: false,
            pan_on_drag: PanOnDrag::Enabled,
            selection_on_drag: false,
            selection_mode: SelectionMode::Full,
            node_origin: DEFAULT_NODE_ORIGIN,
            node_extent: INFINITE_EXTENT,
            translate_extent: INFINITE_EXTENT,
            snap_to_grid: false,
            snap_grid: [1.0, 1.0],
            connection_mode: ConnectionMode::Strict,
            connection_radius: 20.0,
            connection_line_type: ConnectionLineType::Bezier,
            connect_on_click: true,
            reconnect_radius: 10.0,
            min_zoom: 0.5,
            max_zoom: 2.0,
            default_viewport: Viewport::IDENTITY,
            zoom_on_double_click: true,
            zoom_on_scroll: true,
            zoom_on_pinch: true,
            pan_on_scroll: false,
            pan_on_scroll_mode: PanOnScrollMode::Free,
            prevent_scrolling: true,
            delete_key_code: "Backspace".to_string(),
            z_index_mode: ZIndexMode::Auto,
            only_render_visible_elements: false,
            auto_pan_on_node_drag: true,
            fit_view_padding: 0.1,
            has_default_nodes: false,
            has_default_edges: false,
            theme: FlowTheme::light(),
            rf_id: "1".to_string(),
            width: 0.0,
            height: 0.0,
            nodes_selection_active: false,
            user_selection_active: false,
            user_selection_rect: None,
            pane_dragging: false,
            no_pan_class_name: "nopan".to_string(),
            node_drag_threshold: 1.0,
            connection_drag_threshold: 1.0,
            select_nodes_on_drag: true,
            multi_selection_active: false,
            fit_view_queued: false,
            connection_click_start_handle: None,
            aria_live_message: String::new(),
            auto_pan_on_connect: true,
            auto_pan_on_node_focus: false,
            auto_pan_speed: 15.0,
            lib: "gpui".to_string(),
            debug: false,
            auto_pan_on_drag: true,
            selection_key_code: Some("Shift".to_string()),
            pan_activation_key_code: None,
            zoom_activation_key_code: None,
            disable_keyboard_a11y: false,
            default_edge_options: None,
            callbacks: FlowCallbacks::empty(),
        }
    }
}

impl<S> FlowProps<S> {
    pub fn pan_zoom_options(&self) -> PanZoomOptions {
        PanZoomOptions {
            min_zoom: self.min_zoom,
            max_zoom: self.max_zoom,
            translate_extent: self.translate_extent,
            zoom_on_scroll: self.zoom_on_scroll,
            zoom_on_pinch: self.zoom_on_pinch,
            zoom_on_double_click: self.zoom_on_double_click,
            pan_on_drag: self.pan_on_drag.clone(),
            pan_on_scroll: self.pan_on_scroll,
            pan_on_scroll_mode: self.pan_on_scroll_mode,
            pan_on_scroll_speed: 0.5,
            prevent_scrolling: self.prevent_scrolling,
            user_selection_active: false,
            connection_in_progress: false,
            selection_on_drag: self.selection_on_drag,
            pane_click_distance: 1.0,
            pan_activation_key_pressed: false,
            zoom_activation_key_pressed: false,
        }
    }

    pub fn adopt_options(&self) -> AdoptOptions {
        AdoptOptions {
            node_origin: self.node_origin,
            node_extent: self.node_extent,
            elevate_nodes_on_select: self.elevate_nodes_on_select,
            z_index_mode: self.z_index_mode,
            check_equality: true,
        }
    }

    pub fn with_theme(mut self, theme: FlowTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn with_snap_to_grid(mut self, grid: SnapGrid) -> Self {
        self.snap_to_grid = true;
        self.snap_grid = grid;
        self
    }

    pub fn with_pan_on_scroll(mut self, mode: PanOnScrollMode) -> Self {
        self.pan_on_scroll = true;
        self.pan_on_scroll_mode = mode;
        self
    }

    pub fn with_connection_mode(mut self, mode: ConnectionMode) -> Self {
        self.connection_mode = mode;
        self
    }

    pub fn with_min_max_zoom(mut self, min_zoom: f64, max_zoom: f64) -> Self {
        self.min_zoom = min_zoom;
        self.max_zoom = max_zoom;
        self
    }
}

#[derive(Default)]
struct DragRuntime {
    store: DragStore,
    items: BTreeMap<String, NodeDragItem>,
}

pub struct FlowStore<S = String> {
    props: FlowProps<S>,
    nodes: Vec<Node<S>>,
    edges: Vec<Edge<S>>,
    node_lookup: NodeLookup<S>,
    parent_lookup: ParentLookup<S>,
    edge_lookup: BTreeMap<String, Edge<S>>,
    connection_lookup: BTreeMap<(String, Option<String>, String, Option<String>), String>,
    interaction: InteractionState,
    nodes_initialized: bool,
    transform: Transform,
    size: Dimensions,
    drag: RefCell<DragRuntime>,
    connection: ConnectionState<S>,
    pan_zoom: PanZoom,
    needs_update: bool,
    nodes_change_middleware: Vec<Arc<NodeChangeMiddleware<S>>>,
    edges_change_middleware: Vec<Arc<EdgeChangeMiddleware<S>>>,
}

pub type NodeChangeMiddleware<S> =
    dyn Fn(Vec<NodeChange<S>>, &[Node<S>]) -> Vec<NodeChange<S>> + Send + Sync;
pub type EdgeChangeMiddleware<S> =
    dyn Fn(Vec<EdgeChange<S>>, &[Edge<S>]) -> Vec<EdgeChange<S>> + Send + Sync;

impl<S: Clone + PartialEq> FlowStore<S> {
    pub fn new(props: FlowProps<S>) -> Self {
        let pan_zoom = PanZoom::new(props.default_viewport, props.pan_zoom_options());
        FlowStore {
            props,
            nodes: Vec::new(),
            edges: Vec::new(),
            node_lookup: BTreeMap::new(),
            parent_lookup: BTreeMap::new(),
            edge_lookup: BTreeMap::new(),
            connection_lookup: BTreeMap::new(),
            interaction: InteractionState::default(),
            nodes_initialized: false,
            transform: viewport_to_transform(Viewport::IDENTITY),
            size: Dimensions::new(0.0, 0.0),
            drag: RefCell::new(DragRuntime::default()),
            connection: ConnectionState::Idle(NoConnection::default()),
            pan_zoom,
            needs_update: false,
            nodes_change_middleware: Vec::new(),
            edges_change_middleware: Vec::new(),
        }
    }

    pub fn props(&self) -> &FlowProps<S> {
        &self.props
    }

    pub fn props_mut(&mut self) -> &mut FlowProps<S> {
        &mut self.props
    }

    pub fn set_props(&mut self, props: FlowProps<S>) {
        self.pan_zoom.set_options(props.pan_zoom_options());
        self.props = props;
        self.needs_update = true;
    }

    pub fn nodes(&self) -> &[Node<S>] {
        &self.nodes
    }

    pub fn edges(&self) -> &[Edge<S>] {
        &self.edges
    }

    pub fn node_lookup(&self) -> &NodeLookup<S> {
        &self.node_lookup
    }

    pub fn node_lookup_mut(&mut self) -> &mut NodeLookup<S> {
        &mut self.node_lookup
    }

    pub fn nodes_mut(&mut self) -> &mut Vec<Node<S>> {
        &mut self.nodes
    }

    pub fn edges_mut(&mut self) -> &mut Vec<Edge<S>> {
        &mut self.edges
    }

    pub fn edge_lookup(&self) -> &BTreeMap<String, Edge<S>> {
        &self.edge_lookup
    }

    pub fn interaction(&self) -> &InteractionState {
        &self.interaction
    }

    pub fn interaction_mut(&mut self) -> &mut InteractionState {
        &mut self.interaction
    }

    pub fn nodes_initialized(&self) -> bool {
        self.nodes_initialized
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub fn viewport(&self) -> Viewport {
        transform_to_viewport(self.transform)
    }

    pub fn size(&self) -> Dimensions {
        self.size
    }

    pub fn set_size(&mut self, width: f64, height: f64) {
        self.size = Dimensions::new(width, height);
        self.pan_zoom.set_extent(width, height);
    }

    pub fn set_default_nodes(&mut self, nodes: Vec<Node<S>>) {
        self.nodes = nodes;
        self.props.has_default_nodes = true;
        self.needs_update = true;
    }

    pub fn set_default_edges(&mut self, edges: Vec<Edge<S>>) {
        self.edges = edges;
        self.props.has_default_edges = true;
        self.needs_update = true;
    }

    pub fn has_default_nodes(&self) -> bool {
        self.props.has_default_nodes
    }

    pub fn has_default_edges(&self) -> bool {
        self.props.has_default_edges
    }

    pub fn set_nodes(&mut self, nodes: Vec<Node<S>>) {
        self.nodes = nodes;
        self.needs_update = true;
    }

    pub fn set_edges(&mut self, edges: Vec<Edge<S>>) {
        self.edges = edges;
        self.needs_update = true;
    }

    pub fn rebuild_connection_lookup(&mut self) {
        self.connection_lookup.clear();
        for edge in &self.edges {
            let key = (
                edge.source.clone(),
                edge.source_handle.clone(),
                edge.target.clone(),
                edge.target_handle.clone(),
            );
            self.connection_lookup.insert(key, edge.id.clone());
        }
        self.edge_lookup.clear();
        for edge in &self.edges {
            self.edge_lookup.insert(edge.id.clone(), edge.clone());
        }
    }

    pub fn connection_lookup(
        &self,
    ) -> &BTreeMap<(String, Option<String>, String, Option<String>), String> {
        &self.connection_lookup
    }

    pub fn connections_of(&self, node_id: &str) -> Vec<&Edge<S>> {
        self.edges
            .iter()
            .filter(|edge| edge.source == node_id || edge.target == node_id)
            .collect()
    }

    pub fn apply_node_changes(&mut self, changes: &[NodeChange<S>]) {
        self.nodes = apply_node_changes(changes, &self.nodes);
        for change in changes {
            if let NodeChange::Select(selection) = change {
                for node in self.nodes.iter_mut() {
                    if node.id == selection.id {
                        node.selected = selection.selected;
                    }
                }
                if let Some(node) = self.node_lookup.get_mut(&selection.id) {
                    node.selected = selection.selected;
                    node.internals.user_node.selected = selection.selected;
                }
            }
        }
        self.needs_update = true;
    }

    pub fn apply_edge_changes(&mut self, changes: &[EdgeChange<S>]) {
        self.edges = apply_edge_changes(changes, &self.edges);
        self.needs_update = true;
    }

    pub fn trigger_node_changes(&mut self, changes: Vec<NodeChange<S>>) {
        if changes.is_empty() {
            return;
        }
        if self.props.has_default_nodes {
            self.apply_node_changes(&changes);
        }
        if let Some(handler) = self.props.callbacks.on_nodes_change.clone() {
            handler(&changes);
        }
    }

    pub fn trigger_edge_changes(&mut self, changes: Vec<EdgeChange<S>>) {
        if changes.is_empty() {
            return;
        }
        if self.props.has_default_edges {
            self.apply_edge_changes(&changes);
        }
        if let Some(handler) = self.props.callbacks.on_edges_change.clone() {
            handler(&changes);
        }
    }

    pub fn apply_nodes_change_middleware(&self, changes: Vec<NodeChange<S>>) -> Vec<NodeChange<S>> {
        let mut current = changes;
        for middleware in self.nodes_change_middleware.iter() {
            current = middleware(current, &self.nodes);
        }
        current
    }

    pub fn apply_edges_change_middleware(&self, changes: Vec<EdgeChange<S>>) -> Vec<EdgeChange<S>> {
        let mut current = changes;
        for middleware in self.edges_change_middleware.iter() {
            current = middleware(current, &self.edges);
        }
        current
    }

    pub fn add_nodes_change_middleware<F>(&mut self, middleware: F)
    where
        F: Fn(Vec<NodeChange<S>>, &[Node<S>]) -> Vec<NodeChange<S>> + Send + Sync + 'static,
    {
        self.nodes_change_middleware.push(Arc::new(middleware));
    }

    pub fn add_edges_change_middleware<F>(&mut self, middleware: F)
    where
        F: Fn(Vec<EdgeChange<S>>, &[Edge<S>]) -> Vec<EdgeChange<S>> + Send + Sync + 'static,
    {
        self.edges_change_middleware.push(Arc::new(middleware));
    }

    pub fn update_node_positions(&mut self, drag_items: &[NodeDragItem], dragging: bool) {
        let mut changes: Vec<NodeChange<S>> = Vec::with_capacity(drag_items.len());
        for item in drag_items {
            let position = if item.expand_parent {
                XYPosition {
                    x: item.position.x.max(0.0),
                    y: item.position.y.max(0.0),
                }
            } else {
                item.position
            };
            changes.push(NodeChange::Position(NodePositionChange {
                id: item.id.clone(),
                position: Some(position),
                position_absolute: Some(item.position_absolute),
                dragging: Some(dragging),
            }));
        }
        let children: Vec<(String, String, Rect)> = drag_items
            .iter()
            .filter(|item| item.expand_parent)
            .filter_map(|item| {
                let parent_id = item.parent_id.clone()?;
                Some((item.id.clone(), parent_id, parent_expansion_rect(item)))
            })
            .collect();
        if !children.is_empty() {
            changes.extend(handle_expand_parent(
                &children,
                &self.node_lookup,
                &self.parent_lookup,
                self.props.node_origin,
            ));
        }
        let changes = self.apply_nodes_change_middleware(changes);
        self.trigger_node_changes(changes);
    }

    pub fn set_min_zoom(&mut self, min_zoom: f64) {
        if !min_zoom.is_finite() || min_zoom == self.props.min_zoom {
            return;
        }
        self.props.min_zoom = min_zoom;
        self.pan_zoom
            .set_scale_extent(self.props.min_zoom, self.props.max_zoom);
    }

    pub fn set_max_zoom(&mut self, max_zoom: f64) {
        if !max_zoom.is_finite() || max_zoom == self.props.max_zoom {
            return;
        }
        self.props.max_zoom = max_zoom;
        self.pan_zoom
            .set_scale_extent(self.props.min_zoom, self.props.max_zoom);
    }

    pub fn set_translate_extent(&mut self, extent: CoordinateExtent) {
        if extent == self.props.translate_extent {
            return;
        }
        self.props.translate_extent = extent;
        self.pan_zoom.set_translate_extent(extent);
    }

    pub fn set_node_extent(&mut self, node_extent: CoordinateExtent) {
        if node_extent == self.props.node_extent {
            return;
        }
        self.props.node_extent = node_extent;
        let mut options = self.props.adopt_options();
        options.check_equality = false;
        let mut parent_lookup = std::mem::take(&mut self.parent_lookup);
        adopt_user_nodes(
            &self.nodes,
            &mut self.node_lookup,
            &mut parent_lookup,
            &options,
        );
        self.parent_lookup = parent_lookup;
    }

    pub fn set_default_nodes_and_edges(&mut self, nodes: Vec<Node<S>>, edges: Vec<Edge<S>>) {
        self.nodes = nodes;
        self.edges = edges;
        self.props.has_default_nodes = true;
        self.props.has_default_edges = true;
        self.needs_update = true;
    }

    pub fn queued_fit_view(&self) -> bool {
        self.props.fit_view_queued
    }

    pub fn request_fit_view(&mut self) {
        self.props.fit_view_queued = true;
    }

    pub fn resolve_fit_view(&mut self) -> bool {
        let bounds = self.tight_bounds();
        if bounds.width > 0.0
            && bounds.height > 0.0
            && self.size.width > 0.0
            && self.size.height > 0.0
        {
            self.fit_bounds(bounds, self.props.fit_view_padding);
        }
        self.props.fit_view_queued = false;
        true
    }

    pub fn maybe_resolve_fit_view(&mut self) {
        if self.props.fit_view_queued && self.nodes_initialized {
            self.resolve_fit_view();
        }
    }

    pub fn update_nodes(&mut self) {
        if !self.needs_update && self.nodes_initialized {
            return;
        }
        let options = self.props.adopt_options();
        let mut parent_lookup = std::mem::take(&mut self.parent_lookup);
        adopt_user_nodes(
            &self.nodes,
            &mut self.node_lookup,
            &mut parent_lookup,
            &options,
        );
        self.parent_lookup = parent_lookup;
        self.rebuild_connection_lookup();
        self.nodes_initialized = true;
        self.needs_update = false;
    }

    pub fn update_node_internals(&mut self, updates: &[NodeInternalsUpdate]) {
        let options = self.props.adopt_options();
        let mut parent_lookup = std::mem::take(&mut self.parent_lookup);
        update_node_internals(updates, &mut self.node_lookup, &mut parent_lookup, &options);
        self.parent_lookup = parent_lookup;
    }

    pub fn add_selected_nodes(&mut self, ids: Vec<String>) {
        self.unselect_nodes_and_edges();
        for id in ids {
            if let Some(node) = self.node_lookup.get_mut(&id) {
                node.selected = true;
                node.internals.user_node.selected = true;
            }
        }
    }

    pub fn add_selected_edges(&mut self, ids: Vec<String>) {
        for id in ids {
            if let Some(edge) = self.edge_lookup.get_mut(&id) {
                edge.selected = true;
            }
        }
    }

    pub fn selected_node_ids(&self) -> Vec<String> {
        self.node_lookup
            .values()
            .filter(|node| node.selected)
            .map(|node| node.id.clone())
            .collect()
    }

    pub fn selected_edge_ids(&self) -> Vec<String> {
        self.edge_lookup
            .values()
            .filter(|edge| edge.selected)
            .map(|edge| edge.id.clone())
            .collect()
    }

    pub fn unselect_nodes_and_edges(&mut self) {
        for node in self.node_lookup.values_mut() {
            node.selected = false;
            node.internals.user_node.selected = false;
        }
        for edge in self.edge_lookup.values_mut() {
            edge.selected = false;
        }
        for node in self.nodes.iter_mut() {
            node.selected = false;
        }
        for edge in self.edges.iter_mut() {
            edge.selected = false;
        }
    }

    pub fn reset_selected_elements(&mut self) {
        self.unselect_nodes_and_edges();
    }

    pub fn selected_nodes(&self) -> Vec<&InternalNode<S>> {
        self.node_lookup
            .values()
            .filter(|node| node.selected)
            .collect()
    }

    pub fn selected_edges(&self) -> Vec<&Edge<S>> {
        self.edge_lookup
            .values()
            .filter(|edge| edge.selected)
            .collect()
    }

    pub fn selection_changes_for_rect(&self, rect: Rect) -> Vec<NodeChange<S>> {
        let mut changes = Vec::new();
        for node in self.node_lookup.values() {
            let node_rect = box_to_rect(node_to_box(
                &node.internals.user_node,
                self.props.node_origin,
            ));
            let overlap = get_rects_overlapping_area(node_rect, rect);
            let is_inside = match self.props.selection_mode {
                SelectionMode::Full => overlap >= node_rect.width * node_rect.height,
                SelectionMode::Partial => overlap > 0.0,
            };
            if is_inside != node.selected {
                changes.push(NodeChange::Select(crate::core::change::SelectionChange {
                    id: node.id.clone(),
                    selected: is_inside,
                }));
            }
        }
        changes
    }

    pub fn start_drag_items(&mut self, pointer: XYPosition) {
        let mut drag = self.drag.borrow_mut();
        drag.store = DragStore {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_lookup: self
                .node_lookup
                .clone()
                .into_iter()
                .map(|(k, v)| (k, node_to_string_internal(v)))
                .collect(),
            node_extent: self.props.node_extent,
            snap_grid: self.props.snap_grid,
            snap_to_grid: self.props.snap_to_grid,
            node_origin: self.props.node_origin,
            multi_selection_active: false,
            nodes_draggable: self.props.nodes_draggable,
            auto_pan_on_node_drag: self.props.auto_pan_on_node_drag,
            auto_pan_speed: 15.0,
            node_extent_override: None,
        };
        drag.items.clear();
        for node in self.node_lookup.values() {
            if node.selected {
                drag.items
                    .insert(node.id.clone(), NodeDragItem::from_internal(node, pointer));
            }
        }
    }

    pub fn move_nodes_by(&mut self, pointer: XYPosition) {
        let mut results: Vec<(String, XYPosition, XYPosition)> = Vec::new();
        {
            let mut drag = self.drag.borrow_mut();
            let mut items = std::mem::take(&mut drag.items);
            let store = DragStore {
                nodes: Vec::new(),
                edges: Vec::new(),
                node_lookup: self
                    .node_lookup
                    .clone()
                    .into_iter()
                    .map(|(k, v)| (k, node_to_string_internal(v)))
                    .collect(),
                node_extent: drag.store.node_extent,
                snap_grid: drag.store.snap_grid,
                snap_to_grid: drag.store.snap_to_grid,
                node_origin: drag.store.node_origin,
                multi_selection_active: drag.store.multi_selection_active,
                nodes_draggable: drag.store.nodes_draggable,
                auto_pan_on_node_drag: drag.store.auto_pan_on_node_drag,
                auto_pan_speed: drag.store.auto_pan_speed,
                node_extent_override: drag.store.node_extent_override,
            };
            let result = update_nodes(
                UpdateNodesParams {
                    pointer,
                    store: &store,
                },
                &mut items,
            );
            drag.items = items;
            for (id, position) in result.positions {
                results.push((id, position.position, position.position_absolute));
            }
        }

        for (id, position, position_absolute) in results {
            if let Some(node) = self.node_lookup.get_mut(&id) {
                node.position = position;
                node.internals.position_absolute = position_absolute;
                node.internals.user_node.position = position;
                node.dragging = true;
                node.internals.user_node.dragging = true;
            }
            if let Some(user_node) = self.nodes.iter_mut().find(|n| n.id == id) {
                user_node.position = position;
                user_node.dragging = true;
            }
        }
        self.needs_update = true;
    }

    pub fn end_drag(&mut self) {
        self.drag.borrow_mut().items.clear();
        for node in self.node_lookup.values_mut() {
            node.dragging = false;
            node.internals.user_node.dragging = false;
        }
        for node in self.nodes.iter_mut() {
            node.dragging = false;
        }
    }

    pub fn delete_elements(&mut self) {
        self.nodes.retain(|node| !node.selected);
        self.edges.retain(|edge| !edge.selected);
        self.node_lookup.retain(|_, node| !node.selected);
        self.edge_lookup.retain(|_, edge| !edge.selected);
        self.needs_update = true;
    }

    pub fn edge_position(&self, edge: &Edge<S>) -> Option<EdgePosition> {
        let source_node = self.node_lookup.get(&edge.source)?;
        let target_node = self.node_lookup.get(&edge.target)?;
        get_edge_position(EdgePositionParams {
            id: &edge.id,
            source_node,
            source_handle: edge.source_handle.as_deref(),
            target_node,
            target_handle: edge.target_handle.as_deref(),
            connection_mode: self.props.connection_mode,
        })
    }

    pub fn edge_path_for(&self, edge: &Edge<S>) -> Option<EdgeGraphics> {
        let position = self.edge_position(edge)?;
        let edge_type = edge
            .edge_type
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let path_options: Option<&PathOptions> = edge.path_options.as_ref();
        Some(get_edge_graphics(&edge_type, &position, path_options))
    }

    pub fn edge_z_index(&self, edge: &Edge<S>) -> f64 {
        let source = self.node_lookup.get(&edge.source);
        let target = self.node_lookup.get(&edge.target);
        match (source, target) {
            (Some(source), Some(target)) => get_elevated_edge_z_index(
                source,
                target,
                edge.selected,
                edge.z_index.unwrap_or(0.0),
                self.props.elevate_edges_on_select,
                self.props.z_index_mode,
            ),
            _ => edge.z_index.unwrap_or(0.0),
        }
    }

    pub fn tight_bounds(&self) -> Rect {
        minimap_bounds(&self.node_lookup).unwrap_or_else(|| Rect::new(0.0, 0.0, 0.0, 0.0))
    }

    pub fn pan_zoom(&self) -> &PanZoom {
        &self.pan_zoom
    }

    pub fn pan_zoom_mut(&mut self) -> &mut PanZoom {
        &mut self.pan_zoom
    }

    pub fn sync_transform(&mut self) {
        self.transform = self.pan_zoom.transform();
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.pan_zoom.set_viewport(viewport);
        self.sync_transform();
    }

    pub fn pan_by(&mut self, delta: XYPosition) {
        self.pan_zoom.pan_by(delta);
        self.sync_transform();
    }

    pub fn zoom_in(&mut self) {
        self.pan_zoom.zoom_in();
        self.sync_transform();
    }

    pub fn zoom_out(&mut self) {
        self.pan_zoom.zoom_out();
        self.sync_transform();
    }

    pub fn zoom_to(&mut self, zoom: f64) {
        self.pan_zoom.scale_to(zoom, None);
        self.sync_transform();
    }

    pub fn get_zoom(&self) -> f64 {
        self.transform.zoom()
    }

    pub fn set_center(&mut self, x: f64, y: f64) {
        let width = self.size.width;
        let height = self.size.height;
        let zoom = self.transform.zoom();
        self.set_viewport(Viewport {
            x: width / 2.0 - x * zoom,
            y: height / 2.0 - y * zoom,
            zoom,
        });
    }

    pub fn fit_bounds(&mut self, bounds: Rect, padding: f64) {
        let viewport = get_viewport_for_bounds(
            bounds,
            self.size.width,
            self.size.height,
            self.props.min_zoom,
            self.props.max_zoom,
            Padding::all(padding),
        );
        self.set_viewport(viewport);
    }

    pub fn fit_view(&mut self) {
        let bounds = self.tight_bounds();
        if bounds.width > 0.0 && bounds.height > 0.0 {
            self.fit_bounds(bounds, self.props.fit_view_padding);
        }
    }

    pub fn screen_to_flow_position(&self, screen: XYPosition) -> XYPosition {
        self.transform.invert(screen)
    }

    pub fn flow_to_screen_position(&self, flow: XYPosition) -> XYPosition {
        self.transform.apply(flow)
    }

    pub fn start_connection(
        &mut self,
        node_id: &str,
        handle_type: HandleType,
        handle_id: Option<&str>,
        pointer: XYPosition,
    ) {
        let next = start_connection(StartConnectionParams {
            node_id,
            handle_id,
            handle_type,
            pointer,
            transform: self.transform,
            connection_mode: self.props.connection_mode,
            connection_radius: self.props.connection_radius,
            node_lookup: &self.node_lookup,
        });
        if let Some(state) = next {
            self.connection = ConnectionState::InProgress(state);
            self.interaction.connecting = true;
        }
    }

    pub fn update_connection(&mut self, pointer: XYPosition) {
        if let ConnectionState::InProgress(state) = &self.connection {
            let valid = |_c: &crate::core::edge::Connection| true;
            let next = update_connection(UpdateConnectionParams {
                state,
                pointer,
                transform: self.transform,
                connection_mode: self.props.connection_mode,
                connection_radius: self.props.connection_radius,
                is_valid_connection: &valid,
                node_lookup: &self.node_lookup,
            });
            self.connection = ConnectionState::InProgress(next);
        }
    }

    pub fn check_click_connection(&self, _pointer: XYPosition) -> Option<ConnectionResult> {
        let from = match &self.connection {
            ConnectionState::InProgress(state) => state,
            ConnectionState::Idle(_) => return None,
        };
        if !self.props.connect_on_click {
            return None;
        }
        let valid = |_c: &crate::core::edge::Connection| true;
        let handle_id = from.from_handle.id.clone();
        Some(check_click_connection(CheckHandleParams {
            node_id: &from.from_handle.node_id,
            handle_id: handle_id.as_deref(),
            handle_type: from.from_handle.handle_type,
            from_node_id: &from.from_handle.node_id,
            from_handle_id: handle_id.as_deref(),
            from_type: from.from_handle.handle_type,
            connection_mode: self.props.connection_mode,
            is_valid_connection: &valid,
            node_lookup: &self.node_lookup,
        }))
    }

    pub fn cancel_connection(&mut self) {
        self.connection = ConnectionState::Idle(NoConnection::default());
        self.interaction.connecting = false;
    }

    pub fn connection_state(&self) -> &ConnectionState<S> {
        &self.connection
    }

    pub fn connection_line(&self) -> Option<ConnectionLine> {
        match &self.connection {
            ConnectionState::InProgress(state) => Some(ConnectionLine {
                source_x: state.from.x,
                source_y: state.from.y,
                source_position: state.from_position,
                target_x: state.to.x,
                target_y: state.to.y,
                target_position: state.to_position_or_opposite(),
            }),
            ConnectionState::Idle(_) => None,
        }
    }

    pub fn clone_snapshot(&self) -> FlowStore<S> {
        FlowStore {
            props: self.props.clone(),
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            node_lookup: self.node_lookup.clone(),
            parent_lookup: self.parent_lookup.clone(),
            edge_lookup: self.edge_lookup.clone(),
            connection_lookup: self.connection_lookup.clone(),
            interaction: self.interaction.clone(),
            nodes_initialized: self.nodes_initialized,
            transform: self.transform,
            size: self.size,
            drag: RefCell::new(DragRuntime::default()),
            connection: self.connection.clone(),
            pan_zoom: self.pan_zoom.clone(),
            needs_update: self.needs_update,
            nodes_change_middleware: self.nodes_change_middleware.clone(),
            edges_change_middleware: self.edges_change_middleware.clone(),
        }
    }

    pub fn needs_update(&self) -> bool {
        self.needs_update
    }

    pub fn request_update(&mut self) {
        self.needs_update = true;
    }
}

impl<S: Clone + PartialEq> Default for FlowStore<S> {
    fn default() -> Self {
        FlowStore::new(FlowProps::default())
    }
}

fn node_to_string_node<S: Clone>(node: Node<S>) -> Node<String> {
    Node {
        id: node.id,
        position: node.position,
        data: String::new(),
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
        handles: node.handles,
        measured: node.measured,
    }
}

fn node_to_string_internal<S: Clone>(node: InternalNode<S>) -> InternalNode<String> {
    InternalNode {
        id: node.id,
        position: node.position,
        data: String::new(),
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
        internals: node_to_string_internals(node.internals),
    }
}

fn node_to_string_internals<S: Clone>(
    internals: crate::core::node::Internals<S>,
) -> crate::core::node::Internals<String> {
    crate::core::node::Internals {
        position_absolute: internals.position_absolute,
        z: internals.z,
        root_parent_index: internals.root_parent_index,
        user_node: node_to_string_node(internals.user_node),
        handle_bounds: internals.handle_bounds,
        bounds: internals.bounds,
    }
}
