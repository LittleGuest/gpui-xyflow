use crate::core::connect::{
    get_closest_handle, is_valid_handle, start_connection, update_connection, ConnectionInProgress,
    ConnectionState, StartConnectionParams, UpdateConnectionParams, ValidHandleParams,
};
use crate::core::geometry::{Position, Transform, XYPosition};
use crate::core::graph::{get_handle_position, ConnectionMode as Mode};
use crate::core::node::{Handle, HandleType, InternalNode, NodeLookup};

pub fn get_handle<S: Clone>(
    node_id: &str,
    handle_type: HandleType,
    handle_id: Option<&str>,
    node_lookup: &NodeLookup<S>,
    connection_mode: Mode,
    with_absolute_position: bool,
) -> Option<Handle> {
    let node = node_lookup.get(node_id)?;
    let bounds = node.internals.handle_bounds.as_ref()?;

    let handles: Vec<&Handle> = match connection_mode {
        Mode::Strict => match handle_type {
            HandleType::Source => bounds.source.iter().collect(),
            HandleType::Target => bounds.target.iter().collect(),
        },
        Mode::Loose => bounds.all().collect(),
    };

    let mut handle = match handle_id {
        Some(id) => handles.into_iter().find(|handle| handle.id.as_deref() == Some(id)).cloned(),
        None => handles.first().map(|handle| (*handle).clone()),
    }?;

    if with_absolute_position {
        let point = get_handle_position(node, Some(&handle), handle.position, true);
        handle.x = point.x;
        handle.y = point.y;
    }

    Some(handle)
}

pub fn get_handle_type(
    edge_updater_type: Option<HandleType>,
    class_names: &[String],
) -> Option<HandleType> {
    if let Some(handle_type) = edge_updater_type {
        return Some(handle_type);
    }
    if class_names.iter().any(|class| class.contains("target")) {
        return Some(HandleType::Target);
    }
    if class_names.iter().any(|class| class.contains("source")) {
        return Some(HandleType::Source);
    }
    None
}

#[derive(Clone, Debug, PartialEq)]
pub struct XYHandleOptions {
    pub connection_mode: Mode,
    pub connection_radius: f64,
    pub drag_threshold: f64,
    pub auto_pan_on_connect: bool,
    pub auto_pan_speed: f64,
    pub lib: String,
}

