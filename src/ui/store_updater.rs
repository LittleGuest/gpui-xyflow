use crate::{
    core::{
        edge::Edge,
        geometry::{CoordinateExtent, NodeOrigin, ZIndexMode},
        graph::FitViewOptions,
        node::Node,
    },
    ui::store::{FlowProps, FlowStore},
};

pub const REACT_FLOW_FIELDS_TO_TRACK: &[&str] = &[
    "nodes",
    "edges",
    "defaultNodes",
    "defaultEdges",
    "onConnect",
    "onConnectStart",
    "onConnectEnd",
    "onClickConnectStart",
    "onClickConnectEnd",
    "nodesDraggable",
    "autoPanOnNodeFocus",
    "nodesConnectable",
    "nodesFocusable",
    "edgesFocusable",
    "edgesReconnectable",
    "elevateNodesOnSelect",
    "elevateEdgesOnSelect",
    "minZoom",
    "maxZoom",
    "nodeExtent",
    "onNodesChange",
    "onEdgesChange",
    "elementsSelectable",
    "connectionMode",
    "snapGrid",
    "snapToGrid",
    "translateExtent",
    "connectOnClick",
    "defaultEdgeOptions",
    "fitView",
    "fitViewOptions",
    "onNodesDelete",
    "onEdgesDelete",
    "onDelete",
    "onNodeDrag",
    "onNodeDragStart",
    "onNodeDragStop",
    "onSelectionDrag",
    "onSelectionDragStart",
    "onSelectionDragStop",
    "onMoveStart",
    "onMove",
    "onMoveEnd",
    "noPanClassName",
    "nodeOrigin",
    "autoPanOnConnect",
    "autoPanOnNodeDrag",
    "onError",
    "connectionRadius",
    "isValidConnection",
    "selectNodesOnDrag",
    "nodeDragThreshold",
    "connectionDragThreshold",
    "onBeforeDelete",
    "debug",
    "autoPanSpeed",
    "ariaLabelConfig",
    "zIndexMode",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    DedicatedSetter,
    Renamed,
    Direct,
}

pub fn classify_field(field: &str) -> FieldKind {
    match field {
        "nodes" | "edges" | "minZoom" | "maxZoom" | "translateExtent" | "nodeExtent" => {
            FieldKind::DedicatedSetter
        }
        "ariaLabelConfig" | "fitView" | "fitViewOptions" => FieldKind::Renamed,
        _ => FieldKind::Direct,
    }
}

pub fn is_tracked_field(field: &str) -> bool {
    REACT_FLOW_FIELDS_TO_TRACK.contains(&field)
}

#[derive(Clone, Debug, Default)]
pub struct ProviderOptions<S = String> {
    pub nodes: Option<Vec<Node<S>>>,
    pub edges: Option<Vec<Edge<S>>>,
    pub default_nodes: Option<Vec<Node<S>>>,
    pub default_edges: Option<Vec<Edge<S>>>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub fit_view: bool,
    pub fit_view_options: Option<FitViewOptions>,
    pub min_zoom: Option<f64>,
    pub max_zoom: Option<f64>,
    pub node_origin: Option<NodeOrigin>,
    pub node_extent: Option<CoordinateExtent>,
    pub z_index_mode: Option<ZIndexMode>,
}

pub fn create_store<S: Clone + PartialEq>(options: ProviderOptions<S>) -> FlowStore<S> {
    let mut props = FlowProps::default();
    if let Some(min_zoom) = options.min_zoom {
        props.min_zoom = min_zoom;
    }
    if let Some(max_zoom) = options.max_zoom {
        props.max_zoom = max_zoom;
    }
    if let Some(node_origin) = options.node_origin {
        props.node_origin = node_origin;
    }
    if let Some(node_extent) = options.node_extent {
        props.node_extent = node_extent;
    }
    if let Some(z_index_mode) = options.z_index_mode {
        props.z_index_mode = z_index_mode;
    }
    if let Some(padding) = options.fit_view_options.and_then(|o| o.padding) {
        props.fit_view_padding = padding;
    }

    let mut store = FlowStore::new(props);

    let store_nodes = options
        .default_nodes
        .clone()
        .or_else(|| options.nodes.clone())
        .unwrap_or_default();
    let store_edges = options
        .default_edges
        .clone()
        .or_else(|| options.edges.clone())
        .unwrap_or_default();

    if options.default_nodes.is_some() {
        store.set_default_nodes(store_nodes);
    } else {
        store.set_nodes(store_nodes);
    }
    if options.default_edges.is_some() {
        store.set_default_edges(store_edges);
    } else {
        store.set_edges(store_edges);
    }

    store.set_size(options.width.unwrap_or(0.0), options.height.unwrap_or(0.0));

    if options.fit_view {
        store.update_nodes();
        if options.width.is_some() && options.height.is_some() {
            store.request_fit_view();
            store.resolve_fit_view();
        }
    }

    store
}
