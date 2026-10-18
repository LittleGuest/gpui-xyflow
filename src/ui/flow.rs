use crate::core::edge::Edge;
use crate::core::geometry::{Rect, XYPosition};
use crate::core::node::{HandleType, Node};
use crate::ui::background::BackgroundVariant;
use crate::ui::pane::{
    build_render_data, paint_background, paint_connection, paint_edges, paint_nodes,
    paint_selection,
};
use crate::ui::store::{FlowProps, FlowStore};
use gpui_kit::{
    canvas, div, App, Bounds, Context, FocusHandle, Focusable, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Render, ScrollDelta, ScrollWheelEvent,
    InteractiveElement, ParentElement, Styled, Window,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragMode {
    None,
    Pane,
    Node,
    Selection,
    Connection,
}

pub struct Flow {
    store: FlowStore<String>,
    focus_handle: FocusHandle,
    drag_mode: DragMode,
    last_pointer: XYPosition,
    pointer_down_at: Option<XYPosition>,
    pane_bounds: Option<Bounds<Pixels>>,
    pattern_variant: BackgroundVariant,
}

impl Flow {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Flow {
            store: FlowStore::new(FlowProps::default()),
            focus_handle: cx.focus_handle(),
            drag_mode: DragMode::None,
            last_pointer: XYPosition::new(0.0, 0.0),
            pointer_down_at: None,
            pane_bounds: None,
            pattern_variant: BackgroundVariant::Dots,
        }
    }

    pub fn with_props(props: FlowProps, cx: &mut Context<Self>) -> Self {
        Flow {
            store: FlowStore::new(props),
            focus_handle: cx.focus_handle(),
            drag_mode: DragMode::None,
            last_pointer: XYPosition::new(0.0, 0.0),
            pointer_down_at: None,
            pane_bounds: None,
            pattern_variant: BackgroundVariant::Dots,
        }
    }

    pub fn store(&self) -> &FlowStore<String> {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut FlowStore<String> {
        &mut self.store
    }

    pub fn set_nodes(&mut self, nodes: Vec<Node<String>>) {
        self.store.set_nodes(nodes);
        self.store.update_nodes();
    }

    pub fn set_edges(&mut self, edges: Vec<Edge<String>>) {
        self.store.set_edges(edges);
        self.store.update_nodes();
    }

    pub fn set_pattern(&mut self, variant: BackgroundVariant) {
        self.pattern_variant = variant;
    }

    pub fn set_theme(&mut self, theme: crate::ui::theme::FlowTheme) {
        self.store.props_mut().theme = theme;
    }

    pub fn fit_view(&mut self, window: &mut Window) {
        if let Some(bounds) = self.pane_bounds {
            self.store.set_size(
                f32::from(bounds.size.width) as f64,
                f32::from(bounds.size.height) as f64,
            );
            self.store.fit_view();
            window.request_animation_frame();
        }
    }

    fn local_position(&self, window_position: gpui_kit::Point<Pixels>) -> XYPosition {
        match self.pane_bounds {
            Some(bounds) => XYPosition::new(
                f32::from(window_position.x - bounds.origin.x) as f64,
                f32::from(window_position.y - bounds.origin.y) as f64,
            ),
            None => XYPosition::new(
                f32::from(window_position.x) as f64,
                f32::from(window_position.y) as f64,
            ),
        }
    }

    fn node_ids_at(&self, flow_point: XYPosition) -> Option<String> {
        let origin = self.store.props().node_origin;
        let mut hit: Option<(String, f64)> = None;
        for node in self.store.node_lookup().values() {
            if node.hidden {
                continue;
            }
            let dimensions = node.size();
            let position = node.internals.position_absolute;
            let rect = Rect::new(
                position.x - dimensions.width * origin[0],
                position.y - dimensions.height * origin[1],
                dimensions.width,
                dimensions.height,
            );
            let inside = flow_point.x >= rect.x
                && flow_point.x <= rect.x + rect.width
                && flow_point.y >= rect.y
                && flow_point.y <= rect.y + rect.height;
            if inside {
                let z = node.internals.z;
                match &hit {
                    Some((_, best_z)) if *best_z >= z => {}
                    _ => hit = Some((node.id.clone(), z)),
                }
            }
        }
        hit.map(|(id, _)| id)
    }

    fn handle_at(&self, flow_point: XYPosition) -> Option<(String, HandleType, Option<String>)> {
        let origin = self.store.props().node_origin;
        let radius = self.store.props().connection_radius / self.store.get_zoom();
        for node in self.store.node_lookup().values() {
            if node.hidden
                || !node
                    .internals
                    .user_node
                    .is_connectable(self.store.props().nodes_connectable)
            {
                continue;
            }
            let dimensions = node.size();
            let position = node.internals.position_absolute;
            let rect = Rect::new(
                position.x - dimensions.width * origin[0],
                position.y - dimensions.height * origin[1],
                dimensions.width,
                dimensions.height,
            );
            let handles = crate::core::store::collect_handles(&node.internals.user_node.handles);
            for handle in handles.0.iter().chain(handles.1.iter()) {
                let center_x = rect.x + rect.width * handle.x + handle.width / 2.0;
                let center_y = rect.y + rect.height * handle.y + handle.height / 2.0;
                let dx = flow_point.x - center_x;
                let dy = flow_point.y - center_y;
                if dx * dx + dy * dy <= radius * radius {
                    return Some((
                        node.id.clone(),
                        handle.handle_type,
                        handle.id.clone(),
                    ));
                }
            }
        }
        None
    }

    fn select_node(&mut self, node_id: &str, additive: bool) {
        if !additive {
            self.store.unselect_nodes_and_edges();
        }
        if let Some(node) = self.store.node_lookup().get(node_id) {
            let next = !node.selected || !additive;
            let id = node.id.clone();
            for node in self.store.node_lookup_mut().values_mut() {
                if node.id == id {
                    node.selected = next;
                    node.internals.user_node.selected = next;
                }
            }
            for node in self.store.nodes_mut().iter_mut() {
                if node.id == id {
                    node.selected = next;
                }
            }
            let _ = id;
        }
    }

    fn clear_selection(&mut self) {
        self.store.unselect_nodes_and_edges();
    }

    fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let local = self.local_position(event.position);
        let flow_point = self.store.screen_to_flow_position(local);
        self.last_pointer = flow_point;
        self.pointer_down_at = Some(local);

        if event.button == MouseButton::Left {
            if let Some((node_id, handle_type, handle_id)) = self.handle_at(flow_point) {
                self.store
                    .start_connection(&node_id, handle_type, handle_id.as_deref(), local);
                self.drag_mode = DragMode::Connection;
                cx.notify();
                return;
            }
            if let Some(node_id) = self.node_ids_at(flow_point) {
                let additive = event.modifiers.shift;
                self.select_node(&node_id, additive);
                self.store.start_drag_items(flow_point);
                self.drag_mode = DragMode::Node;
                cx.notify();
                return;
            }
            if event.modifiers.shift {
                let start = self.store.screen_to_flow_position(local);
                self.store.interaction_mut().selection_start = Some(start);
                self.store.interaction_mut().selection_end = Some(start);
                self.drag_mode = DragMode::Selection;
                cx.notify();
                return;
            }
            self.clear_selection();
            self.drag_mode = DragMode::Pane;
        } else {
            self.drag_mode = DragMode::Pane;
        }
        self.store.pan_zoom_mut().pointer_down(
            match event.button {
                MouseButton::Left => 0,
                MouseButton::Right => 2,
                MouseButton::Middle => 1,
                _ => 0,
            },
            local,
            event.modifiers.control,
        );
        cx.notify();
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let local = self.local_position(event.position);
        let flow_point = self.store.screen_to_flow_position(local);
        match self.drag_mode {
            DragMode::Node => {
                self.store.move_nodes_by(flow_point);
                self.last_pointer = flow_point;
                cx.notify();
            }
            DragMode::Pane => {
                self.store.pan_zoom_mut().pointer_move(local);
                self.store.sync_transform();
                self.last_pointer = flow_point;
                window.request_animation_frame();
                cx.notify();
            }
            DragMode::Connection => {
                self.store.update_connection(local);
                cx.notify();
            }
            DragMode::Selection => {
                self.store.interaction_mut().selection_end = Some(flow_point);
                cx.notify();
            }
            DragMode::None => {}
        }
    }

    fn handle_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.drag_mode {
            DragMode::Node => {
                self.store.end_drag();
            }
            DragMode::Pane => {
                self.store.pan_zoom_mut().pointer_up();
                self.store.sync_transform();
            }
            DragMode::Connection => {
                if let Some(line) = self.store.connection_line() {
                    if line.target_x != line.source_x || line.target_y != line.source_y {
                        let state = self.store.connection_state().clone();
                        if let crate::core::connect::ConnectionState::InProgress(in_progress) = state {
                            if let Some(target) = in_progress.to_node {
                                let target_handle = in_progress
                                    .to_handle
                                    .as_ref()
                                    .and_then(|handle| handle.id.clone());
                                let source_handle =
                                    in_progress.from_handle.id.clone();
                                let mut edge: Edge<String> =
                                    Edge::default();
                                edge.id = format!("{}-{}", in_progress.from_handle.node_id, target.id);
                                edge.source = in_progress.from_handle.node_id.clone();
                                edge.target = target.id.clone();
                                edge.source_handle = source_handle;
                                edge.target_handle = target_handle;
                                let mut edges = self.store.edges().to_vec();
                                edges.push(edge);
                                self.store.set_edges(edges);
                                self.store.update_nodes();
                            }
                        }
                    }
                }
                self.store.cancel_connection();
            }
            DragMode::Selection => {
                if let Some(rect) = self.store.interaction().selection_rect() {
                    let changes = self.store.selection_changes_for_rect(rect);
                    self.store.apply_node_changes(&changes);
                    self.store.update_nodes();
                }
                self.store.interaction_mut().selection_start = None;
                self.store.interaction_mut().selection_end = None;
            }
            DragMode::None => {}
        }
        self.drag_mode = DragMode::None;
        cx.notify();
    }

    fn handle_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let local = self.local_position(event.position);
        let (delta_y, delta_mode) = match event.delta {
            ScrollDelta::Pixels(delta) => (f32::from(delta.y) as f64, 1),
            ScrollDelta::Lines(delta) => (delta.y as f64, 0),
        };
        self.store.pan_zoom_mut().wheel(
            local,
            delta_y,
            delta_mode,
            event.modifiers.control,
            cfg!(target_os = "macos"),
            event.modifiers.shift,
        );
        self.store.sync_transform();
        window.request_animation_frame();
        cx.notify();
    }
}

