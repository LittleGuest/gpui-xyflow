use crate::core::geometry::{CoordinateExtent, NodeOrigin, SnapGrid};
use crate::core::node::HandleType;

pub const ELEMENT_SELECTION_KEYS: [&str; 3] = ["Enter", " ", "Escape"];

pub const NO_DRAG_CLASS: &str = "nodrag";
pub const NO_WHEEL_CLASS: &str = "nowheel";
pub const NO_PAN_CLASS: &str = "nopan";
pub const NO_KEY_CLASS: &str = "nokey";

pub const DEFAULT_MIN_ZOOM: f64 = 0.5;
pub const DEFAULT_MAX_ZOOM: f64 = 2.0;
pub const DEFAULT_MARKER_COLOR: &str = "#b1b1b7";
pub const DEFAULT_CONNECTION_RADIUS: f64 = 20.0;
pub const DEFAULT_RECONNECT_RADIUS: f64 = 10.0;
pub const DEFAULT_SNAP_GRID: SnapGrid = [15.0, 15.0];
pub const DEFAULT_PAN_ON_SCROLL_SPEED: f64 = 0.5;
pub const DEFAULT_PANE_CLICK_DISTANCE: f64 = 1.0;
pub const DEFAULT_AUTO_PAN_SPEED: f64 = 15.0;
pub const DEFAULT_AUTO_PAN_DISTANCE: f64 = 40.0;
pub const DEFAULT_NODE_DRAG_THRESHOLD: f64 = 1.0;
pub const DEFAULT_CONNECTION_DRAG_THRESHOLD: f64 = 1.0;
pub const SELECTED_NODE_Z: f64 = 1000.0;
pub const ROOT_PARENT_Z_INCREMENT: f64 = 10.0;
pub const WHEEL_DELAY: u64 = 150;
pub const PAN_SCROLL_END_DELAY: u64 = 150;
pub const DBLCLICK_ZOOM_DURATION: f64 = 250.0;
pub const ZOOM_IN_FACTOR: f64 = 1.2;
pub const ZOOM_OUT_FACTOR: f64 = 1.0 / 1.2;
pub const ADDITIONAL_DISTANCE: f64 = 250.0;
pub const MINIMAP_DEFAULT_WIDTH: f64 = 200.0;
pub const MINIMAP_DEFAULT_HEIGHT: f64 = 150.0;
pub const MINIMAP_OFFSET_SCALE: f64 = 5.0;
pub const ARROW_KEY_STEP: f64 = 5.0;
pub const ARROW_KEY_SHIFT_FACTOR: f64 = 4.0;

pub const INFINITE_EXTENT: CoordinateExtent = [
    [f64::NEG_INFINITY, f64::NEG_INFINITY],
    [f64::INFINITY, f64::INFINITY],
];

pub const DEFAULT_NODE_ORIGIN: NodeOrigin = [0.0, 0.0];

pub const CLASS_PREFIX: &str = "gpui-xy-flow";

pub mod error_messages {
    use super::HandleType;

    pub fn error_001(lib: &str) -> String {
        format!(
            "Seems like you have not used {}Provider as an ancestor. Help: https://{lib}flow.dev/error#001",
            if lib == "svelte" {
                "SvelteFlow"
            } else {
                "ReactFlow"
            }
        )
    }

    pub fn error_002() -> &'static str {
        "It looks like you've created a new nodeTypes or edgeTypes object. If this wasn't on purpose please define the nodeTypes/edgeTypes outside of the component or memoize them."
    }

    pub fn error_003(node_type: &str) -> String {
        format!("Node type \"{node_type}\" not found. Using fallback type \"default\".")
    }

    pub fn error_004() -> &'static str {
        "The parent container needs a width and a height to render the graph."
    }

    pub fn error_005() -> &'static str {
        "Only child nodes can use a parent extent."
    }

    pub fn error_006() -> &'static str {
        "Can't create edge. An edge needs a source and a target."
    }

    pub fn error_007(id: &str) -> String {
        format!("The old edge with id={id} does not exist.")
    }

    pub fn error_008(
        handle_type: HandleType,
        id: &str,
        source_handle: Option<&str>,
        target_handle: Option<&str>,
    ) -> String {
        let handle_id = match handle_type {
            HandleType::Source => source_handle,
            HandleType::Target => target_handle,
        }
        .unwrap_or("null");
        format!(
            "Couldn't create edge for {} handle id: \"{}\", edge id: {}.",
            handle_type.as_str(),
            handle_id,
            id
        )
    }

    pub fn error_009(marker_type: &str) -> String {
        format!("Marker type \"{marker_type}\" doesn't exist.")
    }

    pub fn error_010() -> &'static str {
        "Handle: No node id found. Make sure to only use a Handle inside a custom Node."
    }

    pub fn error_011(edge_type: &str) -> String {
        format!("Edge type \"{edge_type}\" not found. Using fallback type \"default\".")
    }

    pub fn error_012(id: &str) -> String {
        format!(
            "Node with id \"{id}\" does not exist, it may have been removed. This can happen when a node is deleted before the \"onNodeClick\" handler is called."
        )
    }

    pub fn error_013(lib: &str) -> String {
        format!(
            "It seems that you haven't loaded the styles. Please import '@xyflow/{lib}/dist/style.css' or base.css to make sure everything is working properly."
        )
    }

    pub fn error_014() -> &'static str {
        "useNodeConnections: No node ID found. Call useNodeConnections inside a custom Node or provide a node ID."
    }

    pub fn error_015() -> &'static str {
        "It seems that you are trying to drag a node that is not initialized. Please use onNodesChange as explained in the docs."
    }

    pub fn error_016(id: &str) -> String {
        format!(
            "Edge with id \"{id}\" does not exist, it may have been removed. This can happen when an edge is deleted before the \"onEdgeClick\" handler is called."
        )
    }
}
