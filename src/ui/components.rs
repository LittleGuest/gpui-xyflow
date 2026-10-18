use crate::{
    core::{
        change::{EdgeChange, NodeChange},
        edge::Edge,
        geometry::{Rect, Transform, XYPosition},
        graph::get_selected_nodes_bounds,
        node::{Node, NodeLookup},
    },
    ui::store::FlowProps,
};

pub const ARIA_NODE_DESC_KEY: &str = "react-flow__node-desc";
pub const ARIA_EDGE_DESC_KEY: &str = "react-flow__edge-desc";
pub const ARIA_LIVE_MESSAGE: &str = "react-flow__aria-live";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A11yDescriptionsProps {
    pub rf_id: String,
    pub disable_keyboard_a11y: bool,
}

impl Default for A11yDescriptionsProps {
    fn default() -> Self {
        A11yDescriptionsProps {
            rf_id: String::from("react-flow"),
            disable_keyboard_a11y: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A11yDescriptions {
    pub node_desc_id: String,
    pub node_desc: String,
    pub edge_desc_id: String,
    pub edge_desc: String,
    pub live_message_id: Option<String>,
    pub live_message: Option<String>,
}

impl A11yDescriptions {
    pub fn node_desc_id(rf_id: &str) -> String {
        format!("{ARIA_NODE_DESC_KEY}-{rf_id}")
    }

    pub fn edge_desc_id(rf_id: &str) -> String {
        format!("{ARIA_EDGE_DESC_KEY}-{rf_id}")
    }

    pub fn live_message_id(rf_id: &str) -> String {
        format!("{ARIA_LIVE_MESSAGE}-{rf_id}")
    }
}

pub fn a11y_descriptions<S: Clone>(
    props: &FlowProps<S>,
    rf_id: &str,
    aria_live_message: &str,
) -> A11yDescriptions {
    let config = &props.aria_label_config;

    let live_message_id = if props.disable_keyboard_a11y {
        None
    } else {
        Some(A11yDescriptions::live_message_id(rf_id))
    };

    A11yDescriptions {
        node_desc_id: A11yDescriptions::node_desc_id(rf_id),
        node_desc: if props.disable_keyboard_a11y {
            config.node_a11y_description_default.clone()
        } else {
            config.node_a11y_description_keyboard_disabled.clone()
        },
        edge_desc_id: A11yDescriptions::edge_desc_id(rf_id),
        edge_desc: config.edge_a11y_description_default.clone(),
        live_message_id,
        live_message: if props.disable_keyboard_a11y {
            None
        } else {
            Some(aria_live_message.to_string())
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribution {
    pub visible: bool,
    pub rf_id: String,
    pub href: String,
    pub label: String,
    pub panel_position: String,
    pub message: String,
}

pub fn attribution(lib: &str, hide_attribution: bool, panel_position: &str) -> Attribution {
    Attribution {
        visible: !hide_attribution,
        rf_id: format!("{lib}-flow"),
        href: format!("https://{lib}flow.dev"),
        label: format!(
            "{} Flow",
            lib.chars()
                .next()
                .map(|first| first.to_uppercase().to_string())
                .unwrap_or_default()
                + &lib.chars().skip(1).collect::<String>()
        ),
        panel_position: panel_position.to_string(),
        message: crate::core::utils_extra::handle_attribution_warning_message(lib),
    }
}

impl Attribution {
    pub fn class_name(&self) -> String {
        format!("{}__attribution", self.rf_id)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PortalTarget {
    pub available: bool,
}

impl PortalTarget {
    pub fn available() -> Self {
        PortalTarget { available: true }
    }

    pub fn class_name(layer: crate::ui::container::RenderLayer, rf_id: &str) -> String {
        layer.class_name(rf_id)
    }
}

pub fn edge_label_portal(rf_id: &str, available: bool) -> PortalTarget {
    let _ = PortalTarget::class_name(crate::ui::container::RenderLayer::EdgeLabels, rf_id);
    PortalTarget { available }
}

pub fn viewport_portal(rf_id: &str, available: bool) -> PortalTarget {
    let _ = PortalTarget::class_name(crate::ui::container::RenderLayer::ViewportPortal, rf_id);
    PortalTarget { available }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UserSelectionRect {
    pub active: bool,
    pub rect: Option<Rect>,
}

impl UserSelectionRect {
    pub fn from_state(active: bool, rect: Option<Rect>) -> Self {
        UserSelectionRect { active, rect }
    }

    pub fn is_visible(&self) -> bool {
        self.active && self.rect.is_some()
    }

    pub fn transform(&self) -> Option<String> {
        self.rect.map(|rect| {
            format!(
                "translate({}px, {}px)",
                crate::core::utils::format_number(rect.x),
                crate::core::utils::format_number(rect.y)
            )
        })
    }

    pub fn class_name(&self, rf_id: &str) -> String {
        format!("{rf_id}__selection {rf_id}__container")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodesSelection {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub transform: Transform,
    pub offset: XYPosition,
    pub user_selection_active: bool,
}

impl Default for NodesSelection {
    fn default() -> Self {
        NodesSelection {
            width: None,
            height: None,
            transform: Transform::IDENTITY,
            offset: XYPosition::ZERO,
            user_selection_active: false,
        }
    }
}

pub fn nodes_selection<S: Clone>(
    node_lookup: &NodeLookup<S>,
    transform: Transform,
    user_selection_active: bool,
) -> NodesSelection {
    let bounds = get_selected_nodes_bounds(node_lookup);

    let width = if crate::core::utils::is_numeric(bounds.width) {
        Some(bounds.width)
    } else {
        None
    };
    let height = if crate::core::utils::is_numeric(bounds.height) {
        Some(bounds.height)
    } else {
        None
    };

    NodesSelection {
        width,
        height,
        transform,
        offset: XYPosition::new(bounds.x, bounds.y),
        user_selection_active,
    }
}

impl NodesSelection {
    pub fn should_render(&self) -> bool {
        !self.user_selection_active && self.width.is_some() && self.height.is_some()
    }

    pub fn transform_string(&self) -> String {
        format!(
            "translate({}px,{}px) scale({}) translate({}px,{}px)",
            crate::core::utils::format_number(self.transform.x()),
            crate::core::utils::format_number(self.transform.y()),
            crate::core::utils::format_number(self.transform.zoom()),
            crate::core::utils::format_number(self.offset.x),
            crate::core::utils::format_number(self.offset.y)
        )
    }

    pub fn class_name(&self, rf_id: &str) -> String {
        format!("{rf_id}__nodesselection {rf_id}__container")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrowKey {
    Up,
    Down,
    Left,
    Right,
}

impl ArrowKey {
    pub fn from_str(value: &str) -> Option<ArrowKey> {
        match value {
            "ArrowUp" => Some(ArrowKey::Up),
            "ArrowDown" => Some(ArrowKey::Down),
            "ArrowLeft" => Some(ArrowKey::Left),
            "ArrowRight" => Some(ArrowKey::Right),
            _ => None,
        }
    }

    pub fn diff(self) -> XYPosition {
        match self {
            ArrowKey::Up => XYPosition::new(0.0, -1.0),
            ArrowKey::Down => XYPosition::new(0.0, 1.0),
            ArrowKey::Left => XYPosition::new(-1.0, 0.0),
            ArrowKey::Right => XYPosition::new(1.0, 0.0),
        }
    }
}

pub fn arrow_key_diff(key: &str) -> Option<XYPosition> {
    ArrowKey::from_str(key).map(|key| key.diff())
}

pub fn move_selected_nodes_delta(key: &str, shift: bool, zoom: f64) -> Option<XYPosition> {
    let diff = arrow_key_diff(key)?;
    let factor = if shift { 4.0 } else { 1.0 };
    let zoom = if zoom == 0.0 { 1.0 } else { zoom };
    Some(XYPosition::new(
        diff.x * factor / zoom,
        diff.y * factor / zoom,
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionSnapshot<S = String> {
    pub nodes: Vec<Node<S>>,
    pub edges: Vec<Edge<S>>,
}

pub fn selected_ids<S: Clone>(
    node_lookup: &NodeLookup<S>,
    edges: &[Edge<S>],
) -> (Vec<String>, Vec<String>) {
    let node_ids: Vec<String> = node_lookup
        .values()
        .filter(|node| node.selected)
        .map(|node| node.id.clone())
        .collect();
    let edge_ids: Vec<String> = edges
        .iter()
        .filter(|edge| edge.selected)
        .map(|edge| edge.id.clone())
        .collect();
    (node_ids, edge_ids)
}

pub fn selection_changed<S: Clone + PartialEq>(
    previous: &SelectionSnapshot<S>,
    node_lookup: &NodeLookup<S>,
    edges: &[Edge<S>],
) -> bool {
    let (node_ids, edge_ids) = selected_ids(node_lookup, edges);
    let previous_node_ids: Vec<String> = previous.nodes.iter().map(|n| n.id.clone()).collect();
    let previous_edge_ids: Vec<String> = previous.edges.iter().map(|e| e.id.clone()).collect();
    node_ids != previous_node_ids || edge_ids != previous_edge_ids
}

pub fn selection_snapshot<S: Clone>(
    node_lookup: &NodeLookup<S>,
    edges: &[Edge<S>],
) -> SelectionSnapshot<S> {
    SelectionSnapshot {
        nodes: node_lookup
            .values()
            .filter(|node| node.selected)
            .map(|node| node.internals.user_node.clone())
            .collect(),
        edges: edges.iter().filter(|edge| edge.selected).cloned().collect(),
    }
}

pub fn should_mount_selection_listener(has_callback: bool, handler_count: usize) -> bool {
    has_callback || handler_count > 0
}

#[derive(Clone, Debug, PartialEq)]
pub struct BatchQueue<S = String> {
    pending_nodes: Vec<Vec<Node<S>>>,
    pending_edges: Vec<Vec<Edge<S>>>,
}

impl<S> Default for BatchQueue<S> {
    fn default() -> Self {
        BatchQueue {
            pending_nodes: Vec::new(),
            pending_edges: Vec::new(),
        }
    }
}

impl<S: Clone> BatchQueue<S> {
    pub fn push_nodes(&mut self, nodes: Vec<Node<S>>) {
        self.pending_nodes.push(nodes);
    }

    pub fn push_edges(&mut self, edges: Vec<Edge<S>>) {
        self.pending_edges.push(edges);
    }

    pub fn has_pending(&self) -> bool {
        !self.pending_nodes.is_empty() || !self.pending_edges.is_empty()
    }

    pub fn pending_node_count(&self) -> usize {
        self.pending_nodes.len()
    }

    pub fn pending_edge_count(&self) -> usize {
        self.pending_edges.len()
    }

    pub fn drain_nodes(&mut self, current: &[Node<S>]) -> (Vec<Node<S>>, bool) {
        if self.pending_nodes.is_empty() {
            return (current.to_vec(), false);
        }
        let mut next = current.to_vec();
        for payload in self.pending_nodes.drain(..) {
            next = payload;
        }
        (next, true)
    }

    pub fn drain_edges(&mut self, current: &[Edge<S>]) -> (Vec<Edge<S>>, bool) {
        if self.pending_edges.is_empty() {
            return (current.to_vec(), false);
        }
        let mut next = current.to_vec();
        for payload in self.pending_edges.drain(..) {
            next = payload;
        }
        (next, true)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BatchResult<S = String> {
    pub nodes: Vec<Node<S>>,
    pub node_changes: Vec<NodeChange<S>>,
    pub edges: Vec<Edge<S>>,
    pub edge_changes: Vec<EdgeChange<S>>,
}

pub fn process_node_queue<S: Clone + PartialEq>(
    queue: &mut BatchQueue<S>,
    current: &[Node<S>],
    node_lookup: &NodeLookup<S>,
) -> (Vec<Node<S>>, Vec<NodeChange<S>>) {
    let (next, changed) = queue.drain_nodes(current);
    if !changed {
        return (next, Vec::new());
    }
    let changes = crate::core::change::get_nodes_diff_changes(&next, node_lookup);
    (next, changes)
}

pub fn process_edge_queue<S: Clone + PartialEq>(
    queue: &mut BatchQueue<S>,
    current: &[Edge<S>],
    edge_lookup: &crate::core::edge::EdgeLookup<S>,
) -> (Vec<Edge<S>>, Vec<EdgeChange<S>>) {
    let (next, changed) = queue.drain_edges(current);
    if !changed {
        return (next, Vec::new());
    }
    let changes = crate::core::change::get_edges_diff_changes(&next, edge_lookup);
    (next, changes)
}
