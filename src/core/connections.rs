use std::collections::BTreeMap;

use crate::core::{
    edge::{Connection, Edge},
    graph::ConnectionMode,
    node::Handle,
};

pub type ConnectionLookup = BTreeMap<String, Connection>;

pub fn get_connection_key(connection: &Connection) -> String {
    let source_handle = connection.source_handle.clone().unwrap_or_default();
    let target_handle = connection.target_handle.clone().unwrap_or_default();
    format!(
        "{}__{}__{}__{}",
        connection.source, source_handle, connection.target, target_handle
    )
}

pub fn add_connection_to_lookup(lookup: &mut ConnectionLookup, connection: Connection) {
    lookup.insert(get_connection_key(&connection), connection);
}

pub fn remove_connection_from_lookup(lookup: &mut ConnectionLookup, connection: &Connection) {
    lookup.remove(&get_connection_key(connection));
}

pub fn are_connection_maps_equal(a: &ConnectionLookup, b: &ConnectionLookup) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .all(|(key, value)| b.get(key).map(|other| other == value).unwrap_or(false))
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