impl Focusable for Flow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Flow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().clone();
        let theme = self.store.props().theme.clone();
        let pattern = self.pattern_variant;

        div()
            .size_full()
            .relative()
            .child(
                canvas(
                    move |bounds, _window, cx| {
                        let mut store = entity.read(cx).store.clone_snapshot();
                        let width = f32::from(bounds.size.width) as f64;
                        let height = f32::from(bounds.size.height) as f64;
                        if store.size().width != width || store.size().height != height {
                            store.set_size(width, height);
                            store.update_nodes();
                        }
                        let mut data = build_render_data(&store, bounds);
                        data.pattern = crate::ui::pane::background_pattern(&theme, pattern);
                        data
                    },
                    |_bounds, data, window, _cx| {
                        paint_background(window, &data);
                        paint_edges(window, &data);
                        paint_nodes(window, &data);
                        paint_connection(window, &data);
                        paint_selection(window, &data);
                    },
                )
                .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.handle_mouse_down(event, window, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.handle_mouse_down(event, window, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                this.handle_mouse_move(event, window, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, window, cx| {
                    this.handle_mouse_up(event, window, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, event: &MouseUpEvent, window, cx| {
                    this.handle_mouse_up(event, window, cx);
                }),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                this.handle_scroll(event, window, cx);
            }))
    }
}
