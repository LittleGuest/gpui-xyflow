use std::collections::BTreeMap;

use crate::core::edge::{Connection, Edge};
use crate::core::graph::ConnectionMode;
use crate::core::node::Handle;

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
