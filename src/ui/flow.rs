use std::{cell::Cell, rc::Rc, time::Instant};

use gpui_kit::{
    AnyElement, App, Bounds, ClickEvent, Context, Div, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, KeyUpEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, Render, ScrollDelta, ScrollWheelEvent,
    StatefulInteractiveElement, Styled, Window, canvas, div, point, px, relative,
};

use crate::{
    core::{
        change::NodeChange,
        connect::{ConnectionInProgress, ConnectionState},
        connections::{ReconnectEdgeOptions, add_edge, connection_exists, reconnect_edge},
        drag::{
            CalculateNodePositionParams, XYDrag, XYDragOptions, auto_pan_for_drag,
            calculate_node_position,
        },
        edge::{Connection, Edge},
        geometry::{
            Alignment, Dimensions, PanelPosition, Position, Rect, Transform, Viewport, XYPosition,
        },
        graph::get_elements_to_remove,
        node::{Handle, HandleType, Node},
        utils::snap_position,
        utils_extra::{ToolbarAlignX, ToolbarAlignY},
        xyhandle::{XYHandle, XYHandleOptions, XYHandleStartParams},
        xypanzoom::{PointerEvent, PointerEventKind, XYPanZoom},
        xyresizer::{ResizeParams, XYResizer, XYResizerOptions},
    },
    ui::{
        background::{BackgroundPattern, BackgroundVariant},
        components::{arrow_key_diff, attribution},
        container::{RendererKeyState, derive_renderer_state, zoom_pane_update_options},
        controls::{ControlButton, ControlsOrientation, ControlsProps},
        edge_renderer::edge_at_point,
        handle::handle_fill,
        minimap::{MinimapCallbacks, MinimapProps, minimap_origin, minimap_render},
        node_renderer::{NodeBox, node_border, node_fill, node_screen_rect, node_text_color},
        pane::{
            EdgeContentRenderer, NodeContentRenderer, ResizeControlPaint, build_render_data,
            paint_background, paint_connection, paint_edges, paint_focus_ring, paint_nodes,
            paint_reconnect_handles, paint_resize_controls, paint_selection,
        },
        paths::parse_svg_path,
        resizer::{
            NodeResizeInfo, NodeResizerProps, find_resize_control, resize_change_to_node_changes,
            resize_controls,
        },
        store::{FlowProps, FlowStore},
        toolbars::{
            EdgeToolbarContent, EdgeToolbarProps, NodeToolbarContent, NodeToolbarProps,
            edge_toolbar, node_toolbar_targets,
        },
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragMode {
    None,
    Pane,
    Node,
    Selection,
    Connection,
    Reconnect,
    Resize,
    Minimap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReconnectEndpoint {
    Source,
    Target,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReconnectDrag {
    edge_id: String,
    endpoint: ReconnectEndpoint,
}

/// Handle that started a click-based connection (connectOnClick).
#[derive(Clone, Debug, PartialEq, Eq)]
struct ClickStartHandle {
    node_id: String,
    handle_id: Option<String>,
    handle_type: HandleType,
}

fn key_matches(config: Option<&str>, key: &str) -> bool {
    let Some(config) = config else {
        return false;
    };
    let config = config.to_lowercase();
    match config.as_str() {
        "ctrl" | "control" => matches!(key, "control" | "ctrl"),
        "cmd" | "command" | "meta" | "super" => matches!(key, "meta" | "cmd" | "command" | "super"),
        "alt" | "option" => matches!(key, "alt" | "option"),
        "shift" => key == "shift",
        "space" | "spacebar" => matches!(key, "space" | "spacebar" | " "),
        other => other == key,
    }
}

fn connection_from_progress(state: &ConnectionInProgress<String>) -> Option<Connection> {
    let to_node = state.to_node.as_ref()?;
    let to_handle = state
        .to_handle
        .as_ref()
        .and_then(|handle| handle.id.clone());
    if state.from_handle.handle_type == HandleType::Target {
        Some(Connection {
            source: to_node.id.clone(),
            source_handle: to_handle,
            target: state.from_handle.node_id.clone(),
            target_handle: state.from_handle.id.clone(),
        })
    } else {
        Some(Connection {
            source: state.from_handle.node_id.clone(),
            source_handle: state.from_handle.id.clone(),
            target: to_node.id.clone(),
            target_handle: to_handle,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewportCallbackKind {
    Start,
    Move,
    End,
}

/// Builds an element inside the flow coordinate space.
pub type ViewportPortalContent =
    std::sync::Arc<dyn Fn(&mut Window, &mut App) -> AnyElement + Send + Sync>;

/// One element anchored at a flow coordinate inside the viewport portal.
#[derive(Clone)]
pub struct ViewportPortalItem {
    pub x: f64,
    pub y: f64,
    pub content: ViewportPortalContent,
}

/// Builds content for a fixed panel inside the flow container.
pub type PanelContent = std::sync::Arc<dyn Fn(&mut Window, &mut App) -> AnyElement + Send + Sync>;

/// A panel pinned to one of the flow container edges.
#[derive(Clone)]
pub struct PanelItem {
    pub id: String,
    pub position: PanelPosition,
    pub margin: f64,
    pub offset: f64,
    pub content: PanelContent,
}

impl PanelItem {
    pub fn new(id: impl Into<String>, position: PanelPosition, content: PanelContent) -> PanelItem {
        PanelItem {
            id: id.into(),
            position,
            margin: 15.0,
            offset: 0.0,
            content,
        }
    }

    pub fn with_margin(mut self, margin: f64) -> Self {
        self.margin = margin;
        self
    }

    pub fn with_offset(mut self, offset: f64) -> Self {
        self.offset = offset;
        self
    }
}

/// Builds an element anchored at an edge's label position.
pub type EdgeLabelContent =
    std::sync::Arc<dyn Fn(&Edge<String>, &mut Window, &mut App) -> AnyElement + Send + Sync>;

/// Builds a real GPUI element rendered on top of a node, aligned to its screen rect.
pub type NodeElementRenderer = std::sync::Arc<
    dyn Fn(&Node<String>, Bounds<Pixels>, f64, &mut Window, &mut App) -> AnyElement + Send + Sync,
>;

/// Builds a real GPUI element rendered on top of a handle, aligned to its screen rect.
///
/// 该元素只做视觉叠加，不拦截画布的连接命中测试。
pub type HandleRenderer = std::sync::Arc<
    dyn Fn(&Node<String>, &Handle, Bounds<Pixels>, f64, &mut Window, &mut App) -> AnyElement
        + Send
        + Sync,
>;

/// Builds a custom minimap node shape.
pub type MinimapNodeComponent =
    std::sync::Arc<dyn Fn(&Node<String>, bool, &mut Window, &mut App) -> AnyElement + Send + Sync>;

/// Builds a real GPUI element rendered on top of an edge, aligned to its screen bounding box.
pub type EdgeElementRenderer = std::sync::Arc<
    dyn Fn(&Edge<String>, Bounds<Pixels>, Transform, &mut Window, &mut App) -> AnyElement
        + Send
        + Sync,
>;

/// Snapshot of an in-progress connection, passed to a custom connection line component.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionLineProps {
    /// Connection start in flow coordinates.
    pub from_x: f64,
    pub from_y: f64,
    /// Connection end (pointer or hovered handle) in flow coordinates.
    pub to_x: f64,
    pub to_y: f64,
    pub from_position: Position,
    pub to_position: Position,
    /// `Some(true/false)` while connecting over a valid/invalid target.
    pub connection_status: Option<bool>,
    pub from_node: Node<String>,
    pub from_handle: Handle,
    pub to_node: Option<Node<String>>,
    pub to_handle: Option<Handle>,
}

impl ConnectionLineProps {
    pub fn from_progress(state: &ConnectionInProgress<String>) -> ConnectionLineProps {
        ConnectionLineProps {
            from_x: state.from.x,
            from_y: state.from.y,
            to_x: state.to.x,
            to_y: state.to.y,
            from_position: state.from_position,
            to_position: state.to_position_or_opposite(),
            connection_status: state.is_valid,
            from_node: state.from_node.internals.user_node.clone(),
            from_handle: state.from_handle.clone(),
            to_node: state
                .to_node
                .as_ref()
                .map(|node| node.internals.user_node.clone()),
            to_handle: state.to_handle.clone(),
        }
    }
}

/// Builds a real GPUI element that replaces the default connection line.
///
/// `bounds` 是整个画布的屏幕矩形，`transform` 用于把 flow 坐标换算成屏幕坐标。
pub type ConnectionLineRenderer = std::sync::Arc<
    dyn Fn(&ConnectionLineProps, Bounds<Pixels>, Transform, &mut Window, &mut App) -> AnyElement
        + Send
        + Sync,
>;

#[derive(Clone, Debug, PartialEq, Eq)]
enum FocusTarget {
    Node(String),
    Edge(String),
}

fn next_focus_index(current: Option<usize>, len: usize, reverse: bool) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(match (current, reverse) {
        (Some(index), false) => (index + 1) % len,
        (Some(index), true) => (index + len - 1) % len,
        (None, false) => 0,
        (None, true) => len - 1,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PressedTarget {
    Node(String),
    Edge(String),
    Pane,
}

pub struct Flow {
    store: FlowStore<String>,
    pan_zoom: XYPanZoom,
    drag: XYDrag,
    handle: XYHandle<String>,
    resizer: XYResizer,
    resize_node_id: Option<String>,
    resize_control_position: Option<String>,
    focus_handle: FocusHandle,
    drag_mode: DragMode,
    pressed: Option<PressedTarget>,
    node_drag_started: bool,
    selection_drag_started: bool,
    pane_moved: bool,
    last_pointer: XYPosition,
    pointer_down_at: Option<XYPosition>,
    pane_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    pattern: BackgroundPattern,
    controls: Option<ControlsProps>,
    minimap: Option<MinimapProps>,
    minimap_callbacks: MinimapCallbacks,
    minimap_node_component: Option<MinimapNodeComponent>,
    node_content_renderer: Option<NodeContentRenderer>,
    edge_content_renderer: Option<EdgeContentRenderer>,
    node_toolbar: Option<NodeToolbarProps>,
    node_toolbar_content: Option<NodeToolbarContent>,
    edge_toolbar: Option<EdgeToolbarProps>,
    edge_toolbar_content: Option<EdgeToolbarContent>,
    viewport_portal_items: Vec<ViewportPortalItem>,
    panel_items: Vec<PanelItem>,
    edge_label_renderer: Option<EdgeLabelContent>,
    node_element_renderer: Option<NodeElementRenderer>,
    edge_element_renderer: Option<EdgeElementRenderer>,
    handle_renderer: Option<HandleRenderer>,
    connection_line_renderer: Option<ConnectionLineRenderer>,
    node_components: bool,
    reconnect: Option<ReconnectDrag>,
    connection_click_start: Option<ClickStartHandle>,
    connection_dragged: bool,
    selection_key_pressed: bool,
    pan_activation_key_pressed: bool,
    zoom_activation_key_pressed: bool,
    hovered_edge: Option<String>,
    hovered_node: Option<String>,
    node_resizer: Option<NodeResizerProps>,
    focus_target: Option<FocusTarget>,
    started_at: Instant,
}

fn place_panel(element: Div, position: PanelPosition, margin: f64, width: f32, height: f32) -> Div {
    let margin = px(margin as f32);
    match position {
        PanelPosition::TopLeft => element.top(margin).left(margin),
        PanelPosition::TopRight => element.top(margin).right(margin),
        PanelPosition::BottomLeft => element.bottom(margin).left(margin),
        PanelPosition::BottomRight => element.bottom(margin).right(margin),
        PanelPosition::TopCenter => element.top(margin).left(relative(0.5)).ml(px(-width / 2.0)),
        PanelPosition::BottomCenter => element
            .bottom(margin)
            .left(relative(0.5))
            .ml(px(-width / 2.0)),
        PanelPosition::CenterLeft => element
            .left(margin)
            .top(relative(0.5))
            .mt(px(-height / 2.0)),
        PanelPosition::CenterRight => element
            .right(margin)
            .top(relative(0.5))
            .mt(px(-height / 2.0)),
    }
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
            handle: XYHandle::new(XYHandleOptions::default()),
            resizer: XYResizer::default(),
            resize_node_id: None,
            resize_control_position: None,
            focus_handle: cx.focus_handle(),
            drag_mode: DragMode::None,
            pressed: None,
            node_drag_started: false,
            selection_drag_started: false,
            pane_moved: false,
            last_pointer: XYPosition::new(0.0, 0.0),
            pointer_down_at: None,
            pane_bounds: Rc::new(Cell::new(None)),
            pattern: BackgroundPattern::default(),
            controls: None,
            minimap: None,
            minimap_callbacks: MinimapCallbacks::default(),
            minimap_node_component: None,
            node_content_renderer: None,
            edge_content_renderer: None,
            node_toolbar: None,
            node_toolbar_content: None,
            edge_toolbar: None,
            edge_toolbar_content: None,
            viewport_portal_items: Vec::new(),
            panel_items: Vec::new(),
            edge_label_renderer: None,
            node_element_renderer: None,
            edge_element_renderer: None,
            handle_renderer: None,
            connection_line_renderer: None,
            node_components: false,
            reconnect: None,
            connection_click_start: None,
            connection_dragged: false,
            selection_key_pressed: false,
            pan_activation_key_pressed: false,
            zoom_activation_key_pressed: false,
            hovered_edge: None,
            hovered_node: None,
            node_resizer: None,
            focus_target: None,
            started_at: Instant::now(),
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
            handle: XYHandle::new(XYHandleOptions::default()),
            resizer: XYResizer::default(),
            resize_node_id: None,
            resize_control_position: None,
            focus_handle: cx.focus_handle(),
            drag_mode: DragMode::None,
            pressed: None,
            node_drag_started: false,
            selection_drag_started: false,
            pane_moved: false,
            last_pointer: XYPosition::new(0.0, 0.0),
            pointer_down_at: None,
            pane_bounds: Rc::new(Cell::new(None)),
            pattern: BackgroundPattern::default(),
            controls: None,
            minimap: None,
            minimap_callbacks: MinimapCallbacks::default(),
            minimap_node_component: None,
            node_content_renderer: None,
            edge_content_renderer: None,
            node_toolbar: None,
            node_toolbar_content: None,
            edge_toolbar: None,
            edge_toolbar_content: None,
            viewport_portal_items: Vec::new(),
            panel_items: Vec::new(),
            edge_label_renderer: None,
            node_element_renderer: None,
            edge_element_renderer: None,
            handle_renderer: None,
            connection_line_renderer: None,
            node_components: false,
            reconnect: None,
            connection_click_start: None,
            connection_dragged: false,
            selection_key_pressed: false,
            pan_activation_key_pressed: false,
            zoom_activation_key_pressed: false,
            hovered_edge: None,
            hovered_node: None,
            node_resizer: None,
            focus_target: None,
            started_at: Instant::now(),
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
        self.pattern.variant = variant;
    }

    pub fn with_background_pattern(mut self, pattern: BackgroundPattern) -> Self {
        self.pattern = pattern;
        self
    }

    pub fn set_background_pattern(&mut self, pattern: BackgroundPattern) {
        self.pattern = pattern;
    }

    pub fn set_theme(&mut self, theme: crate::ui::theme::FlowTheme) {
        self.store.props_mut().theme = theme;
    }

    pub fn with_controls(mut self, props: ControlsProps) -> Self {
        self.controls = Some(props);
        self
    }

    pub fn with_minimap(mut self, props: MinimapProps) -> Self {
        self.minimap = Some(props);
        self
    }

    pub fn set_controls(&mut self, props: Option<ControlsProps>) {
        self.controls = props;
    }

    pub fn set_minimap(&mut self, props: Option<MinimapProps>) {
        self.minimap = props;
    }

    pub fn with_minimap_callbacks(mut self, callbacks: MinimapCallbacks) -> Self {
        self.minimap_callbacks = callbacks;
        self
    }

    pub fn set_minimap_callbacks(&mut self, callbacks: MinimapCallbacks) {
        self.minimap_callbacks = callbacks;
    }

    pub fn with_minimap_node_component(mut self, component: MinimapNodeComponent) -> Self {
        self.minimap_node_component = Some(component);
        self
    }

    pub fn set_minimap_node_component(&mut self, component: Option<MinimapNodeComponent>) {
        self.minimap_node_component = component;
    }

    pub fn with_node_content_renderer(mut self, renderer: NodeContentRenderer) -> Self {
        self.node_content_renderer = Some(renderer);
        self
    }

    pub fn set_node_content_renderer(&mut self, renderer: Option<NodeContentRenderer>) {
        self.node_content_renderer = renderer;
    }

    pub fn with_edge_content_renderer(mut self, renderer: EdgeContentRenderer) -> Self {
        self.edge_content_renderer = Some(renderer);
        self
    }

    pub fn set_edge_content_renderer(&mut self, renderer: Option<EdgeContentRenderer>) {
        self.edge_content_renderer = renderer;
    }

    pub fn with_node_toolbar(
        mut self,
        props: NodeToolbarProps,
        content: NodeToolbarContent,
    ) -> Self {
        self.node_toolbar = Some(props);
        self.node_toolbar_content = Some(content);
        self
    }

    pub fn set_node_toolbar(
        &mut self,
        props: Option<NodeToolbarProps>,
        content: Option<NodeToolbarContent>,
    ) {
        self.node_toolbar = props;
        self.node_toolbar_content = content;
    }

    pub fn with_edge_toolbar(
        mut self,
        props: EdgeToolbarProps,
        content: EdgeToolbarContent,
    ) -> Self {
        self.edge_toolbar = Some(props);
        self.edge_toolbar_content = Some(content);
        self
    }

    pub fn set_edge_toolbar(
        &mut self,
        props: Option<EdgeToolbarProps>,
        content: Option<EdgeToolbarContent>,
    ) {
        self.edge_toolbar = props;
        self.edge_toolbar_content = content;
    }

    pub fn with_viewport_portal(mut self, items: Vec<ViewportPortalItem>) -> Self {
        self.viewport_portal_items = items;
        self
    }

    pub fn set_viewport_portal(&mut self, items: Vec<ViewportPortalItem>) {
        self.viewport_portal_items = items;
    }

    pub fn with_node_resizer(mut self, props: NodeResizerProps) -> Self {
        self.node_resizer = Some(props);
        self
    }

    pub fn set_node_resizer(&mut self, props: Option<NodeResizerProps>) {
        self.node_resizer = props;
    }

    pub fn with_panel(mut self, item: PanelItem) -> Self {
        self.panel_items.push(item);
        self
    }

    pub fn set_panels(&mut self, items: Vec<PanelItem>) {
        self.panel_items = items;
    }

    pub fn with_edge_label_renderer(mut self, renderer: EdgeLabelContent) -> Self {
        self.edge_label_renderer = Some(renderer);
        self
    }

    pub fn set_edge_label_renderer(&mut self, renderer: Option<EdgeLabelContent>) {
        self.edge_label_renderer = renderer;
    }

    pub fn with_node_element_renderer(mut self, renderer: NodeElementRenderer) -> Self {
        self.node_element_renderer = Some(renderer);
        self
    }

    pub fn set_node_element_renderer(&mut self, renderer: Option<NodeElementRenderer>) {
        self.node_element_renderer = renderer;
    }

    pub fn with_edge_element_renderer(mut self, renderer: EdgeElementRenderer) -> Self {
        self.edge_element_renderer = Some(renderer);
        self
    }

    pub fn set_edge_element_renderer(&mut self, renderer: Option<EdgeElementRenderer>) {
        self.edge_element_renderer = renderer;
    }

    pub fn with_handle_renderer(mut self, renderer: HandleRenderer) -> Self {
        self.handle_renderer = Some(renderer);
        self
    }

    pub fn set_handle_renderer(&mut self, renderer: Option<HandleRenderer>) {
        self.handle_renderer = renderer;
    }

    pub fn with_connection_line_renderer(mut self, renderer: ConnectionLineRenderer) -> Self {
        self.connection_line_renderer = Some(renderer);
        self
    }

    pub fn set_connection_line_renderer(&mut self, renderer: Option<ConnectionLineRenderer>) {
        self.connection_line_renderer = renderer;
    }

    /// 启用元素层节点组件：节点底、边框与标签改由 GPUI 元素绘制（命中测试仍在画布上）。
    pub fn with_node_components(mut self) -> Self {
        self.node_components = true;
        self
    }

    pub fn set_node_components(&mut self, enabled: bool) {
        self.node_components = enabled;
    }

    pub fn node_components(&self) -> bool {
        self.node_components
    }

    pub fn toggle_interactivity(&mut self) -> bool {
        let props = self.store.props_mut();
        let interactive =
            props.nodes_draggable || props.nodes_connectable || props.elements_selectable;
        props.nodes_draggable = !interactive;
        props.nodes_connectable = !interactive;
        props.elements_selectable = !interactive;
        !interactive
    }

    fn render_controls(
        &self,
        props: ControlsProps,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = self.store.props().theme.clone();
        let mut container = place_panel(
            div()
                .absolute()
                .flex()
                .rounded(px(6.0))
                .bg(theme.controls_background)
                .border_1()
                .border_color(theme.controls_border)
                .overflow_hidden(),
            props.position,
            15.0,
            props.width(),
            props.height(),
        );
        if props.orientation == ControlsOrientation::Vertical {
            container = container.flex_col();
        }
        for button in props.buttons() {
            let button_size = props.button_size;
            let hover_background = theme.controls_background_hover;
            let content = match props.button_content(button) {
                Some(render) => render(window, cx),
                None => button.label().into_any_element(),
            };
            container = container.child(
                div()
                    .w(px(button_size))
                    .h(px(button_size))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_size(px(13.0))
                    .text_color(theme.controls_foreground)
                    .hover(move |style| style.bg(hover_background))
                    .child(content)
                    .id(format!("xyflow-control-{}", button.label()))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.control_clicked(button, window, cx);
                    })),
            );
        }
        for (index, item) in props.custom_items.iter().enumerate() {
            let button_size = props.button_size;
            container = container.child(
                div()
                    .w(px(button_size))
                    .h(px(button_size))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_size(px(13.0))
                    .text_color(theme.controls_foreground)
                    .hover(move |style| style.bg(theme.controls_background_hover))
                    .child((item.content)(window, cx))
                    .id(format!("xyflow-control-custom-{index}"))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            );
        }
        container
    }

    fn render_minimap(
        &self,
        props: MinimapProps,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.store.props().theme.clone();
        let bg_color = props.bg_color.unwrap_or(theme.minimap_background);
        let mask_color = props.mask_color.unwrap_or(theme.minimap_mask_background);
        let mask_stroke = props.mask_stroke_color.unwrap_or(theme.minimap_mask_stroke);
        let node_fill = props.node_color.unwrap_or(theme.minimap_node_background);
        let node_stroke = props.node_stroke_color.unwrap_or(theme.minimap_node_stroke);
        let size = self.store.size();
        let render = minimap_render(
            self.store.node_lookup(),
            self.store.transform(),
            size.width,
            size.height,
            &props,
        );
        let layout = render.layout;
        let scale = props.width / layout.width.max(1.0);
        let viewbox_x = layout.view_x - (layout.width - layout.view_width) / 2.0;
        let viewbox_y = layout.view_y - (layout.height - layout.view_height) / 2.0;

        let mut container = place_panel(
            div()
                .absolute()
                .w(px(props.width as f32))
                .h(px(props.height as f32))
                .rounded(px(4.0))
                .bg(bg_color)
                .border_1()
                .border_color(theme.controls_border)
                .overflow_hidden(),
            props.position,
            props.margin,
            props.width as f32,
            props.height as f32,
        );

        for node in &render.nodes {
            let x = ((node.x - viewbox_x) * scale) as f32;
            let y = ((node.y - viewbox_y) * scale) as f32;
            let width = (node.width * scale).max(2.0) as f32;
            let height = (node.height * scale).max(2.0) as f32;
            let content = if let Some(component) = self.minimap_node_component.as_ref() {
                let Some(internal) = self.store.node_lookup().get(&node.id) else {
                    continue;
                };
                component(&internal.internals.user_node, node.selected, window, cx)
            } else {
                div()
                    .w_full()
                    .h_full()
                    .rounded(px(props.node_border_radius as f32))
                    .bg(if node.selected {
                        node_stroke
                    } else {
                        node_fill
                    })
                    .border_1()
                    .border_color(node_stroke)
                    .into_any_element()
            };
            container = container.child(
                div()
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .w(px(width))
                    .h(px(height))
                    .child(content),
            );
        }

        let viewport_x = ((layout.view_x - viewbox_x) * scale) as f32;
        let viewport_y = ((layout.view_y - viewbox_y) * scale) as f32;
        let viewport_width = (layout.view_width * scale) as f32;
        let viewport_height = (layout.view_height * scale) as f32;
        let top_mask = div()
            .absolute()
            .left(px(0.0))
            .top(px(0.0))
            .w_full()
            .h(px(viewport_y.max(0.0)))
            .bg(mask_color);
        let bottom_mask = div()
            .absolute()
            .left(px(0.0))
            .top(px((viewport_y + viewport_height).max(0.0)))
            .w_full()
            .h(px(
                (props.height as f32 - viewport_y - viewport_height).max(0.0)
            ))
            .bg(mask_color);
        let left_mask = div()
            .absolute()
            .left(px(0.0))
            .top(px(viewport_y.max(0.0)))
            .w(px(viewport_x.max(0.0)))
            .h(px(viewport_height.max(0.0)))
            .bg(mask_color);
        let right_mask = div()
            .absolute()
            .left(px((viewport_x + viewport_width).max(0.0)))
            .top(px(viewport_y.max(0.0)))
            .w(px(
                (props.width as f32 - viewport_x - viewport_width).max(0.0)
            ))
            .h(px(viewport_height.max(0.0)))
            .bg(mask_color);
        container
            .id("xyflow-minimap")
            .child(top_mask)
            .child(bottom_mask)
            .child(left_mask)
            .child(right_mask)
            .child(
                div()
                    .absolute()
                    .left(px(viewport_x))
                    .top(px(viewport_y))
                    .w(px(viewport_width))
                    .h(px(viewport_height))
                    .border_1()
                    .border_color(mask_stroke),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.handle_minimap_mouse_down(event, window, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                this.handle_minimap_mouse_move(event, window, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, window, cx| {
                    this.handle_minimap_mouse_up(event, window, cx);
                }),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                this.handle_minimap_scroll(event, window, cx);
            }))
            .into_any_element()
    }

    fn render_node_toolbars(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<Div> {
        let (Some(props), Some(content)) = (&self.node_toolbar, &self.node_toolbar_content) else {
            return Vec::new();
        };
        let selected = self.store.selected_node_ids();
        let ids = node_toolbar_targets(props, self.store.node_lookup(), &selected);
        if ids.is_empty() {
            return Vec::new();
        }

        let transform = self.store.transform();
        let origin = self.store.props().node_origin;
        let offset = props.offset as f32;
        let mut overlays = Vec::new();

        let resolved: Vec<_> = ids
            .iter()
            .filter_map(|id| self.store.node_lookup().get(id))
            .collect();
        if resolved.is_empty() {
            return overlays;
        }
        let screens: Vec<_> = resolved
            .iter()
            .map(|internal| {
                node_screen_rect(&NodeBox::from_internal(internal, origin).rect, transform)
            })
            .collect();
        let group = resolved.len() > 1;
        let screen = if group {
            let min_x = screens
                .iter()
                .map(|rect| rect.x)
                .fold(f64::INFINITY, f64::min);
            let min_y = screens
                .iter()
                .map(|rect| rect.y)
                .fold(f64::INFINITY, f64::min);
            let max_x = screens
                .iter()
                .map(|rect| rect.x + rect.width)
                .fold(f64::NEG_INFINITY, f64::max);
            let max_y = screens
                .iter()
                .map(|rect| rect.y + rect.height)
                .fold(f64::NEG_INFINITY, f64::max);
            Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
        } else {
            screens[0]
        };
        let width = screen.width as f32;
        let height = screen.height as f32;
        let primary = &resolved[0].internals.user_node;
        let toolbar_id = if group {
            format!("group-{}", primary.id)
        } else {
            primary.id.clone()
        };

        let mut body = div()
            .absolute()
            .id(format!("xyflow-node-toolbar-{toolbar_id}"))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(content(primary, window, cx));

        body = match props.position {
            Position::Top => body.bottom(px(height + offset)).w_full().flex(),
            Position::Bottom => body.top(px(height + offset)).w_full().flex(),
            Position::Left => body.right(px(width + offset)).top(px(0.0)),
            Position::Right => body.left(px(width + offset)).top(px(0.0)),
        };
        if matches!(props.position, Position::Top | Position::Bottom) {
            body = match props.align {
                Alignment::Start => body.justify_start(),
                Alignment::Center => body.justify_center(),
                Alignment::End => body.justify_end(),
            };
        }

        overlays.push(
            div()
                .absolute()
                .left(px(screen.x as f32))
                .top(px(screen.y as f32))
                .w(px(width))
                .h(px(height))
                .child(body),
        );

        overlays
    }

    /// 元素层节点组件：按 z 排序、按视口裁剪地绘制节点底、边框与居中标签。
    ///
    /// 仅当 [`Flow::with_node_components`] 启用时生效；命中测试与拖拽仍由画布负责。
    fn render_node_wrapper_elements(&self, window: &Window) -> Vec<AnyElement> {
        if !self.node_components {
            return Vec::new();
        }
        // 首帧画布尚未执行 prepaint，用窗口尺寸兜底，避免节点组件缺一帧。
        let bounds = self.pane_bounds.get().unwrap_or_else(|| Bounds {
            origin: point(px(0.0), px(0.0)),
            size: window.viewport_size(),
        });
        let data = build_render_data(&self.store, bounds);
        let theme = &data.theme;
        let zoom = data.transform.zoom();
        data.nodes
            .nodes
            .iter()
            .map(|node| {
                let screen = node_screen_rect(&node.rect, data.transform);
                let border_width = if node.selected { 1.5 } else { 1.0 };
                let content_renderer = self.node_content_renderer.as_ref().and_then(|renderer| {
                    self.store
                        .node_lookup()
                        .get(&node.id)
                        .map(|source| (renderer.clone(), source.internals.user_node.clone()))
                });
                let mut wrapper = div()
                    .absolute()
                    .left(px(screen.x as f32))
                    .top(px(screen.y as f32))
                    .w(px(screen.width as f32))
                    .h(px(screen.height as f32))
                    .overflow_hidden()
                    .rounded(px(theme.node_border_radius))
                    .bg(node_fill(theme, node.variant, node.selected))
                    .border(px(border_width))
                    .border_color(node_border(theme, node.selected))
                    .id(format!("xyflow-node-wrapper-{}", node.id));
                if !node.label.is_empty() {
                    wrapper = wrapper
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(theme.node_font_size * zoom as f32))
                        .text_color(node_text_color(theme, node.variant))
                        .child(node.label.clone());
                }
                // 节点自定义内容随组件一起迁到元素层，避免被节点底遮住。
                if let Some((renderer, source)) = content_renderer {
                    wrapper = wrapper.child(
                        canvas(
                            |_bounds, _window, _cx| (),
                            move |bounds, _prepaint, window, cx| {
                                renderer(&source, bounds, window, cx);
                            },
                        )
                        .absolute()
                        .top(px(0.0))
                        .left(px(0.0))
                        .size_full(),
                    );
                }
                wrapper.into_any_element()
            })
            .collect()
    }

    /// 元素层默认手柄：节点组件启用后，手柄不能再由画布绘制（会被节点底遮挡）。
    ///
    /// 仅在未安装自定义手柄组件时生效。
    fn render_default_handle_elements(&self) -> Vec<AnyElement> {
        if !self.node_components || self.handle_renderer.is_some() {
            return Vec::new();
        }
        let transform = self.store.transform();
        let zoom = transform.zoom();
        let node_origin = self.store.props().node_origin;
        let theme = &self.store.props().theme;
        let diameter = ((theme.handle_size as f64) * zoom).max(4.0);
        let radius = diameter / 2.0;
        let mut elements = Vec::new();
        for node in self.store.node_lookup().values() {
            if node.hidden || node.node_type.as_deref() == Some("group") {
                continue;
            }
            let dimensions = node.size();
            let position = node.internals.position_absolute;
            let rect = Rect::new(
                position.x - dimensions.width * node_origin[0],
                position.y - dimensions.height * node_origin[1],
                dimensions.width,
                dimensions.height,
            );
            let bounds = node.handle_bounds_or(&node.internals.user_node.handles);
            for (index, handle) in bounds.all().enumerate() {
                let center = transform.apply(XYPosition::new(
                    rect.x + handle.x + handle.width / 2.0,
                    rect.y + handle.y + handle.height / 2.0,
                ));
                elements.push(
                    div()
                        .absolute()
                        .left(px((center.x - radius) as f32))
                        .top(px((center.y - radius) as f32))
                        .w(px(diameter as f32))
                        .h(px(diameter as f32))
                        .rounded(px(radius as f32))
                        .bg(handle_fill(theme, handle.handle_type))
                        .id(format!("xyflow-default-handle-{}-{}", node.id, index))
                        .into_any_element(),
                );
            }
        }
        elements
    }

    fn render_node_elements(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(renderer) = self.node_element_renderer.as_ref() else {
            return Vec::new();
        };
        let transform = self.store.transform();
        let zoom = transform.zoom();
        let pane_origin = self
            .pane_bounds
            .get()
            .map(|bounds| bounds.origin)
            .unwrap_or_else(|| point(px(0.0), px(0.0)));
        self.store
            .node_lookup()
            .values()
            .filter(|node| !node.hidden)
            .map(|node| {
                let dimensions = node.size();
                let screen = transform.apply(XYPosition::new(
                    node.internals.position_absolute.x,
                    node.internals.position_absolute.y,
                ));
                let width = (dimensions.width * zoom) as f32;
                let height = (dimensions.height * zoom) as f32;
                let bounds = Bounds {
                    origin: point(
                        pane_origin.x + px(screen.x as f32),
                        pane_origin.y + px(screen.y as f32),
                    ),
                    size: gpui_kit::size(px(width), px(height)),
                };
                div()
                    .absolute()
                    .left(px(screen.x as f32))
                    .top(px(screen.y as f32))
                    .w(px(width))
                    .h(px(height))
                    .id(format!("xyflow-node-element-{}", node.id))
                    .child(renderer(
                        &node.internals.user_node,
                        bounds,
                        zoom,
                        window,
                        cx,
                    ))
                    .into_any_element()
            })
            .collect()
    }

    fn render_edge_elements(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(renderer) = self.edge_element_renderer.as_ref() else {
            return Vec::new();
        };
        let transform = self.store.transform();
        let pane_origin = self
            .pane_bounds
            .get()
            .map(|bounds| bounds.origin)
            .unwrap_or_else(|| point(px(0.0), px(0.0)));
        let mut elements = Vec::new();
        for edge in self.store.edges() {
            if edge.hidden {
                continue;
            }
            let Some(graphics) = self.store.edge_path_for(edge) else {
                continue;
            };
            let samples = parse_svg_path(&graphics.path).sample(12);
            let Some(first) = samples.first() else {
                continue;
            };
            let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
            for sample in &samples {
                min_x = min_x.min(sample.x);
                min_y = min_y.min(sample.y);
                max_x = max_x.max(sample.x);
                max_y = max_y.max(sample.y);
            }
            let origin = transform.apply(XYPosition::new(min_x, min_y));
            let corner = transform.apply(XYPosition::new(max_x, max_y));
            let width = (corner.x - origin.x).max(1.0) as f32;
            let height = (corner.y - origin.y).max(1.0) as f32;
            let element_bounds = Bounds {
                origin: point(
                    pane_origin.x + px(origin.x as f32),
                    pane_origin.y + px(origin.y as f32),
                ),
                size: gpui_kit::size(px(width), px(height)),
            };
            elements.push(
                div()
                    .absolute()
                    .left(px(origin.x as f32))
                    .top(px(origin.y as f32))
                    .w(px(width))
                    .h(px(height))
                    .id(format!("xyflow-edge-element-{}", edge.id))
                    .child(renderer(edge, element_bounds, transform, window, cx))
                    .into_any_element(),
            );
        }
        elements
    }

    fn render_handle_elements(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let Some(renderer) = self.handle_renderer.as_ref() else {
            return Vec::new();
        };
        let transform = self.store.transform();
        let zoom = transform.zoom();
        let node_origin = self.store.props().node_origin;
        let theme = &self.store.props().theme;
        let diameter = ((theme.handle_size as f64) * zoom).max(4.0) as f32;
        let pane_origin = self
            .pane_bounds
            .get()
            .map(|bounds| bounds.origin)
            .unwrap_or_else(|| point(px(0.0), px(0.0)));
        let mut overlays = Vec::new();
        for node in self.store.node_lookup().values() {
            if node.hidden || node.node_type.as_deref() == Some("group") {
                continue;
            }
            let dimensions = node.size();
            let position = node.internals.position_absolute;
            let rect = Rect::new(
                position.x - dimensions.width * node_origin[0],
                position.y - dimensions.height * node_origin[1],
                dimensions.width,
                dimensions.height,
            );
            let user_node = &node.internals.user_node;
            let bounds = node.handle_bounds_or(&user_node.handles);
            for (index, handle) in bounds.all().enumerate() {
                let center = transform.apply(XYPosition::new(
                    rect.x + handle.x + handle.width / 2.0,
                    rect.y + handle.y + handle.height / 2.0,
                ));
                let left = center.x as f32 - diameter / 2.0;
                let top = center.y as f32 - diameter / 2.0;
                let box_bounds = Bounds {
                    origin: point(pane_origin.x + px(left), pane_origin.y + px(top)),
                    size: gpui_kit::size(px(diameter), px(diameter)),
                };
                overlays.push(
                    div()
                        .absolute()
                        .left(px(left))
                        .top(px(top))
                        .w(px(diameter))
                        .h(px(diameter))
                        .id(format!(
                            "xyflow-handle-element-{}-{}",
                            node.id,
                            handle.id.clone().unwrap_or_else(|| index.to_string())
                        ))
                        .child(renderer(user_node, handle, box_bounds, zoom, window, cx))
                        .into_any_element(),
                );
            }
        }
        overlays
    }

    fn render_resize_control_elements(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let Some(props) = self.node_resizer.as_ref() else {
            return Vec::new();
        };
        let Some(renderer) = props.control_renderer.as_ref() else {
            return Vec::new();
        };
        let transform = self.store.transform();
        let zoom = transform.zoom();
        let handle_size = props.handle_size;
        let line_width = props.line_width;
        let pane_origin = self
            .pane_bounds
            .get()
            .map(|bounds| bounds.origin)
            .unwrap_or_else(|| point(px(0.0), px(0.0)));
        let mut overlays = Vec::new();
        for (node_id, rect) in self.resize_targets_with(&self.store) {
            for control in resize_controls(rect, handle_size, line_width) {
                let screen = transform.apply(XYPosition::new(control.rect.x, control.rect.y));
                let width = (control.rect.width * zoom) as f32;
                let height = (control.rect.height * zoom) as f32;
                let control_bounds = Bounds {
                    origin: point(
                        pane_origin.x + px(screen.x as f32),
                        pane_origin.y + px(screen.y as f32),
                    ),
                    size: gpui_kit::size(px(width), px(height)),
                };
                overlays.push(
                    div()
                        .absolute()
                        .left(px(screen.x as f32))
                        .top(px(screen.y as f32))
                        .w(px(width))
                        .h(px(height))
                        .id(format!(
                            "xyflow-resize-control-{}-{}",
                            node_id, control.position
                        ))
                        .child(renderer(&control, control_bounds, zoom, window, cx))
                        .into_any_element(),
                );
            }
        }
        overlays
    }

    fn render_connection_line(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let renderer = self.connection_line_renderer.as_ref()?;
        let ConnectionState::InProgress(progress) = self.store.connection_state() else {
            return None;
        };
        let bounds = self.pane_bounds.get()?;
        let props = ConnectionLineProps::from_progress(progress);
        let transform = self.store.transform();
        Some(
            div()
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .w(bounds.size.width)
                .h(bounds.size.height)
                .id("xyflow-connection-line")
                .child(renderer(&props, bounds, transform, window, cx))
                .into_any_element(),
        )
    }

    fn render_panels(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        self.panel_items
            .iter()
            .map(|item| {
                let margin = px(item.margin as f32);
                let offset = px(item.offset as f32);
                let container = div().absolute().id(format!("xyflow-panel-{}", item.id));
                let container = match item.position {
                    PanelPosition::TopLeft => container.top(margin + offset).left(margin),
                    PanelPosition::TopRight => container.top(margin + offset).right(margin),
                    PanelPosition::BottomLeft => container.bottom(margin + offset).left(margin),
                    PanelPosition::BottomRight => container.bottom(margin + offset).right(margin),
                    PanelPosition::TopCenter => container
                        .top(margin + offset)
                        .left(px(0.0))
                        .right(px(0.0))
                        .flex()
                        .justify_center(),
                    PanelPosition::BottomCenter => container
                        .bottom(margin + offset)
                        .left(px(0.0))
                        .right(px(0.0))
                        .flex()
                        .justify_center(),
                    PanelPosition::CenterLeft => container
                        .left(margin + offset)
                        .top(px(0.0))
                        .bottom(px(0.0))
                        .flex()
                        .items_center(),
                    PanelPosition::CenterRight => container
                        .right(margin + offset)
                        .top(px(0.0))
                        .bottom(px(0.0))
                        .flex()
                        .items_center(),
                };
                container
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child((item.content)(window, cx))
                    .into_any_element()
            })
            .collect()
    }

    fn render_edge_labels(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(renderer) = self.edge_label_renderer.as_ref() else {
            return Vec::new();
        };
        let transform = self.store.transform();
        self.store
            .edge_lookup()
            .values()
            .filter(|edge| !edge.hidden)
            .filter_map(|edge| {
                let graphics = self.store.edge_path_for(edge)?;
                let screen = transform.apply(XYPosition::new(
                    graphics.label_x + graphics.offset_x,
                    graphics.label_y + graphics.offset_y,
                ));
                Some(
                    div()
                        .absolute()
                        .left(px(screen.x as f32))
                        .top(px(screen.y as f32))
                        .id(format!("xyflow-edge-label-{}", edge.id))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                        .child(renderer(edge, window, cx))
                        .into_any_element(),
                )
            })
            .collect()
    }

    fn render_edge_toolbar(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<Div> {
        let props = self.edge_toolbar.as_ref()?;
        let content = self.edge_toolbar_content.as_ref()?;
        let edge = self.store.edge_lookup().get(&props.edge_id)?.clone();
        if !edge_toolbar(props, Some(&edge), self.store.get_zoom()).active {
            return None;
        }

        let screen = self
            .store
            .transform()
            .apply(XYPosition::new(props.x, props.y));
        let mut body = div()
            .absolute()
            .id(format!("xyflow-edge-toolbar-{}", props.edge_id))
            .flex()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(content(&edge, window, cx));

        body = match props.align_x {
            ToolbarAlignX::Left => body.left(px(0.0)),
            ToolbarAlignX::Center | ToolbarAlignX::Right => body.right(px(0.0)),
        };
        body = match props.align_y {
            ToolbarAlignY::Top => body.top(px(0.0)),
            ToolbarAlignY::Center | ToolbarAlignY::Bottom => body.bottom(px(0.0)),
        };

        Some(
            div()
                .absolute()
                .left(px(screen.x as f32))
                .top(px(screen.y as f32))
                .w(px(0.0))
                .h(px(0.0))
                .child(body),
        )
    }

    fn render_viewport_portal(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<Div> {
        if self.viewport_portal_items.is_empty() {
            return Vec::new();
        }
        let transform = self.store.transform();

        self.viewport_portal_items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let screen = transform.apply(XYPosition::new(item.x, item.y));
                div()
                    .absolute()
                    .left(px(screen.x as f32))
                    .top(px(screen.y as f32))
                    .child(
                        div()
                            .id(format!("xyflow-viewport-portal-{index}"))
                            .flex()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                            .child((item.content)(window, cx)),
                    )
            })
            .collect()
    }

    fn render_attribution(&self) -> Option<AnyElement> {
        let props = self.store.props();
        let badge = attribution(&props.lib, props.hide_attribution, "bottom-right");
        if !badge.visible {
            return None;
        }
        let theme = props.theme.clone();
        let href = badge.href.clone();

        Some(
            div()
                .absolute()
                .bottom(px(8.0))
                .right(px(8.0))
                .id("xyflow-attribution")
                .px(px(8.0))
                .py(px(3.0))
                .rounded(px(4.0))
                .bg(theme.controls_background)
                .border_1()
                .border_color(theme.controls_border)
                .text_size(px(10.0))
                .text_color(theme.controls_foreground)
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(move |_, _, cx| cx.open_url(&href))
                .child(badge.label)
                .into_any_element(),
        )
    }

    fn control_clicked(
        &mut self,
        button: ControlButton,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let viewport_before = self.store.viewport();
        let transform_before = self.store.transform();
        let callbacks = self.controls.as_ref().map(|props| props.callbacks.clone());
        let mut interactive = None;
        match button {
            ControlButton::ZoomIn => self.store.zoom_in(),
            ControlButton::ZoomOut => self.store.zoom_out(),
            ControlButton::FitView => self.fit_view(window),
            ControlButton::Interactive => interactive = Some(self.toggle_interactivity()),
        }
        if let Some(callbacks) = callbacks {
            match button {
                ControlButton::ZoomIn => {
                    if let Some(handler) = callbacks.on_zoom_in {
                        handler();
                    }
                }
                ControlButton::ZoomOut => {
                    if let Some(handler) = callbacks.on_zoom_out {
                        handler();
                    }
                }
                ControlButton::FitView => {
                    if let Some(handler) = callbacks.on_fit_view {
                        handler();
                    }
                }
                ControlButton::Interactive => {
                    if let Some(handler) = callbacks.on_interactive_change
                        && let Some(interactive) = interactive
                    {
                        handler(interactive);
                    }
                }
            }
        }
        self.notify_viewport_change(
            viewport_before,
            transform_before,
            ViewportCallbackKind::Move,
        );
        cx.notify();
    }

    pub fn fit_view(&mut self, _window: &mut Window) {
        if let Some(bounds) = self.pane_bounds.get() {
            self.store.set_size(
                f32::from(bounds.size.width) as f64,
                f32::from(bounds.size.height) as f64,
            );
            let options = self
                .controls
                .as_ref()
                .map(|controls| controls.fit_view_options)
                .unwrap_or_default();
            let bounds = self
                .store
                .bounds_including_hidden(options.include_hidden_nodes);
            if bounds.width > 0.0 && bounds.height > 0.0 {
                self.store.fit_bounds_with(
                    bounds,
                    options.padding,
                    options.min_zoom.unwrap_or(self.store.props().min_zoom),
                    options.max_zoom.unwrap_or(self.store.props().max_zoom),
                );
            }
        }
    }

    fn local_position(&self, window_position: gpui_kit::Point<Pixels>) -> XYPosition {
        match self.pane_bounds.get() {
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
            let Some(bounds) = &node.internals.handle_bounds else {
                continue;
            };
            for handle in bounds.source.iter().chain(bounds.target.iter()) {
                let center_x = rect.x + handle.x + handle.width / 2.0;
                let center_y = rect.y + handle.y + handle.height / 2.0;
                let dx = flow_point.x - center_x;
                let dy = flow_point.y - center_y;
                if dx * dx + dy * dy <= radius * radius {
                    return Some((node.id.clone(), handle.handle_type, handle.id.clone()));
                }
            }
        }
        None
    }

    fn reconnect_handle_at(&self, flow_point: XYPosition) -> Option<(String, ReconnectEndpoint)> {
        let props = self.store.props();
        if !props.edges_reconnectable {
            return None;
        }
        let radius = (props.reconnect_radius / self.store.get_zoom()).max(4.0);
        let mut best: Option<(String, ReconnectEndpoint, f64)> = None;

        for edge in self.store.edge_lookup().values() {
            if !edge.selected || edge.hidden {
                continue;
            }
            let Some(graphics) = self.store.edge_path_for(edge) else {
                continue;
            };
            let samples = parse_svg_path(&graphics.path).sample(2);
            if samples.len() < 2 {
                continue;
            }
            let endpoints = [
                (ReconnectEndpoint::Source, samples[0]),
                (ReconnectEndpoint::Target, samples[samples.len() - 1]),
            ];
            for (endpoint, point) in endpoints {
                let dx = point.x - flow_point.x;
                let dy = point.y - flow_point.y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance <= radius
                    && best
                        .as_ref()
                        .map(|(_, _, current)| distance < *current)
                        .unwrap_or(true)
                {
                    best = Some((edge.id.clone(), endpoint, distance));
                }
            }
        }

        best.map(|(edge_id, endpoint, _)| (edge_id, endpoint))
    }

    fn select_node(&mut self, node_id: &str, additive: bool) {
        let next = self
            .store
            .node_lookup()
            .get(node_id)
            .map(|node| if additive { !node.selected } else { true })
            .unwrap_or(true);
        if !additive {
            self.store.unselect_nodes_and_edges();
        }
        self.store.set_node_selected(node_id, next);
    }

    fn select_edge(&mut self, edge_id: &str, additive: bool) {
        let next = self
            .store
            .edge_lookup()
            .get(edge_id)
            .map(|edge| if additive { !edge.selected } else { true })
            .unwrap_or(true);
        if !additive {
            self.store.unselect_nodes_and_edges();
        }
        self.store.set_edge_selected(edge_id, next);
    }

    fn clear_selection(&mut self) {
        self.store.unselect_nodes_and_edges();
    }

    fn selection_signature(&self) -> (Vec<String>, Vec<String>) {
        (
            self.store.selected_node_ids(),
            self.store.selected_edge_ids(),
        )
    }

    fn notify_selection_change(&self, before: &(Vec<String>, Vec<String>)) {
        if *before != self.selection_signature()
            && let Some(handler) = self.store.props().callbacks.on_selection_change.clone()
        {
            handler();
        }
    }

    fn user_node(&self, id: &str) -> Option<Node<String>> {
        self.store
            .node_lookup()
            .get(id)
            .map(|node| node.internals.user_node.clone())
    }

    fn user_edge(&self, id: &str) -> Option<Edge<String>> {
        self.store.edge_lookup().get(id).cloned()
    }

    fn edge_at(&self, flow_point: XYPosition) -> Option<String> {
        edge_at_point(&self.store, flow_point, self.store.get_zoom())
    }

    fn dragging_nodes(&self) -> Vec<Node<String>> {
        self.store
            .nodes()
            .iter()
            .filter(|node| node.dragging)
            .cloned()
            .collect()
    }

    fn selected_user_nodes(&self) -> Vec<Node<String>> {
        self.store
            .node_lookup()
            .values()
            .filter(|node| node.selected)
            .map(|node| node.internals.user_node.clone())
            .collect()
    }

    fn connection_is_valid(&self, connection: &Connection) -> bool {
        let props = self.store.props();
        let Some(source) = self.store.node_lookup().get(&connection.source) else {
            return false;
        };
        let Some(target) = self.store.node_lookup().get(&connection.target) else {
            return false;
        };
        if source.hidden || target.hidden {
            return false;
        }
        if !source
            .internals
            .user_node
            .is_connectable(props.nodes_connectable)
            || !target
                .internals
                .user_node
                .is_connectable(props.nodes_connectable)
        {
            return false;
        }
        let candidate = Edge {
            source: connection.source.clone(),
            target: connection.target.clone(),
            source_handle: connection.source_handle.clone(),
            target_handle: connection.target_handle.clone(),
            ..Edge::default()
        };
        if connection_exists(&candidate, self.store.edges()) {
            return false;
        }
        props
            .callbacks
            .is_valid_connection
            .as_ref()
            .map(|handler| handler(connection))
            .unwrap_or(true)
    }

    fn notify_viewport_change(
        &self,
        viewport_before: crate::core::geometry::Viewport,
        transform_before: Transform,
        kind: ViewportCallbackKind,
    ) {
        let callbacks = self.store.props().callbacks.clone();
        let viewport = self.store.viewport();
        let transform = self.store.transform();
        match kind {
            ViewportCallbackKind::Start => {
                if let Some(handler) = callbacks.on_move_start.clone() {
                    handler(viewport);
                }
                if let Some(handler) = callbacks.on_viewport_change_start.clone() {
                    handler(transform);
                }
            }
            ViewportCallbackKind::Move => {
                if viewport != viewport_before
                    && let Some(handler) = callbacks.on_move.clone()
                {
                    handler(viewport);
                }
                if transform != transform_before
                    && let Some(handler) = callbacks.on_viewport_change.clone()
                {
                    handler(transform);
                }
            }
            ViewportCallbackKind::End => {
                if let Some(handler) = callbacks.on_move_end.clone() {
                    handler(viewport);
                }
                if let Some(handler) = callbacks.on_viewport_change_end.clone() {
                    handler(transform);
                }
            }
        }
    }

    fn delete_selected(&mut self) {
        let nodes_deletable = self.store.props().nodes_deletable;
        let edges_deletable = self.store.props().edges_deletable;
        let selected_nodes: Vec<Node<String>> = self
            .store
            .node_lookup()
            .values()
            .filter(|node| {
                nodes_deletable && node.selected && node.internals.user_node.is_deletable()
            })
            .map(|node| node.internals.user_node.clone())
            .collect();
        let selected_edges: Vec<Edge<String>> = self
            .store
            .edge_lookup()
            .values()
            .filter(|edge| edges_deletable && edge.selected && edge.is_deletable())
            .cloned()
            .collect();
        if selected_nodes.is_empty() && selected_edges.is_empty() {
            return;
        }
        let before = self.selection_signature();
        let (nodes_to_remove, edges_to_remove) = get_elements_to_remove(
            &selected_nodes,
            &selected_edges,
            self.store.nodes(),
            self.store.edges(),
        );
        if nodes_to_remove.is_empty() && edges_to_remove.is_empty() {
            return;
        }
        let node_ids: std::collections::BTreeSet<&str> = nodes_to_remove
            .iter()
            .map(|node| node.id.as_str())
            .collect();
        let edge_ids: std::collections::BTreeSet<&str> = edges_to_remove
            .iter()
            .map(|edge| edge.id.as_str())
            .collect();
        let nodes: Vec<Node<String>> = self
            .store
            .nodes()
            .iter()
            .filter(|node| !node_ids.contains(node.id.as_str()))
            .cloned()
            .collect();
        let edges: Vec<Edge<String>> = self
            .store
            .edges()
            .iter()
            .filter(|edge| !edge_ids.contains(edge.id.as_str()))
            .cloned()
            .collect();
        self.store.set_nodes(nodes);
        self.store.set_edges(edges);
        self.store.update_nodes();
        if let Some(handler) = self.store.props().callbacks.on_nodes_delete.clone() {
            handler(&nodes_to_remove);
        }
        if let Some(handler) = self.store.props().callbacks.on_edges_delete.clone() {
            handler(&edges_to_remove);
        }
        if let Some(handler) = self.store.props().callbacks.on_delete.clone() {
            handler(&nodes_to_remove, &edges_to_remove);
        }
        self.notify_selection_change(&before);
    }

    fn move_selected_by_key(&mut self, key: &str, shift: bool) {
        let Some(diff) = arrow_key_diff(key) else {
            return;
        };
        let factor = if shift { 4.0 } else { 1.0 };
        let snap_to_grid = self.store.props().snap_to_grid;
        let snap_grid = self.store.props().snap_grid;
        let velocity = if snap_to_grid {
            [snap_grid[0].abs(), snap_grid[1].abs()]
        } else {
            [5.0, 5.0]
        };
        let delta = [diff.x * velocity[0] * factor, diff.y * velocity[1] * factor];
        if delta == [0.0, 0.0] {
            return;
        }
        let mut changes: Vec<NodeChange<String>> = Vec::new();
        for id in self.store.selected_node_ids() {
            let Some(node) = self.store.node_lookup().get(&id) else {
                continue;
            };
            if !node
                .internals
                .user_node
                .is_draggable(self.store.props().nodes_draggable)
            {
                continue;
            }
            let position_absolute = node.internals.position_absolute;
            let Some(update) = calculate_node_position(CalculateNodePositionParams {
                node_id: &id,
                next_position: XYPosition::new(
                    position_absolute.x + delta[0],
                    position_absolute.y + delta[1],
                ),
                node_lookup: self.store.node_lookup(),
                node_origin: self.store.props().node_origin,
                node_extent: self.store.props().node_extent,
            }) else {
                continue;
            };
            changes.push(NodeChange::position(
                id,
                update.position.x,
                update.position.y,
                false,
            ));
        }
        if changes.is_empty() {
            return;
        }
        if self.store.has_default_nodes() {
            self.store.trigger_node_changes(changes);
        } else {
            self.store.apply_node_changes(&changes);
            self.store.trigger_node_changes(changes);
        }
        self.store.update_nodes();
    }

    fn cancel_connection(&mut self) {
        self.handle.cancel();
        self.store
            .set_connection(ConnectionState::Idle(Default::default()));
        if matches!(self.drag_mode, DragMode::Connection | DragMode::Reconnect) {
            self.drag_mode = DragMode::None;
        }
        self.reconnect = None;
        self.connection_click_start = None;
        self.connection_dragged = false;
        self.pressed = None;
    }

    fn update_key_state(&mut self, key: &str, pressed: bool) {
        let props = self.store.props();
        if key_matches(props.selection_key_code.as_deref(), key) {
            self.selection_key_pressed = pressed;
        }
        if key_matches(props.pan_activation_key_code.as_deref(), key) {
            self.pan_activation_key_pressed = pressed;
        }
        if key_matches(props.zoom_activation_key_code.as_deref(), key) {
            self.zoom_activation_key_pressed = pressed;
        }
        self.sync_pan_zoom_options();
    }

    fn selection_active(&self, event: &MouseDownEvent) -> bool {
        let additive_modifier = event.modifiers.control || event.modifiers.platform;
        let configured = match self.store.props().selection_key_code.as_deref() {
            Some(code) => key_matches(
                Some(code),
                if event.modifiers.shift {
                    "shift"
                } else if event.modifiers.alt {
                    "alt"
                } else if additive_modifier {
                    "control"
                } else {
                    ""
                },
            ),
            None => false,
        };
        self.selection_key_pressed || configured || additive_modifier
    }

    fn focusable_targets(&self) -> Vec<FocusTarget> {
        let props = self.store.props();
        let mut targets = Vec::new();
        if props.nodes_focusable {
            targets.extend(
                self.store
                    .node_lookup()
                    .values()
                    .filter(|node| !node.hidden)
                    .map(|node| FocusTarget::Node(node.id.clone())),
            );
        }
        if props.edges_focusable {
            targets.extend(
                self.store
                    .edge_lookup()
                    .values()
                    .filter(|edge| !edge.hidden)
                    .map(|edge| FocusTarget::Edge(edge.id.clone())),
            );
        }
        targets
    }

    fn cycle_focus(&mut self, reverse: bool) {
        let targets = self.focusable_targets();
        let current = self
            .focus_target
            .as_ref()
            .and_then(|target| targets.iter().position(|candidate| candidate == target));
        let Some(index) = next_focus_index(current, targets.len(), reverse) else {
            self.focus_target = None;
            return;
        };
        self.focus_target = Some(targets[index].clone());
        let label = match &targets[index] {
            FocusTarget::Node(id) | FocusTarget::Edge(id) => id.as_str(),
        };
        self.store.props_mut().aria_live_message = format!("Focused {label}");
    }

    fn activate_focus(&mut self) {
        if !self.store.props().elements_selectable {
            return;
        }
        let target = self.focus_target.clone();
        self.store.unselect_nodes_and_edges();
        match target {
            Some(FocusTarget::Node(id)) => self.store.set_node_selected(&id, true),
            Some(FocusTarget::Edge(id)) => self.store.set_edge_selected(&id, true),
            None => {}
        }
    }

    fn focus_rect(&self, store: &FlowStore<String>) -> Option<Rect> {
        match self.focus_target.as_ref()? {
            FocusTarget::Node(id) => {
                let node = store.node_lookup().get(id)?;
                if node.hidden {
                    return None;
                }
                let dimensions = node.size();
                Some(Rect::new(
                    node.internals.position_absolute.x,
                    node.internals.position_absolute.y,
                    dimensions.width,
                    dimensions.height,
                ))
            }
            FocusTarget::Edge(id) => {
                let edge = store.edge_lookup().get(id)?;
                if edge.hidden {
                    return None;
                }
                let graphics = store.edge_path_for(edge)?;
                Some(Rect::new(
                    graphics.label_x + graphics.offset_x - 12.0,
                    graphics.label_y + graphics.offset_y - 12.0,
                    24.0,
                    24.0,
                ))
            }
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.to_lowercase();
        self.update_key_state(&key, true);
        if key == "tab"
            && !self.store.props().disable_keyboard_a11y
            && (self.store.props().nodes_focusable || self.store.props().edges_focusable)
        {
            self.cycle_focus(event.keystroke.modifiers.shift);
            cx.notify();
            return;
        }
        if (key == "enter" || key == "space") && self.focus_target.is_some() {
            self.activate_focus();
            cx.notify();
            return;
        }
        if key == self.store.props().delete_key_code.to_lowercase() {
            self.delete_selected();
            cx.notify();
            return;
        }
        if key == "escape" {
            self.focus_target = None;
            self.cancel_connection();
            cx.notify();
            return;
        }
        if matches!(key.as_str(), "up" | "down" | "left" | "right")
            || matches!(
                key.as_str(),
                "arrowup" | "arrowdown" | "arrowleft" | "arrowright"
            )
        {
            self.move_selected_by_key(&key, event.keystroke.modifiers.shift);
            cx.notify();
        }
    }

    fn on_key_up(&mut self, event: &KeyUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.to_lowercase();
        self.update_key_state(&key, false);
        cx.notify();
    }

    fn sync_pan_zoom_options(&mut self) {
        let keys = RendererKeyState {
            selection_key_pressed: self.selection_key_pressed,
            pan_activation_key_pressed: self.pan_activation_key_pressed,
            zoom_activation_key_pressed: self.zoom_activation_key_pressed,
        };
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

    /// Tries to complete a click-started connection on `clicked`.
    fn finish_click_connection(
        &mut self,
        start: &ClickStartHandle,
        clicked: &ClickStartHandle,
    ) -> bool {
        let props = self.store.props();
        let edges = self.store.edges().to_vec();
        let lookup = self.store.node_lookup().clone();
        let custom = props.callbacks.is_valid_connection.clone();
        let nodes_connectable = props.nodes_connectable;
        let is_valid = move |connection: &Connection| {
            let Some(source) = lookup.get(&connection.source) else {
                return false;
            };
            let Some(target) = lookup.get(&connection.target) else {
                return false;
            };
            if source.hidden
                || target.hidden
                || !source.internals.user_node.is_connectable(nodes_connectable)
                || !target.internals.user_node.is_connectable(nodes_connectable)
            {
                return false;
            }
            let candidate = Edge {
                source: connection.source.clone(),
                target: connection.target.clone(),
                source_handle: connection.source_handle.clone(),
                target_handle: connection.target_handle.clone(),
                ..Edge::default()
            };
            if connection_exists(&candidate, &edges) {
                return false;
            }
            custom
                .as_ref()
                .map(|handler| handler(connection))
                .unwrap_or(true)
        };
        let result =
            crate::core::connect::check_click_connection(crate::core::connect::CheckHandleParams {
                node_id: &clicked.node_id,
                handle_id: clicked.handle_id.as_deref(),
                handle_type: clicked.handle_type,
                from_node_id: &start.node_id,
                from_handle_id: start.handle_id.as_deref(),
                from_type: start.handle_type,
                connection_mode: props.connection_mode,
                is_valid_connection: &is_valid,
                node_lookup: self.store.node_lookup(),
            });
        let Some(connection) = result.connection else {
            return false;
        };
        let edge = Edge {
            source: connection.source.clone(),
            target: connection.target.clone(),
            source_handle: connection.source_handle.clone(),
            target_handle: connection.target_handle.clone(),
            ..Edge::default()
        };
        let Some(edges) = add_edge(
            edge,
            self.store.edges(),
            self.store.props().default_edge_options.as_ref(),
        ) else {
            return false;
        };
        let mut completed = match self.store.connection_state() {
            ConnectionState::InProgress(state) => state.clone(),
            ConnectionState::Idle(_) => {
                let lookup = self.store.node_lookup().clone();
                let Some(mut state) = crate::core::connect::start_connection(
                    crate::core::connect::StartConnectionParams {
                        node_id: &start.node_id,
                        handle_id: start.handle_id.as_deref(),
                        handle_type: start.handle_type,
                        pointer: XYPosition::new(0.0, 0.0),
                        transform: self.store.transform(),
                        connection_mode: self.store.props().connection_mode,
                        connection_radius: self.store.props().connection_radius,
                        node_lookup: &lookup,
                    },
                ) else {
                    return false;
                };
                state.to_handle = result.to_handle.clone();
                state
            }
        };
        completed.is_valid = Some(true);
        completed.to_handle = result.to_handle.clone();
        if let Some(handle) = result.to_handle.clone() {
            completed.to = XYPosition::new(handle.x, handle.y);
            completed.to_position = handle.position;
            completed.to_node = self.store.node_lookup().get(&handle.node_id).cloned();
        }
        self.store.set_edges(edges);
        self.store.update_nodes();
        if let Some(handler) = self.store.props().callbacks.on_connect.clone() {
            handler(&completed);
        }
        if let Some(handler) = self.store.props().callbacks.on_click_connect_end.clone() {
            handler(&completed);
        }
        self.store
            .set_connection(ConnectionState::Idle(Default::default()));
        self.connection_click_start = None;
        self.connection_dragged = false;
        true
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

    fn sync_handle_options(&mut self) {
        let props = self.store.props();
        let options = XYHandleOptions {
            connection_mode: props.connection_mode,
            connection_radius: props.connection_radius,
            drag_threshold: props.connection_drag_threshold,
            auto_pan_on_connect: props.auto_pan_on_connect,
            auto_pan_speed: props.auto_pan_speed,
            lib: props.lib.clone(),
        };
        self.handle.set_options(options);
    }

    fn sync_resizer_options(&mut self) {
        let props = self.store.props();
        let (keep_aspect_ratio, boundaries) = self
            .node_resizer
            .as_ref()
            .map(|resizer| (resizer.keep_aspect_ratio, resizer.boundaries()))
            .unwrap_or_else(|| {
                (
                    false,
                    crate::core::xyresizer::Boundaries::with_defaults(0.0, 0.0),
                )
            });
        let options = XYResizerOptions {
            keep_aspect_ratio,
            node_origin: props.node_origin,
            extent: None,
            child_extent: None,
            boundaries,
            snap_to_grid: props.snap_to_grid,
            snap_grid: props.snap_grid,
        };
        self.resizer.set_options(options);
    }

    /// Nodes that should currently show resize handles, paired with their flow-space rect.
    fn resize_targets_with(&self, store: &FlowStore<String>) -> Vec<(String, Rect)> {
        let resizer = self.node_resizer.as_ref();
        if resizer.is_none() && !store.props().nodes_resizable {
            return Vec::new();
        }
        let requested: Vec<String> = match resizer {
            Some(props) if !props.node_ids.is_empty() => props.node_ids.clone(),
            _ => store
                .node_lookup()
                .values()
                .filter(|node| node.selected)
                .map(|node| node.id.clone())
                .collect(),
        };
        let mut targets = Vec::new();
        for id in requested {
            let Some(node) = store.node_lookup().get(&id) else {
                continue;
            };
            if node.hidden {
                continue;
            }
            let visible = resizer
                .and_then(|props| props.is_visible)
                .unwrap_or(node.selected);
            if !visible {
                continue;
            }
            let dimensions = node.size();
            targets.push((
                id.clone(),
                Rect::new(
                    node.internals.position_absolute.x,
                    node.internals.position_absolute.y,
                    dimensions.width,
                    dimensions.height,
                ),
            ));
        }
        targets
    }

    fn resize_paint_specs(&self, store: &FlowStore<String>) -> Vec<ResizeControlPaint> {
        // 自定义控制点元素接管外观后不再绘制默认控制点。
        if self
            .node_resizer
            .as_ref()
            .and_then(|props| props.control_renderer.as_ref())
            .is_some()
        {
            return Vec::new();
        }
        let targets = self.resize_targets_with(store);
        if targets.is_empty() {
            return Vec::new();
        }
        let (handle_size, line_width, color) = match &self.node_resizer {
            Some(props) => (props.handle_size, props.line_width, props.color),
            None => (10.0, 4.0, None),
        };
        let theme = &store.props().theme;
        let mut paints = Vec::new();
        for (_, rect) in targets {
            for control in resize_controls(rect, handle_size, line_width) {
                let is_handle = control.kind.is_handle();
                let background = if is_handle {
                    Some(color.unwrap_or(theme.resize_background))
                } else {
                    color
                };
                let border = if is_handle {
                    Some(color.unwrap_or(theme.selection_border))
                } else {
                    color
                };
                if background.is_none() && border.is_none() {
                    continue;
                }
                paints.push(ResizeControlPaint {
                    rect: control.rect,
                    kind: control.kind,
                    background,
                    border,
                });
            }
        }
        paints
    }

    fn resize_control_at(&self, flow_point: XYPosition) -> Option<(String, String)> {
        let resizer = self.node_resizer.as_ref();
        let (handle_size, line_width) = match resizer {
            Some(props) => (props.handle_size, props.line_width),
            None => (10.0, 4.0),
        };
        for (node_id, rect) in self.resize_targets_with(&self.store) {
            let controls = resize_controls(rect, handle_size, line_width);
            if let Some(control) = find_resize_control(&controls, flow_point) {
                return Some((node_id, control.position.clone()));
            }
        }
        None
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

        self.pressed = None;
        self.node_drag_started = false;
        self.selection_drag_started = false;
        self.pane_moved = false;
        let additive = self.selection_active(event);

        if event.button == MouseButton::Left {
            if let Some((edge_id, endpoint)) = self.reconnect_handle_at(flow_point)
                && let Some(edge) = self.store.edge_lookup().get(&edge_id).cloned()
            {
                let (anchor_node, anchor_handle, anchor_type) = match endpoint {
                    ReconnectEndpoint::Source => (
                        edge.target.clone(),
                        edge.target_handle.clone(),
                        HandleType::Target,
                    ),
                    ReconnectEndpoint::Target => (
                        edge.source.clone(),
                        edge.source_handle.clone(),
                        HandleType::Source,
                    ),
                };
                self.sync_handle_options();
                let transform = self.store.transform();
                let lookup = self.store.node_lookup().clone();
                let started = self.handle.on_pointer_down(
                    XYHandleStartParams {
                        node_id: &anchor_node,
                        handle_id: anchor_handle.as_deref(),
                        handle_type: anchor_type,
                        pointer: local,
                        transform,
                    },
                    &lookup,
                );
                if started {
                    self.store.set_connection(self.handle.connection().clone());
                    self.reconnect = Some(ReconnectDrag { edge_id, endpoint });
                    self.drag_mode = DragMode::Reconnect;
                    cx.notify();
                    return;
                }
            }
            if self.connection_click_start.is_some() && self.handle_at(flow_point).is_none() {
                self.cancel_connection();
            }
            if let Some((node_id, handle_type, handle_id)) = self.handle_at(flow_point) {
                let clicked = ClickStartHandle {
                    node_id,
                    handle_id,
                    handle_type,
                };
                if self.store.props().connect_on_click
                    && let Some(start) = self.connection_click_start.clone()
                {
                    if start == clicked {
                        self.cancel_connection();
                        cx.notify();
                        return;
                    }
                    if self.finish_click_connection(&start, &clicked) {
                        cx.notify();
                        return;
                    }
                    self.cancel_connection();
                }
                self.sync_handle_options();
                let transform = self.store.transform();
                let lookup = self.store.node_lookup().clone();
                let started = self.handle.on_pointer_down(
                    XYHandleStartParams {
                        node_id: &clicked.node_id,
                        handle_id: clicked.handle_id.as_deref(),
                        handle_type: clicked.handle_type,
                        pointer: local,
                        transform,
                    },
                    &lookup,
                );
                if started {
                    let connection = self.handle.connection().clone();
                    self.store.set_connection(connection.clone());
                    if let ConnectionState::InProgress(in_progress) = &connection {
                        if let Some(handler) = self.store.props().callbacks.on_connect_start.clone()
                        {
                            handler(in_progress);
                        }
                        if self.store.props().connect_on_click {
                            self.connection_click_start = Some(ClickStartHandle {
                                node_id: in_progress.from_handle.node_id.clone(),
                                handle_id: in_progress.from_handle.id.clone(),
                                handle_type: in_progress.from_handle.handle_type,
                            });
                            if let Some(handler) =
                                self.store.props().callbacks.on_click_connect_start.clone()
                            {
                                handler(in_progress);
                            }
                        }
                    }
                    self.connection_dragged = false;
                    self.drag_mode = DragMode::Connection;
                } else {
                    self.drag_mode = DragMode::None;
                }
                cx.notify();
                return;
            }
            if let Some((node_id, control_position)) = self.resize_control_at(flow_point) {
                self.sync_resizer_options();
                if let Some(node) = self.store.node_lookup().get(&node_id).cloned() {
                    let dimensions = node.size();
                    let rect = Rect::new(
                        node.internals.position_absolute.x,
                        node.internals.position_absolute.y,
                        dimensions.width,
                        dimensions.height,
                    );
                    self.resizer.start(
                        &control_position,
                        ResizeParams::new(rect.x, rect.y, rect.width, rect.height),
                        flow_point,
                    );
                    self.resize_node_id = Some(node_id.clone());
                    self.resize_control_position = Some(control_position.clone());
                    self.drag_mode = DragMode::Resize;
                    if let Some(handler) = self
                        .node_resizer
                        .as_ref()
                        .and_then(|resizer| resizer.callbacks.on_resize_start.clone())
                    {
                        handler(&node_id, NodeResizeInfo::from_rect(rect));
                    }
                    cx.notify();
                    return;
                }
            }
            if let Some(node_id) = self.node_ids_at(flow_point) {
                let selectable = self
                    .store
                    .node_lookup()
                    .get(&node_id)
                    .map(|node| {
                        node.internals
                            .user_node
                            .is_selectable(self.store.props().elements_selectable)
                    })
                    .unwrap_or(true);
                let draggable = self
                    .store
                    .node_lookup()
                    .get(&node_id)
                    .map(|node| {
                        node.internals
                            .user_node
                            .is_draggable(self.store.props().nodes_draggable)
                    })
                    .unwrap_or(true);
                let before = self.selection_signature();
                if selectable {
                    self.select_node(&node_id, additive);
                }
                self.notify_selection_change(&before);
                self.pressed = Some(PressedTarget::Node(node_id.clone()));
                if draggable {
                    self.sync_drag_options();
                    let lookup = self.store.node_lookup().clone();
                    self.drag
                        .on_pointer_down(flow_point, Some(&node_id), &lookup);
                    self.drag_mode = DragMode::Node;
                } else {
                    self.drag_mode = DragMode::None;
                }
                cx.notify();
                return;
            }
            if let Some(edge_id) = self.edge_at(flow_point) {
                let selectable = self
                    .store
                    .edge_lookup()
                    .get(&edge_id)
                    .map(|edge| edge.selectable.unwrap_or(true))
                    .unwrap_or(true);
                let before = self.selection_signature();
                if selectable && self.store.props().elements_selectable {
                    self.select_edge(&edge_id, additive);
                }
                self.notify_selection_change(&before);
                self.pressed = Some(PressedTarget::Edge(edge_id));
                self.drag_mode = DragMode::None;
                cx.notify();
                return;
            }
            if event.modifiers.shift
                || self.selection_key_pressed
                || self.store.props().selection_on_drag
            {
                let start = flow_point;
                self.store.interaction_mut().selection_start = Some(start);
                self.store.interaction_mut().selection_end = Some(start);
                if !additive {
                    let before = self.selection_signature();
                    self.clear_selection();
                    self.notify_selection_change(&before);
                }
                if let Some(handler) = self.store.props().callbacks.on_selection_start.clone() {
                    handler(&Rect::new(start.x, start.y, 0.0, 0.0));
                }
                self.drag_mode = DragMode::Selection;
                self.pressed = Some(PressedTarget::Pane);
                cx.notify();
                return;
            }
            let before = self.selection_signature();
            self.clear_selection();
            self.notify_selection_change(&before);
            self.pressed = Some(PressedTarget::Pane);
            self.drag_mode = DragMode::Pane;
        } else if event.button == MouseButton::Right {
            if let Some(node_id) = self.node_ids_at(flow_point) {
                if let Some(handler) = self.store.props().callbacks.on_node_context_menu.clone()
                    && let Some(node) = self.user_node(&node_id)
                {
                    handler(&node);
                }
                cx.notify();
                return;
            }
            if let Some(edge_id) = self.edge_at(flow_point) {
                if let Some(handler) = self.store.props().callbacks.on_edge_context_menu.clone()
                    && let Some(edge) = self.user_edge(&edge_id)
                {
                    handler(&edge);
                }
                cx.notify();
                return;
            }
            if let Some(handler) = self.store.props().callbacks.on_pane_context_menu.clone() {
                handler();
            }
            cx.notify();
            return;
        } else {
            self.pressed = Some(PressedTarget::Pane);
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

    fn update_node_drag_at(&mut self, pointer: XYPosition) {
        let node_origin = self.store.props().node_origin;
        let node_extent = self.store.props().node_extent;
        let snap_grid = self.store.props().snap_grid;
        let snap_to_grid = self.store.props().snap_to_grid;
        let lookup = self.store.node_lookup().clone();
        let update = self
            .drag
            .on_pointer_move(pointer, &lookup, move |item, lookup| {
                let mut next =
                    XYPosition::new(pointer.x - item.distance.x, pointer.y - item.distance.y);
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
            if !self.node_drag_started {
                self.node_drag_started = true;
                if let Some(handler) = self.store.props().callbacks.on_node_drag_start.clone() {
                    handler(&self.dragging_nodes());
                }
            }
            if let Some(handler) = self.store.props().callbacks.on_node_drag.clone() {
                handler(&self.dragging_nodes());
            }
        }
        self.last_pointer = pointer;
    }

    /// Pans the viewport while dragging near an edge; returns true while still panning.
    fn auto_pan_tick(&mut self) -> bool {
        let enabled = match self.drag_mode {
            DragMode::Node => self.store.props().auto_pan_on_node_drag,
            DragMode::Connection | DragMode::Reconnect => self.store.props().auto_pan_on_connect,
            _ => false,
        };
        if !enabled {
            return false;
        }
        let size = self
            .pane_bounds
            .get()
            .map(|bounds| {
                Dimensions::new(
                    f32::from(bounds.size.width) as f64,
                    f32::from(bounds.size.height) as f64,
                )
            })
            .unwrap_or_else(|| self.store.size());
        if size.width <= 0.0 || size.height <= 0.0 {
            return false;
        }
        let speed = self.store.props().auto_pan_speed;
        let velocity = auto_pan_for_drag(self.last_pointer, size, speed, 20.0);
        if velocity.x.abs() < 0.01 && velocity.y.abs() < 0.01 {
            return false;
        }
        let viewport = self.store.viewport();
        let zoom = viewport.zoom;
        self.store.set_viewport(Viewport::new(
            viewport.x + velocity.x / zoom,
            viewport.y + velocity.y / zoom,
            zoom,
        ));
        let next = XYPosition::new(
            self.last_pointer.x + velocity.x / zoom,
            self.last_pointer.y + velocity.y / zoom,
        );
        self.last_pointer = next;
        match self.drag_mode {
            DragMode::Node => self.update_node_drag_at(next),
            DragMode::Connection | DragMode::Reconnect => self.store.update_connection(next),
            _ => {}
        }
        true
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let local = self.local_position(event.position);
        let flow_point = self.store.screen_to_flow_position(local);
        match self.drag_mode {
            DragMode::Node => {
                self.update_node_drag_at(flow_point);
                cx.notify();
            }
            DragMode::Pane => {
                let viewport_before = self.store.viewport();
                let transform_before = self.store.transform();
                self.store.pan_zoom_mut().pointer_move(local);
                self.store.sync_transform();
                if !self.pane_moved {
                    self.pane_moved = true;
                    self.notify_viewport_change(
                        viewport_before,
                        transform_before,
                        ViewportCallbackKind::Start,
                    );
                }
                self.notify_viewport_change(
                    viewport_before,
                    transform_before,
                    ViewportCallbackKind::Move,
                );
                self.last_pointer = flow_point;
                cx.notify();
            }
            DragMode::Connection | DragMode::Reconnect => {
                if self
                    .pointer_down_at
                    .map(|start| {
                        start.distance_to(local) > self.store.props().connection_drag_threshold
                    })
                    .unwrap_or(false)
                {
                    self.connection_dragged = true;
                }
                let transform = self.store.transform();
                let lookup = self.store.node_lookup().clone();
                let validation_lookup = self.store.node_lookup().clone();
                let edges = self.store.edges().to_vec();
                let custom = self.store.props().callbacks.is_valid_connection.clone();
                let nodes_connectable = self.store.props().nodes_connectable;
                let is_valid = move |connection: &Connection| {
                    let Some(source) = validation_lookup.get(&connection.source) else {
                        return false;
                    };
                    let Some(target) = validation_lookup.get(&connection.target) else {
                        return false;
                    };
                    if source.hidden
                        || target.hidden
                        || !source.internals.user_node.is_connectable(nodes_connectable)
                        || !target.internals.user_node.is_connectable(nodes_connectable)
                    {
                        return false;
                    }
                    let candidate = Edge {
                        source: connection.source.clone(),
                        target: connection.target.clone(),
                        source_handle: connection.source_handle.clone(),
                        target_handle: connection.target_handle.clone(),
                        ..Edge::default()
                    };
                    if connection_exists(&candidate, &edges) {
                        return false;
                    }
                    custom
                        .as_ref()
                        .map(|handler| handler(connection))
                        .unwrap_or(true)
                };
                if let Some(state) = self
                    .handle
                    .on_pointer_move(local, transform, &lookup, &is_valid)
                {
                    self.store
                        .set_connection(ConnectionState::InProgress(state));
                }
                cx.notify();
            }
            DragMode::Selection => {
                self.store.interaction_mut().selection_end = Some(flow_point);
                if !self.selection_drag_started {
                    self.selection_drag_started = true;
                    if let Some(handler) =
                        self.store.props().callbacks.on_selection_drag_start.clone()
                    {
                        handler(&self.selected_user_nodes());
                    }
                }
                if let Some(handler) = self.store.props().callbacks.on_selection_drag.clone() {
                    handler(&self.selected_user_nodes());
                }
                cx.notify();
            }
            DragMode::Resize => {
                if let Some(node_id) = self.resize_node_id.clone()
                    && let Some(change) = self.resizer.resize(flow_point)
                {
                    let changes = resize_change_to_node_changes::<String>(&node_id, &change, true);
                    self.store.apply_node_changes(&changes);
                    self.store.update_nodes();
                    if let Some(handler) = self
                        .node_resizer
                        .as_ref()
                        .and_then(|resizer| resizer.callbacks.on_resize.clone())
                    {
                        handler(
                            &node_id,
                            NodeResizeInfo {
                                x: change.x,
                                y: change.y,
                                width: change.width,
                                height: change.height,
                            },
                        );
                    }
                }
                self.last_pointer = flow_point;
                cx.notify();
            }
            DragMode::Minimap => {
                self.drag_minimap(event.position, cx);
            }
            DragMode::None => {
                let hovered = self.edge_at(flow_point);
                if hovered != self.hovered_edge {
                    let leave = self.store.props().callbacks.on_edge_mouse_leave.clone();
                    let enter = self.store.props().callbacks.on_edge_mouse_enter.clone();
                    if let Some(previous) = self.hovered_edge.take()
                        && let Some(handler) = leave
                        && let Some(edge) = self.user_edge(&previous)
                    {
                        handler(&edge);
                    }
                    if let Some(next) = hovered.clone()
                        && let Some(handler) = enter
                        && let Some(edge) = self.user_edge(&next)
                    {
                        handler(&edge);
                    }
                    self.hovered_edge = hovered;
                }
                if let Some(id) = self.hovered_edge.clone()
                    && let Some(handler) = self.store.props().callbacks.on_edge_mouse_move.clone()
                    && let Some(edge) = self.user_edge(&id)
                {
                    handler(&edge);
                }
                let hovered_node = self.node_ids_at(flow_point);
                if hovered_node != self.hovered_node {
                    let leave = self.store.props().callbacks.on_node_mouse_leave.clone();
                    let enter = self.store.props().callbacks.on_node_mouse_enter.clone();
                    if let Some(previous) = self.hovered_node.take()
                        && let Some(handler) = leave
                        && let Some(node) = self.user_node(&previous)
                    {
                        handler(&node);
                    }
                    if let Some(next) = hovered_node.clone()
                        && let Some(handler) = enter
                        && let Some(node) = self.user_node(&next)
                    {
                        handler(&node);
                    }
                    self.hovered_node = hovered_node;
                }
                if let Some(id) = self.hovered_node.clone()
                    && let Some(handler) = self.store.props().callbacks.on_node_mouse_move.clone()
                    && let Some(node) = self.user_node(&id)
                {
                    handler(&node);
                }
            }
        }
    }

    fn handle_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let local = self.local_position(event.position);
        match self.drag_mode {
            DragMode::Node => {
                let dragged_nodes = self.dragging_nodes();
                let (dragged, items) = self.drag.on_pointer_up();
                self.finish_drag(dragged, items);
                if dragged {
                    if let Some(handler) = self.store.props().callbacks.on_node_drag_stop.clone() {
                        handler(&dragged_nodes);
                    }
                } else if let Some(PressedTarget::Node(node_id)) = self.pressed.clone() {
                    let double = event.click_count >= 2;
                    let handler = if double {
                        self.store.props().callbacks.on_node_double_click.clone()
                    } else {
                        self.store.props().callbacks.on_node_click.clone()
                    };
                    if let Some(handler) = handler
                        && let Some(node) = self.user_node(&node_id)
                    {
                        handler(&node);
                    }
                }
            }
            DragMode::Pane => {
                let viewport_before = self.store.viewport();
                let transform_before = self.store.transform();
                let (_, moved) = self.store.pan_zoom_mut().pointer_up();
                self.store.sync_transform();
                if self.pane_moved || moved {
                    self.notify_viewport_change(
                        viewport_before,
                        transform_before,
                        ViewportCallbackKind::End,
                    );
                } else if event.click_count >= 2 && self.store.props().zoom_on_double_click {
                    let (transform, _) = self
                        .store
                        .pan_zoom_mut()
                        .double_click(local, event.modifiers.shift);
                    self.store.set_viewport(transform.viewport());
                    self.notify_viewport_change(
                        viewport_before,
                        transform_before,
                        ViewportCallbackKind::Move,
                    );
                    self.notify_viewport_change(
                        viewport_before,
                        transform_before,
                        ViewportCallbackKind::End,
                    );
                } else if event.click_count == 1
                    && let Some(handler) = self.store.props().callbacks.on_pane_click.clone()
                {
                    handler();
                }
            }
            DragMode::Reconnect => {
                let state = self.handle.on_pointer_up();
                if let ConnectionState::InProgress(in_progress) = state
                    && let Some(connection) = connection_from_progress(&in_progress)
                    && self.connection_is_valid(&connection)
                    && let Some(drag) = self.reconnect.clone()
                    && let Some(old_edge) = self.store.edge_lookup().get(&drag.edge_id).cloned()
                {
                    let edges = reconnect_edge(
                        &old_edge,
                        &connection,
                        self.store.edges(),
                        ReconnectEdgeOptions::default(),
                    );
                    if let Ok(edges) = edges {
                        self.store.set_edges(edges);
                        self.store.update_nodes();
                    }
                }
                self.store
                    .set_connection(ConnectionState::Idle(Default::default()));
            }
            DragMode::Connection => {
                let moved = self
                    .pointer_down_at
                    .map(|start| {
                        start.distance_to(local) > self.store.props().connection_drag_threshold
                    })
                    .unwrap_or(false);
                if self.store.props().connect_on_click
                    && !self.connection_dragged
                    && !moved
                    && let ConnectionState::InProgress(in_progress) = self.store.connection_state()
                    && in_progress.to_node.is_none()
                {
                    // Keep the connection pending until the next handle click.
                    self.drag_mode = DragMode::None;
                    self.pressed = None;
                    self.node_drag_started = false;
                    self.selection_drag_started = false;
                    self.pane_moved = false;
                    cx.notify();
                    return;
                }
                let state = self.handle.on_pointer_up();
                let mut final_state = None;
                if let ConnectionState::InProgress(in_progress) = state {
                    if let Some(target) = &in_progress.to_node {
                        let connection = Connection {
                            source: in_progress.from_handle.node_id.clone(),
                            source_handle: in_progress.from_handle.id.clone(),
                            target: target.id.clone(),
                            target_handle: in_progress
                                .to_handle
                                .as_ref()
                                .and_then(|handle| handle.id.clone()),
                        };
                        let is_valid = self.connection_is_valid(&connection);
                        if is_valid {
                            let edge = Edge {
                                source: connection.source.clone(),
                                target: connection.target.clone(),
                                source_handle: connection.source_handle.clone(),
                                target_handle: connection.target_handle.clone(),
                                ..Edge::default()
                            };
                            if let Some(edges) = add_edge(
                                edge,
                                self.store.edges(),
                                self.store.props().default_edge_options.as_ref(),
                            ) {
                                self.store.set_edges(edges);
                                self.store.update_nodes();
                                if let Some(handler) =
                                    self.store.props().callbacks.on_connect.clone()
                                {
                                    handler(&in_progress);
                                }
                            }
                        }
                        let mut completed = in_progress.clone();
                        completed.is_valid = Some(is_valid);
                        final_state = Some(completed);
                    } else {
                        let mut completed = in_progress.clone();
                        completed.is_valid = Some(false);
                        final_state = Some(completed);
                    }
                }
                self.store
                    .set_connection(ConnectionState::Idle(Default::default()));
                if let Some(handler) = self.store.props().callbacks.on_connect_end.clone()
                    && let Some(state) = final_state
                {
                    handler(&state);
                }
            }
            DragMode::Selection => {
                let before = self.selection_signature();
                let rect = self.store.interaction().selection_rect();
                let has_area = rect
                    .map(|rect| rect.width.abs() > 0.0 || rect.height.abs() > 0.0)
                    .unwrap_or(false);
                if let Some(rect) = rect
                    && has_area
                {
                    let changes = self.store.selection_changes_for_rect(rect);
                    if self.store.has_default_nodes() {
                        self.store.trigger_node_changes(changes);
                    } else {
                        self.store.apply_node_changes(&changes);
                        self.store.trigger_node_changes(changes);
                    }
                    self.store.update_nodes();
                    if let Some(handler) = self.store.props().callbacks.on_selection_end.clone() {
                        handler(&rect);
                    }
                    if self.selection_drag_started
                        && let Some(handler) =
                            self.store.props().callbacks.on_selection_drag_stop.clone()
                    {
                        handler(&self.selected_user_nodes());
                    }
                    self.notify_selection_change(&before);
                } else if event.click_count == 1
                    && let Some(handler) = self.store.props().callbacks.on_pane_click.clone()
                {
                    handler();
                }
                self.store.interaction_mut().selection_start = None;
                self.store.interaction_mut().selection_end = None;
            }
            DragMode::Resize => {
                if let Some(node_id) = self.resize_node_id.clone()
                    && let Some(change) = self.resizer.end()
                {
                    let changes = resize_change_to_node_changes::<String>(&node_id, &change, false);
                    self.store.apply_node_changes(&changes);
                    self.store.update_nodes();
                    if let Some(handler) = self
                        .node_resizer
                        .as_ref()
                        .and_then(|resizer| resizer.callbacks.on_resize_end.clone())
                    {
                        handler(
                            &node_id,
                            NodeResizeInfo {
                                x: change.x,
                                y: change.y,
                                width: change.width,
                                height: change.height,
                            },
                        );
                    }
                }
                self.resize_node_id = None;
                self.resize_control_position = None;
            }
            DragMode::Minimap => {}
            DragMode::None => {
                if let Some(PressedTarget::Edge(edge_id)) = self.pressed.clone() {
                    let double = event.click_count >= 2;
                    let handler = if double {
                        self.store.props().callbacks.on_edge_double_click.clone()
                    } else {
                        self.store.props().callbacks.on_edge_click.clone()
                    };
                    if let Some(handler) = handler
                        && let Some(edge) = self.user_edge(&edge_id)
                    {
                        handler(&edge);
                    }
                }
            }
        }
        self.pressed = None;
        self.drag_mode = DragMode::None;
        self.reconnect = None;
        self.node_drag_started = false;
        self.selection_drag_started = false;
        self.pane_moved = false;
        cx.notify();
    }

    fn handle_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (delta_y, delta_mode) = match event.delta {
            ScrollDelta::Pixels(delta) => (f32::from(delta.y) as f64, 1),
            ScrollDelta::Lines(delta) => (delta.y as f64, 0),
        };

        self.sync_pan_zoom_options();
        self.pan_zoom.sync_viewport(self.store.viewport());
        let viewport_before = self.store.viewport();
        let transform_before = self.store.transform();

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
            self.notify_viewport_change(
                viewport_before,
                transform_before,
                ViewportCallbackKind::Move,
            );
        }
        cx.notify();
    }

    fn minimap_local_point(
        &self,
        props: &MinimapProps,
        position: gpui_kit::Point<Pixels>,
    ) -> Option<XYPosition> {
        let bounds = self.pane_bounds.get()?;
        let origin = minimap_origin(bounds, props);
        Some(XYPosition::new(
            f32::from(position.x) as f64 - origin.x,
            f32::from(position.y) as f64 - origin.y,
        ))
    }

    fn handle_minimap_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let Some(props) = self.minimap.clone() else {
            return;
        };
        let Some(local) = self.minimap_local_point(&props, event.position) else {
            return;
        };
        let viewport = self.store.viewport();
        let size = self.store.size();
        let render = minimap_render(
            self.store.node_lookup(),
            self.store.transform(),
            size.width,
            size.height,
            &props,
        );
        let scale = props.width / render.layout.width.max(1.0);
        let viewbox_x =
            render.layout.view_x - (render.layout.width - render.layout.view_width) / 2.0;
        let viewbox_y =
            render.layout.view_y - (render.layout.height - render.layout.view_height) / 2.0;
        let flow = XYPosition::new(viewbox_x + local.x / scale, viewbox_y + local.y / scale);
        if let Some(handler) = self.minimap_callbacks.on_click.clone() {
            handler(flow);
        }
        if let Some(handler) = self.minimap_callbacks.on_node_click.clone()
            && let Some(hit) = render.nodes.iter().find(|node| {
                flow.x >= node.x
                    && flow.x <= node.x + node.width
                    && flow.y >= node.y
                    && flow.y <= node.y + node.height
            })
            && let Some(node) = self.store.node_lookup().get(&hit.id)
        {
            handler(&node.internals.user_node);
        }
        if !props.pannable {
            return;
        }
        let viewport_before = viewport;
        let transform_before = self.store.transform();
        let zoom = viewport.zoom;
        self.store.set_viewport(Viewport::new(
            flow.x - size.width / zoom / 2.0,
            flow.y - size.height / zoom / 2.0,
            zoom,
        ));
        self.last_pointer = local;
        self.drag_mode = DragMode::Minimap;
        self.notify_viewport_change(
            viewport_before,
            transform_before,
            ViewportCallbackKind::Move,
        );
        cx.notify();
    }

    fn drag_minimap(&mut self, position: gpui_kit::Point<Pixels>, cx: &mut Context<Self>) {
        let Some(props) = self.minimap.clone() else {
            return;
        };
        let Some(local) = self.minimap_local_point(&props, position) else {
            return;
        };
        let delta = XYPosition::new(local.x - self.last_pointer.x, local.y - self.last_pointer.y);
        if delta.x == 0.0 && delta.y == 0.0 {
            return;
        }
        let delta = if props.inverse_pan {
            XYPosition::new(-delta.x, -delta.y)
        } else {
            delta
        };
        let size = self.store.size();
        let viewport_before = self.store.viewport();
        let transform_before = self.store.transform();
        let render = minimap_render(
            self.store.node_lookup(),
            self.store.transform(),
            size.width,
            size.height,
            &props,
        );
        let next =
            crate::ui::minimap::minimap_drag_viewport(self.store.viewport(), delta, &render.layout);
        self.store.set_viewport(next);
        self.last_pointer = local;
        self.notify_viewport_change(
            viewport_before,
            transform_before,
            ViewportCallbackKind::Move,
        );
        cx.notify();
    }

    fn handle_minimap_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.drag_mode != DragMode::Minimap {
            return;
        }
        cx.stop_propagation();
        self.drag_minimap(event.position, cx);
    }

    fn handle_minimap_mouse_up(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.drag_mode == DragMode::Minimap {
            cx.stop_propagation();
            self.drag_mode = DragMode::None;
            cx.notify();
        }
    }

    fn handle_minimap_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let Some(props) = self.minimap.clone() else {
            return;
        };
        if !props.zoomable {
            return;
        }
        let delta = match event.delta {
            ScrollDelta::Pixels(delta) => -(f32::from(delta.y) as f64) / 100.0,
            ScrollDelta::Lines(delta) => -(delta.y as f64),
        };
        if delta == 0.0 {
            return;
        }
        let viewport_before = self.store.viewport();
        let transform_before = self.store.transform();
        let props_bound = self.store.props();
        let next = crate::ui::minimap::minimap_zoom_viewport(
            self.store.viewport(),
            delta,
            props.zoom_step,
            props_bound.min_zoom,
            props_bound.max_zoom,
        );
        self.store.set_viewport(next);
        self.notify_viewport_change(
            viewport_before,
            transform_before,
            ViewportCallbackKind::Move,
        );
        cx.notify();
    }
}

impl Focusable for Flow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Flow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().clone();
        if self.auto_pan_tick() {
            window.request_animation_frame();
            cx.notify();
        }
        let pattern = self.pattern;
        let node_content_renderer = self.node_content_renderer.clone();
        let edge_content_renderer = self.edge_content_renderer.clone();
        let pane_bounds = self.pane_bounds.clone();
        let dash_offset = (self.started_at.elapsed().as_secs_f32() * 24.0) % 10.0;
        if self
            .store
            .edge_lookup()
            .values()
            .any(|edge| edge.animated && !edge.hidden)
        {
            window.request_animation_frame();
        }
        let node_toolbars = self.render_node_toolbars(window, cx);
        let edge_elements = self.render_edge_elements(window, cx);
        let node_wrapper_elements = self.render_node_wrapper_elements(window);
        let default_handle_elements = self.render_default_handle_elements();
        let node_elements = self.render_node_elements(window, cx);
        let handle_elements = self.render_handle_elements(window, cx);
        let resize_control_elements = self.render_resize_control_elements(window, cx);
        let connection_line_element = self.render_connection_line(window, cx);
        let edge_toolbar = self.render_edge_toolbar(window, cx);
        let viewport_portal = self.render_viewport_portal(window, cx);
        let edge_labels = self.render_edge_labels(window, cx);
        let panels = self.render_panels(window, cx);
        let attribution = self.render_attribution();

        div()
            .size_full()
            .relative()
            .track_focus(&self.focus_handle)
            .child(
                canvas(
                    move |bounds, _window, cx| {
                        pane_bounds.set(Some(bounds));
                        let mut store = entity.read(cx).store.clone_snapshot();
                        let width = f32::from(bounds.size.width) as f64;
                        let height = f32::from(bounds.size.height) as f64;
                        if store.size().width != width || store.size().height != height {
                            store.set_size(width, height);
                            store.update_nodes();
                        }
                        let mut data = build_render_data(&store, bounds);
                        data.pattern = pattern;
                        data.attach_custom_renderers(
                            &store,
                            node_content_renderer.clone(),
                            edge_content_renderer.clone(),
                        );
                        data.resize_controls = entity.read(cx).resize_paint_specs(&store);
                        data.focus_rect = entity.read(cx).focus_rect(&store);
                        data.hovered_edge = entity.read(cx).hovered_edge.clone();
                        data.dash_offset = dash_offset;
                        // 元素层节点组件启用时，画布不再绘制默认节点底、边框与标签。
                        data.skip_node_bodies = entity.read(cx).node_components();
                        // 自定义连接线组件接管后不再绘制默认连接线。
                        if entity.read(cx).connection_line_renderer.is_some() {
                            data.connection_path = None;
                        }
                        data
                    },
                    |_bounds, data, window, cx| {
                        paint_background(window, &data);
                        paint_edges(window, &data, cx);
                        paint_nodes(window, &data, cx);
                        paint_resize_controls(window, &data);
                        paint_focus_ring(window, &data);
                        paint_connection(window, &data);
                        paint_selection(window, &data);
                        paint_reconnect_handles(window, &data);
                    },
                )
                .size_full(),
            )
            .children(edge_elements)
            .children(node_wrapper_elements)
            .children(default_handle_elements)
            .children(node_elements)
            .children(handle_elements)
            .children(resize_control_elements)
            .children(connection_line_element)
            .children(viewport_portal)
            .children(edge_labels)
            .children(panels)
            .children(
                self.controls
                    .clone()
                    .map(|props| self.render_controls(props, window, cx)),
            )
            .children(
                self.minimap
                    .clone()
                    .map(|props| self.render_minimap(props, window, cx)),
            )
            .children(node_toolbars)
            .children(edge_toolbar)
            .children(attribution)
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
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.on_key_down(event, window, cx);
            }))
            .on_key_up(cx.listener(|this, event: &KeyUpEvent, window, cx| {
                this.on_key_up(event, window, cx);
            }))
    }
}

#[cfg(test)]
mod focus_tests {
    use super::next_focus_index;

    #[test]
    fn focus_index_cycles_forward_and_backward() {
        assert_eq!(next_focus_index(None, 3, false), Some(0));
        assert_eq!(next_focus_index(Some(2), 3, false), Some(0));
        assert_eq!(next_focus_index(None, 3, true), Some(2));
        assert_eq!(next_focus_index(Some(0), 3, true), Some(2));
        assert_eq!(next_focus_index(None, 0, false), None);
    }
}
