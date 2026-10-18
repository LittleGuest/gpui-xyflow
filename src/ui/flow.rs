use gpui_kit::{
    App, Bounds, Context, FocusHandle, Focusable, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render, ScrollDelta,
    ScrollWheelEvent, Styled, Window, canvas, div,
};

use crate::{
    core::{
        drag::{CalculateNodePositionParams, XYDrag, XYDragOptions, calculate_node_position},
        edge::Edge,
        geometry::{Rect, XYPosition},
        node::{HandleType, Node},
        utils::snap_position,
        xypanzoom::{PointerEvent, PointerEventKind, XYPanZoom},
    },
    ui::{
        background::BackgroundVariant,
        container::{RendererKeyState, derive_renderer_state, zoom_pane_update_options},
        pane::{
            build_render_data, paint_background, paint_connection, paint_edges, paint_nodes,
            paint_selection,
        },
        store::{FlowProps, FlowStore},
    },
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
    pan_zoom: XYPanZoom,
    drag: XYDrag,
    focus_handle: FocusHandle,
    drag_mode: DragMode,
    last_pointer: XYPosition,
    pointer_down_at: Option<XYPosition>,
    pane_bounds: Option<Bounds<Pixels>>,
    pattern_variant: BackgroundVariant,
}

impl Flow {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let store = FlowStore::new(FlowProps::default());
        let pan_zoom = XYPanZoom::new(
            store.viewport(),
            store.props().pan_zoom_options(),
            store.props().translate_extent,
        );
        Flow {
            store,
            pan_zoom,
            drag: XYDrag::default(),
            focus_handle: cx.focus_handle(),
            drag_mode: DragMode::None,
            last_pointer: XYPosition::new(0.0, 0.0),
            pointer_down_at: None,
            pane_bounds: None,
            pattern_variant: BackgroundVariant::Dots,
        }
    }

    pub fn with_props(props: FlowProps, cx: &mut Context<Self>) -> Self {
        let store = FlowStore::new(props);
        let pan_zoom = XYPanZoom::new(
            store.viewport(),
            store.props().pan_zoom_options(),
            store.props().translate_extent,
        );
        Flow {
            store,
            pan_zoom,
            drag: XYDrag::default(),
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
                    return Some((node.id.clone(), handle.handle_type, handle.id.clone()));
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

    fn sync_pan_zoom_options(&mut self) {
        let keys = RendererKeyState::default();
        let derived = derive_renderer_state(self.store.props(), keys, false);
        let options = zoom_pane_update_options(
            self.store.props(),
            derived,
            keys,
            self.store.props().user_selection_active,
            self.store.connection_state().is_in_progress(),
        );
        self.pan_zoom.update(&options);
    }

    fn pointer_event(
        &self,
        kind: PointerEventKind,
        button: u32,
        ctrl: bool,
        shift: bool,
    ) -> PointerEvent {
        PointerEvent {
            kind,
            button,
            ctrl_key: ctrl,
            shift_key: shift,
            ..PointerEvent::default()
        }
    }

    fn sync_drag_options(&mut self) {
        let props = self.store.props();
        let options = XYDragOptions {
            node_drag_threshold: props.node_drag_threshold,
            no_drag_class_name: None,
            handle_selector: None,
            auto_pan_on_node_drag: props.auto_pan_on_node_drag,
            auto_pan_speed: props.auto_pan_speed,
            snap_to_grid: props.snap_to_grid,
            snap_grid: props.snap_grid,
            nodes_draggable: props.nodes_draggable,
        };
        self.drag.set_options(options);
    }

    fn apply_drag_update(
        &mut self,
        positions: &std::collections::BTreeMap<String, crate::core::drag::NodePositionUpdate>,
    ) {
        for (id, update) in positions {
            if let Some(node) = self.store.node_lookup_mut().get_mut(id) {
                node.position = update.position;
                node.internals.position_absolute = update.position_absolute;
                node.dragging = true;
                node.internals.user_node.position = update.position;
                node.internals.user_node.dragging = true;
            }
            if let Some(user_node) = self.store.nodes_mut().iter_mut().find(|n| n.id == *id) {
                user_node.position = update.position;
                user_node.dragging = true;
            }
        }
    }

    fn finish_drag(&mut self, dragged: bool, items: Vec<crate::core::node::NodeDragItem>) {
        if dragged {
            self.store.update_node_positions(&items, false);
        }
        for node in self.store.node_lookup_mut().values_mut() {
            node.dragging = false;
            node.internals.user_node.dragging = false;
        }
        for node in self.store.nodes_mut().iter_mut() {
            node.dragging = false;
        }
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
                self.sync_drag_options();
                let lookup = self.store.node_lookup().clone();
                self.drag
                    .on_pointer_down(flow_point, Some(&node_id), &lookup);
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

        let button = match event.button {
            MouseButton::Left => 0,
            MouseButton::Right => 2,
            MouseButton::Middle => 1,
            _ => 0,
        };
        let pointer = self.pointer_event(
            PointerEventKind::MouseDown,
            button,
            event.modifiers.control,
            event.modifiers.shift,
        );
        self.sync_pan_zoom_options();
        if self.drag_mode == DragMode::Pane && self.pan_zoom.accepts(&pointer) {
            self.store
                .pan_zoom_mut()
                .pointer_down(button, local, event.modifiers.control);
        } else if self.drag_mode == DragMode::Pane {
            self.drag_mode = DragMode::None;
        }
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
                let node_origin = self.store.props().node_origin;
                let node_extent = self.store.props().node_extent;
                let snap_grid = self.store.props().snap_grid;
                let snap_to_grid = self.store.props().snap_to_grid;
                let lookup = self.store.node_lookup().clone();
                let pointer = flow_point;
                let update = self
                    .drag
                    .on_pointer_move(pointer, &lookup, move |item, lookup| {
                        let mut next = XYPosition::new(
                            pointer.x - item.distance.x,
                            pointer.y - item.distance.y,
                        );
                        if snap_to_grid {
                            next = snap_position(next, snap_grid);
                        }
                        if let Some(result) = calculate_node_position(CalculateNodePositionParams {
                            node_id: &item.id,
                            next_position: next,
                            node_lookup: lookup,
                            node_origin,
                            node_extent,
                        }) {
                            item.position = result.position;
                            item.position_absolute = result.position_absolute;
                        }
                    });
                if update.changed {
                    self.apply_drag_update(&update.positions);
                }
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
                let (dragged, items) = self.drag.on_pointer_up();
                self.finish_drag(dragged, items);
            }
            DragMode::Pane => {
                self.store.pan_zoom_mut().pointer_up();
                self.store.sync_transform();
            }
            DragMode::Connection => {
                if let Some(line) = self.store.connection_line()
                    && (line.target_x != line.source_x || line.target_y != line.source_y)
                {
                    let state = self.store.connection_state().clone();
                    if let crate::core::connect::ConnectionState::InProgress(in_progress) = state
                        && let Some(target) = in_progress.to_node
                    {
                        let target_handle = in_progress
                            .to_handle
                            .as_ref()
                            .and_then(|handle| handle.id.clone());
                        let source_handle = in_progress.from_handle.id.clone();
                        let edge: Edge<String> = Edge {
                            id: format!("{}-{}", in_progress.from_handle.node_id, target.id),
                            source: in_progress.from_handle.node_id.clone(),
                            target: target.id.clone(),
                            source_handle,
                            target_handle,
                            ..Edge::default()
                        };
                        let mut edges = self.store.edges().to_vec();
                        edges.push(edge);
                        self.store.set_edges(edges);
                        self.store.update_nodes();
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
        let (delta_y, delta_mode) = match event.delta {
            ScrollDelta::Pixels(delta) => (f32::from(delta.y) as f64, 1),
            ScrollDelta::Lines(delta) => (delta.y as f64, 0),
        };

        self.sync_pan_zoom_options();
        self.pan_zoom.sync_viewport(self.store.viewport());

        let pointer = PointerEvent {
            kind: PointerEventKind::Wheel,
            delta_y,
            delta_mode,
            ctrl_key: event.modifiers.control,
            shift_key: event.modifiers.shift,
            ..PointerEvent::default()
        };
        let now_ms = 0.0;
        let outcome = self.pan_zoom.wheel(&pointer, now_ms);

        if let Some(transform) = outcome.transform {
            self.store.set_viewport(transform.viewport());
        }
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