impl Default for XYHandleOptions {
    fn default() -> Self {
        XYHandleOptions {
            connection_mode: Mode::Strict,
            connection_radius: 20.0,
            drag_threshold: 1.0,
            auto_pan_on_connect: true,
            auto_pan_speed: 15.0,
            lib: "gpui".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct XYHandleStartParams<'a> {
    pub node_id: &'a str,
    pub handle_id: Option<&'a str>,
    pub handle_type: HandleType,
    pub pointer: XYPosition,
    pub transform: Transform,
}

#[derive(Clone, Debug, PartialEq)]
pub enum XYHandlePhase {
    Idle,
    Dragging,
    ClickPending,
}

#[derive(Clone, Debug)]
pub struct XYHandle<S = String> {
    options: XYHandleOptions,
    connection: ConnectionState<S>,
    phase: XYHandlePhase,
    start_pointer: Option<XYPosition>,
    click_start_handle: Option<Handle>,
}

impl<S: Clone> XYHandle<S> {
    pub fn new(options: XYHandleOptions) -> Self {
        XYHandle {
            options,
            connection: ConnectionState::Idle(Default::default()),
            phase: XYHandlePhase::Idle,
            start_pointer: None,
            click_start_handle: None,
        }
    }

    pub fn options(&self) -> &XYHandleOptions {
        &self.options
    }

    pub fn set_options(&mut self, options: XYHandleOptions) {
        self.options = options;
    }

    pub fn connection(&self) -> &ConnectionState<S> {
        &self.connection
    }

    pub fn phase(&self) -> &XYHandlePhase {
        &self.phase
    }

    pub fn click_start_handle(&self) -> Option<&Handle> {
        self.click_start_handle.as_ref()
    }

    pub fn is_connecting(&self) -> bool {
        self.connection.is_in_progress()
    }

    pub fn on_pointer_down(
        &mut self,
        params: XYHandleStartParams<'_>,
        node_lookup: &NodeLookup<S>,
    ) -> bool {
        let next = start_connection(StartConnectionParams {
            node_id: params.node_id,
            handle_id: params.handle_id,
            handle_type: params.handle_type,
            pointer: params.pointer,
            transform: params.transform,
            connection_mode: self.options.connection_mode,
            connection_radius: self.options.connection_radius,
            node_lookup,
        });
        match next {
            Some(state) => {
                self.connection = ConnectionState::InProgress(state);
                self.phase = XYHandlePhase::Dragging;
                self.start_pointer = Some(params.pointer);
                self.click_start_handle = self.connection.from_handle().cloned();
                true
            }
            None => false,
        }
    }

    pub fn on_pointer_move(
        &mut self,
        pointer: XYPosition,
        transform: Transform,
        node_lookup: &NodeLookup<S>,
        is_valid_connection: &dyn Fn(&crate::core::edge::Connection) -> bool,
    ) -> Option<ConnectionInProgress<S>> {
        let ConnectionState::InProgress(state) = &self.connection else {
            return None;
        };

        if let Some(start) = self.start_pointer {
            let distance = ((pointer.x - start.x).powi(2) + (pointer.y - start.y).powi(2)).sqrt();
            if distance < self.options.drag_threshold && self.phase == XYHandlePhase::ClickPending {
                return None;
            }
        }

        let next = update_connection(UpdateConnectionParams {
            state,
            pointer,
            transform,
            connection_mode: self.options.connection_mode,
            connection_radius: self.options.connection_radius,
            is_valid_connection,
            node_lookup,
        });
        let next = ConnectionState::InProgress(next);
        self.connection = next;
        self.phase = XYHandlePhase::Dragging;
        match &self.connection {
            ConnectionState::InProgress(state) => Some(state.clone()),
            ConnectionState::Idle(_) => None,
        }
    }

    pub fn on_pointer_up(&mut self) -> ConnectionState<S> {
        let state = std::mem::replace(
            &mut self.connection,
            ConnectionState::Idle(Default::default()),
        );
        self.phase = XYHandlePhase::Idle;
        self.start_pointer = None;
        state
    }

    pub fn cancel(&mut self) {
        self.connection = ConnectionState::Idle(Default::default());
        self.phase = XYHandlePhase::Idle;
        self.start_pointer = None;
        self.click_start_handle = None;
    }

    pub fn begin_click_connection(&mut self, handle: Handle) {
        self.click_start_handle = Some(handle);
        self.phase = XYHandlePhase::ClickPending;
    }

    pub fn clear_click_connection(&mut self) {
        self.click_start_handle = None;
        if self.phase == XYHandlePhase::ClickPending {
            self.phase = XYHandlePhase::Idle;
        }
    }

    pub fn valid_handle_at(
        &self,
        pointer: XYPosition,
        transform: Transform,
        node_lookup: &NodeLookup<S>,
        is_valid_connection: &dyn Fn(&crate::core::edge::Connection) -> bool,
    ) -> Option<Handle> {
        let Some(from) = self.click_start_handle.as_ref() else {
            return None;
        };
        let logical = transform.invert(pointer);
        let closest = get_closest_handle(
            logical,
            self.options.connection_radius,
            node_lookup,
            from,
        )?;
        let result = is_valid_handle(ValidHandleParams {
            handle: Some(&closest),
            connection_mode: self.options.connection_mode,
            from_node_id: &from.node_id,
            from_handle_id: from.id.as_deref(),
            from_type: from.handle_type,
            is_valid_connection,
            node_lookup,
        });
        if result.is_valid {
            Some(closest)
        } else {
            None
        }
    }
}

pub fn handle_for_node<S: Clone>(
    node: &InternalNode<S>,
    handle_type: HandleType,
    handle_id: Option<&str>,
) -> Option<Handle> {
    let bounds = node.internals.handle_bounds.as_ref()?;
    let handles: Vec<&Handle> = match handle_type {
        HandleType::Source => bounds.source.iter().collect(),
        HandleType::Target => bounds.target.iter().collect(),
    };
    match handle_id {
        Some(id) => handles.into_iter().find(|h| h.id.as_deref() == Some(id)).cloned(),
        None => handles.first().map(|h| (*h).clone()),
    }
}

pub fn handle_position_of<S: Clone>(
    node: &InternalNode<S>,
    handle: &Handle,
) -> XYPosition {
    get_handle_position(node, Some(handle), handle.position, true)
}

pub fn opposite_position(position: Position) -> Position {
    position.opposite()
}
