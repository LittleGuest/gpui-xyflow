use std::collections::BTreeMap;

use crate::core::{
    edge::{Connection, Edge, EdgeLookup, HandleConnection},
    graph::ConnectionMode,
    node::Handle,
};

pub type ConnectionLookup = BTreeMap<String, BTreeMap<String, HandleConnection>>;

pub fn get_connection_key(
    source: &str,
    source_handle: Option<&str>,
    target: &str,
    target_handle: Option<&str>,
) -> String {
    format!(
        "{}-{}--{}-{}",
        source,
        source_handle.unwrap_or(""),
        target,
        target_handle.unwrap_or("")
    )
}

pub fn add_connection_to_lookup(
    handle_type: &str,
    connection: &HandleConnection,
    connection_key: &str,
    lookup: &mut ConnectionLookup,
    node_id: &str,
    handle_id: Option<&str>,
) {
    let mut insert = |key: String| {
        let map = lookup.entry(key).or_default();
        map.insert(connection_key.to_string(), connection.clone());
    };

    insert(node_id.to_string());
    insert(format!("{node_id}-{handle_type}"));
    if let Some(handle_id) = handle_id {
        insert(format!("{node_id}-{handle_type}-{handle_id}"));
    }
}

pub fn update_connection_lookup<S: Clone>(
    connection_lookup: &mut ConnectionLookup,
    edge_lookup: &mut EdgeLookup<S>,
    edges: &[Edge<S>],
) {
    connection_lookup.clear();
    edge_lookup.clear();

    for edge in edges {
        let connection = HandleConnection {
            connection: Connection {
                source: edge.source.clone(),
                target: edge.target.clone(),
                source_handle: edge.source_handle.clone(),
                target_handle: edge.target_handle.clone(),
            },
            edge_id: edge.id.clone(),
        };

        let source_key = get_connection_key(
            &edge.source,
            edge.source_handle.as_deref(),
            &edge.target,
            edge.target_handle.as_deref(),
        );
        let target_key = get_connection_key(
            &edge.target,
            edge.target_handle.as_deref(),
            &edge.source,
            edge.source_handle.as_deref(),
        );

        add_connection_to_lookup(
            "source",
            &connection,
            &target_key,
            connection_lookup,
            &edge.source,
            edge.source_handle.as_deref(),
        );
        add_connection_to_lookup(
            "target",
            &connection,
            &source_key,
            connection_lookup,
            &edge.target,
            edge.target_handle.as_deref(),
        );

        edge_lookup.insert(edge.id.clone(), edge.clone());
    }
}

pub fn are_connection_maps_equal(
    a: Option<&BTreeMap<String, HandleConnection>>,
    b: Option<&BTreeMap<String, HandleConnection>>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.keys().all(|key| b.contains_key(key))
        }
        _ => false,
    }
}

pub fn handle_connection_change<F>(
    a: &BTreeMap<String, HandleConnection>,
    b: &BTreeMap<String, HandleConnection>,
    cb: F,
) where
    F: FnOnce(&[HandleConnection]),
{
    let diff: Vec<HandleConnection> = a
        .iter()
        .filter(|(key, _)| !b.contains_key(*key))
        .map(|(_, connection)| connection.clone())
        .collect();

    if !diff.is_empty() {
        cb(&diff);
    }
}

pub fn connection_status(is_valid: Option<bool>) -> Option<&'static str> {
    match is_valid {
        None => None,
        Some(true) => Some("valid"),
        Some(false) => Some("invalid"),
    }
}

pub fn get_connection_status(connection: &Connection, edges: &[Edge]) -> (bool, Option<bool>) {
    if connection.source.is_empty() || connection.target.is_empty() {
        return (false, None);
    }
    let exists = edges.iter().any(|edge| {
        edge.source == connection.source
            && edge.target == connection.target
            && edge.source_handle.as_deref() == connection.source_handle.as_deref()
            && edge.target_handle.as_deref() == connection.target_handle.as_deref()
    });
    (true, Some(!exists))
}

pub fn is_valid_connection(source: &str, target: &str, connection_mode: ConnectionMode) -> bool {
    if source == target {
        return false;
    }
    match connection_mode {
        ConnectionMode::Strict | ConnectionMode::Loose => true,
    }
}

