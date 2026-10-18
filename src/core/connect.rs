use crate::core::{
    constants::{ADDITIONAL_DISTANCE, DEFAULT_CONNECTION_RADIUS},
    edge::Connection,
    geometry::{Position, Rect, Transform, XYPosition},
    graph::{ConnectionMode, get_handle_by_id, get_handle_position},
    node::{Handle, HandleType, InternalNode, NodeLookup},
    utils::{get_rects_overlapping_area, renderer_point_to_point},
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct NoConnection {
    pub in_progress: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionInProgress<S = String> {
    pub is_valid: Option<bool>,
    pub from: XYPosition,
    pub from_handle: Handle,
    pub from_position: Position,
    pub from_node: InternalNode<S>,
    pub to: XYPosition,
    pub to_handle: Option<Handle>,
    pub to_position: Position,
    pub to_node: Option<InternalNode<S>>,
    pub pointer: XYPosition,
}

impl<S: Clone> ConnectionInProgress<S> {
    pub fn to_position_or_opposite(&self) -> Position {
        match &self.to_handle {
            Some(handle) => handle.position,
            None => self.from_handle.position.opposite(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionState<S = String> {
    Idle(NoConnection),
    InProgress(ConnectionInProgress<S>),
}

impl<S: Clone> Default for ConnectionState<S> {
    fn default() -> Self {
        ConnectionState::Idle(NoConnection::default())
    }
}

impl<S: Clone> ConnectionState<S> {
    pub fn is_in_progress(&self) -> bool {
        matches!(self, ConnectionState::InProgress(_))
    }

    pub fn from_node(&self) -> Option<&InternalNode<S>> {
        match self {
            ConnectionState::InProgress(state) => Some(&state.from_node),
            ConnectionState::Idle(_) => None,
        }
    }

    pub fn from_handle(&self) -> Option<&Handle> {
        match self {
            ConnectionState::InProgress(state) => Some(&state.from_handle),
            ConnectionState::Idle(_) => None,
        }
    }

    pub fn to_handle(&self) -> Option<&Handle> {
        match self {
            ConnectionState::InProgress(state) => state.to_handle.as_ref(),
            ConnectionState::Idle(_) => None,
        }
    }

    pub fn is_valid(&self) -> Option<bool> {
        match self {
            ConnectionState::InProgress(state) => state.is_valid,
            ConnectionState::Idle(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionResult {
    pub is_valid: bool,
    pub connection: Option<Connection>,
    pub to_handle: Option<Handle>,
}

impl ConnectionResult {
    pub fn invalid() -> Self {
        ConnectionResult {
            is_valid: false,
            connection: None,
            to_handle: None,
        }
    }
}

pub fn is_connection_valid(
    is_inside_connection_radius: bool,
    is_handle_valid: bool,
) -> Option<bool> {
    if is_handle_valid {
        Some(true)
    } else if is_inside_connection_radius {
        Some(false)
    } else {
        None
    }
}

fn nodes_within_distance<S: Clone>(
    position: XYPosition,
    node_lookup: &NodeLookup<S>,
    distance: f64,
) -> Vec<&InternalNode<S>> {
    let rect = Rect::new(
        position.x - distance,
        position.y - distance,
        distance * 2.0,
        distance * 2.0,
    );

    node_lookup
        .values()
        .filter(|node| {
            let dimensions = node.size();
            let node_rect = Rect::new(
                node.internals.position_absolute.x,
                node.internals.position_absolute.y,
                dimensions.width,
                dimensions.height,
            );
            get_rects_overlapping_area(rect, node_rect) > 0.0
        })
        .collect()
}

pub fn get_closest_handle<S: Clone>(
    position: XYPosition,
    connection_radius: f64,
    node_lookup: &NodeLookup<S>,
    from_handle: &Handle,
) -> Option<Handle> {
    let mut closest: Vec<Handle> = Vec::new();
    let mut min_distance = f64::INFINITY;

    let close_nodes = nodes_within_distance(
        position,
        node_lookup,
        connection_radius + ADDITIONAL_DISTANCE,
    );

    for node in close_nodes {
        let Some(bounds) = node.internals.handle_bounds.as_ref() else {
            continue;
        };
        for handle in bounds.all() {
            if from_handle.node_id == handle.node_id
                && from_handle.handle_type == handle.handle_type
                && from_handle.id == handle.id
            {
                continue;
            }

            let point = get_handle_position(node, Some(handle), handle.position, true);
            let distance = point.distance_to(position);
            if distance > connection_radius {
                continue;
            }

            let mut candidate = handle.clone();
            candidate.x = point.x;
            candidate.y = point.y;

            if distance < min_distance {
                closest = vec![candidate];
                min_distance = distance;
            } else if distance == min_distance {
                closest.push(candidate);
            }
        }
    }

    if closest.is_empty() {
        return None;
    }

    if closest.len() > 1 {
        let opposite = from_handle.handle_type.opposite();
        return closest
            .iter()
            .find(|handle| handle.handle_type == opposite)
            .cloned()
            .or_else(|| closest.first().cloned());
    }

    closest.into_iter().next()
}

pub struct ValidHandleParams<'a, S> {
    pub handle: Option<&'a Handle>,
    pub connection_mode: ConnectionMode,
    pub from_node_id: &'a str,
    pub from_handle_id: Option<&'a str>,
    pub from_type: HandleType,
    pub is_valid_connection: &'a dyn Fn(&Connection) -> bool,
    pub node_lookup: &'a NodeLookup<S>,
}

pub fn is_valid_handle<S: Clone>(params: ValidHandleParams<'_, S>) -> ConnectionResult {
    let is_target = params.from_type == HandleType::Target;

    let Some(handle) = params.handle else {
        return ConnectionResult::invalid();
    };

    let handle_node_id = handle.node_id.as_str();
    let handle_id = handle.id.as_deref();

    let connection = if is_target {
        Connection {
            source: handle_node_id.to_string(),
            source_handle: handle_id.map(str::to_string),
            target: params.from_node_id.to_string(),
            target_handle: params.from_handle_id.map(str::to_string),
        }
    } else {
        Connection {
            source: params.from_node_id.to_string(),
            source_handle: params.from_handle_id.map(str::to_string),
            target: handle_node_id.to_string(),
            target_handle: handle_id.map(str::to_string),
        }
    };

    let handle_type_matches = match params.connection_mode {
        ConnectionMode::Strict => {
            (is_target && handle.handle_type == HandleType::Source)
                || (!is_target && handle.handle_type == HandleType::Target)
        }
        ConnectionMode::Loose => {
            handle_node_id != params.from_node_id || handle_id != params.from_handle_id
        }
    };

    let is_valid = handle_type_matches && (params.is_valid_connection)(&connection);

    let to_handle = get_handle_by_id_raw(
        handle_node_id,
        handle.handle_type,
        handle_id,
        params.node_lookup,
        params.connection_mode,
    );

    ConnectionResult {
        is_valid,
        connection: if is_valid { Some(connection) } else { None },
        to_handle,
    }
}

fn get_handle_by_id_raw<S: Clone>(
    node_id: &str,
    handle_type: HandleType,
    handle_id: Option<&str>,
    node_lookup: &NodeLookup<S>,
    connection_mode: ConnectionMode,
) -> Option<Handle> {
    let node = node_lookup.get(node_id)?;
    let handle = get_handle_by_id(node, handle_type, handle_id, connection_mode)?;
    let point = get_handle_position(node, Some(&handle), handle.position, true);
    let mut handle = handle;
    handle.x = point.x;
    handle.y = point.y;
    Some(handle)
}

pub struct StartConnectionParams<'a, S> {
    pub node_id: &'a str,
    pub handle_id: Option<&'a str>,
    pub handle_type: HandleType,
    pub pointer: XYPosition,
    pub transform: Transform,
    pub connection_mode: ConnectionMode,
    pub connection_radius: f64,
    pub node_lookup: &'a NodeLookup<S>,
}

pub fn start_connection<S: Clone>(
    params: StartConnectionParams<'_, S>,
) -> Option<ConnectionInProgress<S>> {
    let node = params.node_lookup.get(params.node_id)?;
    let handle = get_handle_by_id(
        node,
        params.handle_type,
        params.handle_id,
        params.connection_mode,
    )?;
    let from = get_handle_position(node, Some(&handle), Position::Left, true);

    Some(ConnectionInProgress {
        is_valid: None,
        from,
        from_handle: handle.clone(),
        from_position: handle.position,
        from_node: node.clone(),
        to: params.pointer,
        to_handle: None,
        to_position: handle.position.opposite(),
        to_node: None,
        pointer: params.pointer,
    })
}

pub struct UpdateConnectionParams<'a, S> {
    pub state: &'a ConnectionInProgress<S>,
    pub pointer: XYPosition,
    pub transform: Transform,
    pub connection_mode: ConnectionMode,
    pub connection_radius: f64,
    pub is_valid_connection: &'a dyn Fn(&Connection) -> bool,
    pub node_lookup: &'a NodeLookup<S>,
}

pub fn update_connection<S: Clone>(
    params: UpdateConnectionParams<'_, S>,
) -> ConnectionInProgress<S> {
    let renderer_pointer = params.transform.invert(params.pointer);
    let closest = get_closest_handle(
        renderer_pointer,
        params.connection_radius,
        params.node_lookup,
        &params.state.from_handle,
    );

    let result = is_valid_handle(ValidHandleParams {
        handle: closest.as_ref(),
        connection_mode: params.connection_mode,
        from_node_id: &params.state.from_handle.node_id,
        from_handle_id: params.state.from_handle.id.as_deref(),
        from_type: params.state.from_handle.handle_type,
        is_valid_connection: params.is_valid_connection,
        node_lookup: params.node_lookup,
    });

    let is_valid = is_connection_valid(closest.is_some(), result.is_valid);

    let from = params
        .node_lookup
        .get(&params.state.from_handle.node_id)
        .map(|node| {
            get_handle_position(node, Some(&params.state.from_handle), Position::Left, true)
        })
        .unwrap_or(params.state.from);

    let to = match (&result.to_handle, is_valid) {
        (Some(handle), Some(true)) => {
            renderer_point_to_point(XYPosition::new(handle.x, handle.y), params.transform)
        }
        _ => params.pointer,
    };

    let to_position = match (&result.to_handle, is_valid) {
        (Some(handle), Some(true)) => handle.position,
        _ => params.state.from_handle.position.opposite(),
    };

    let to_node = result
        .to_handle
        .as_ref()
        .and_then(|handle| params.node_lookup.get(&handle.node_id).cloned());

    ConnectionInProgress {
        is_valid,
        from,
        from_handle: params.state.from_handle.clone(),
        from_position: params.state.from_position,
        from_node: params.state.from_node.clone(),
        to,
        to_handle: result.to_handle,
        to_position,
        to_node,
        pointer: params.pointer,
    }
}

pub struct CheckHandleParams<'a, S> {
    pub node_id: &'a str,
    pub handle_id: Option<&'a str>,
    pub handle_type: HandleType,
    pub from_node_id: &'a str,
    pub from_handle_id: Option<&'a str>,
    pub from_type: HandleType,
    pub connection_mode: ConnectionMode,
    pub is_valid_connection: &'a dyn Fn(&Connection) -> bool,
    pub node_lookup: &'a NodeLookup<S>,
}

pub fn check_click_connection<S: Clone>(params: CheckHandleParams<'_, S>) -> ConnectionResult {
    let handle = params.node_lookup.get(params.node_id).and_then(|node| {
        get_handle_by_id(
            node,
            params.handle_type,
            params.handle_id,
            params.connection_mode,
        )
    });

    let handle = handle.map(|mut handle| {
        if let Some(node) = params.node_lookup.get(params.node_id) {
            let point = get_handle_position(node, Some(&handle), handle.position, true);
            handle.x = point.x;
            handle.y = point.y;
        }
        handle
    });

    is_valid_handle(ValidHandleParams {
        handle: handle.as_ref(),
        connection_mode: params.connection_mode,
        from_node_id: params.from_node_id,
        from_handle_id: params.from_handle_id,
        from_type: params.from_type,
        is_valid_connection: params.is_valid_connection,
        node_lookup: params.node_lookup,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionLine {
    pub source_x: f64,
    pub source_y: f64,
    pub source_position: Position,
    pub target_x: f64,
    pub target_y: f64,
    pub target_position: Position,
}

pub fn connection_line(state: &ConnectionState) -> Option<ConnectionLine> {
    match state {
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

pub fn default_connection_radius() -> f64 {
    DEFAULT_CONNECTION_RADIUS
}

pub fn build_connections(node_count: usize) -> usize {
    node_count
}