pub fn is_horizontal_handle(handle: &Handle) -> bool {
    handle.position.is_horizontal()
}

pub fn get_edge_id(
    source: &str,
    source_handle: Option<&str>,
    target: &str,
    target_handle: Option<&str>,
) -> String {
    format!(
        "xy-edge__{}{}-{}{}",
        source,
        source_handle.unwrap_or(""),
        target,
        target_handle.unwrap_or("")
    )
}

pub fn connection_exists<S: Clone>(
    edge: &crate::core::edge::Edge<S>,
    edges: &[crate::core::edge::Edge<S>],
) -> bool {
    edges.iter().any(|candidate| {
        candidate.source == edge.source
            && candidate.target == edge.target
            && (candidate.source_handle == edge.source_handle
                || (candidate.source_handle.is_none() && edge.source_handle.is_none()))
            && (candidate.target_handle == edge.target_handle
                || (candidate.target_handle.is_none() && edge.target_handle.is_none()))
    })
}

pub fn add_edge<S: Clone>(
    edge_params: crate::core::edge::Edge<S>,
    edges: &[crate::core::edge::Edge<S>],
    default_edge_options: Option<&crate::core::edge::Edge<S>>,
) -> Option<Vec<crate::core::edge::Edge<S>>> {
    if edge_params.source == edge_params.target {
        return None;
    }
    let mut next = edge_params.clone();
    if next.id.is_empty() {
        next.id = get_edge_id(
            &next.source,
            next.source_handle.as_deref(),
            &next.target,
            next.target_handle.as_deref(),
        );
    }
    if let Some(options) = default_edge_options {
        if next.edge_type.is_none() {
            next.edge_type = options.edge_type.clone();
        }
        if !next.animated {
            next.animated = options.animated;
        }
        if next.marker_start.is_none() {
            next.marker_start = options.marker_start.clone();
        }
        if next.marker_end.is_none() {
            next.marker_end = options.marker_end.clone();
        }
        if next.label.is_none() {
            next.label = options.label.clone();
        }
    }
    if connection_exists(&next, edges) {
        return None;
    }
    let mut result = edges.to_vec();
    result.push(next);
    Some(result)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReconnectEdgeOptions {
    pub should_replace_id: bool,
}

impl Default for ReconnectEdgeOptions {
    fn default() -> Self {
        ReconnectEdgeOptions {
            should_replace_id: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReconnectEdgeError {
    MissingSourceOrTarget,
    EdgeNotFound(String),
}

impl ReconnectEdgeError {
    pub fn code(&self) -> &'static str {
        match self {
            ReconnectEdgeError::MissingSourceOrTarget => "006",
            ReconnectEdgeError::EdgeNotFound(_) => "007",
        }
    }

    pub fn message(&self) -> String {
        match self {
            ReconnectEdgeError::MissingSourceOrTarget => crate::core::utils_extra::error_006(),
            ReconnectEdgeError::EdgeNotFound(id) => crate::core::utils_extra::error_007(id),
        }
    }
}

pub fn reconnect_edge<S: Clone>(
    old_edge: &Edge<S>,
    new_connection: &Connection,
    edges: &[Edge<S>],
    options: ReconnectEdgeOptions,
) -> Result<Vec<Edge<S>>, ReconnectEdgeError> {
    if new_connection.source.is_empty() || new_connection.target.is_empty() {
        return Err(ReconnectEdgeError::MissingSourceOrTarget);
    }

    if !edges.iter().any(|edge| edge.id == old_edge.id) {
        return Err(ReconnectEdgeError::EdgeNotFound(old_edge.id.clone()));
    }

    let mut next = old_edge.clone();
    next.id = if options.should_replace_id {
        get_edge_id(
            &new_connection.source,
            new_connection.source_handle.as_deref(),
            &new_connection.target,
            new_connection.target_handle.as_deref(),
        )
    } else {
        old_edge.id.clone()
    };
    next.source = new_connection.source.clone();
    next.target = new_connection.target.clone();
    next.source_handle = new_connection.source_handle.clone();
    next.target_handle = new_connection.target_handle.clone();

    let mut result: Vec<Edge<S>> = edges
        .iter()
        .filter(|edge| edge.id != old_edge.id)
        .cloned()
        .collect();
    result.push(next);
    Ok(result)
}
