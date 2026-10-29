use gpui_kit::*;
use gpui_xyflow::{
    core::{
        edge::{Edge, EdgeMarker, EdgeMarkerType, MarkerType},
        geometry::{PanelPosition, Position, Transform, XYPosition},
        node::{Handle, HandleType, Node},
    },
    ui::{
        background::BackgroundVariant,
        controls::ControlsProps,
        flow::{ConnectionLineProps, ConnectionLineRenderer, Flow, HandleRenderer, PanelItem},
        minimap::MinimapProps,
        resizer::{NodeResizerProps, ResizeControl, ResizeControlRenderer},
        store::FlowProps,
        theme::FlowTheme,
        toolbars::NodeToolbarProps,
    },
};

struct Gallery {
    flow: Entity<Flow>,
}

impl Gallery {
    fn build(cx: &mut App, dark: bool) -> Self {
        let theme = FlowTheme::of(dark);
        let flow = cx.new(|cx| {
            let mut flow = Flow::with_props(
                FlowProps {
                    theme: theme.clone(),
                    snap_to_grid: true,
                    snap_grid: [10.0, 10.0],
                    elevate_nodes_on_select: true,
                    nodes_focusable: true,
                    edges_focusable: true,
                    ..FlowProps::default()
                },
                cx,
            );
            flow.set_pattern(BackgroundVariant::Dots);
            flow.set_nodes(gallery_nodes(dark));
            flow.set_edges(gallery_edges());
            // 启用元素层节点组件：节点底、边框与标签由 GPUI 元素绘制。
            flow.set_node_components(true);
            flow.set_controls(Some(ControlsProps::default()));
            flow.set_minimap(Some(MinimapProps {
                margin: 40.0,
                pannable: true,
                zoomable: true,
                ..Default::default()
            }));
            let handle_element: HandleRenderer = std::sync::Arc::new(
                |_node: &Node<String>,
                 handle: &Handle,
                 _bounds: Bounds<Pixels>,
                 _zoom: f64,
                 _window: &mut Window,
                 _cx: &mut App| {
                    let accent = match handle.handle_type {
                        HandleType::Source => rgb(0x3b82f6),
                        HandleType::Target => rgb(0x10b981),
                    };
                    div()
                        .size_full()
                        .rounded(px(4.0))
                        .bg(rgb(0xffffff))
                        .border_1()
                        .border_color(accent)
                        .into_any_element()
                },
            );
            flow.set_handle_renderer(Some(handle_element));
            let edge_element: gpui_xyflow::ui::flow::EdgeElementRenderer = std::sync::Arc::new(
                |_edge: &Edge<String>,
                 _bounds: Bounds<Pixels>,
                 _transform: Transform,
                 _window: &mut Window,
                 _cx: &mut App| {
                    div()
                        .absolute()
                        .left(relative(0.5))
                        .top(relative(0.5))
                        .ml(px(-2.0))
                        .mt(px(-2.0))
                        .w(px(4.0))
                        .h(px(4.0))
                        .rounded(px(2.0))
                        .bg(rgb(0x3b82f6))
                        .into_any_element()
                },
            );
            flow.set_edge_element_renderer(Some(edge_element));
            let connection_line: ConnectionLineRenderer = std::sync::Arc::new(
                |props: &ConnectionLineProps,
                 _bounds: Bounds<Pixels>,
                 transform: Transform,
                 _window: &mut Window,
                 _cx: &mut App| {
                    let to = transform.apply(XYPosition::new(props.to_x, props.to_y));
                    let color = if props.connection_status == Some(false) {
                        rgb(0xef4444)
                    } else {
                        rgb(0x3b82f6)
                    };
                    div()
                        .absolute()
                        .left(px(to.x as f32 - 4.0))
                        .top(px(to.y as f32 - 4.0))
                        .w(px(8.0))
                        .h(px(8.0))
                        .rounded(px(4.0))
                        .bg(color)
                        .into_any_element()
                },
            );
            flow.set_connection_line_renderer(Some(connection_line));
            let resize_control: ResizeControlRenderer = std::sync::Arc::new(
                |control: &ResizeControl,
                 _bounds: Bounds<Pixels>,
                 _zoom: f64,
                 _window: &mut Window,
                 _cx: &mut App| {
                    let color = if control.kind.is_handle() {
                        rgb(0x3b82f6)
                    } else {
                        rgba(0x3b82f633)
                    };
                    div().size_full().bg(color).into_any_element()
                },
            );
            flow.set_node_resizer(Some(
                NodeResizerProps {
                    min_width: 60.0,
                    min_height: 40.0,
                    ..Default::default()
                }
                .with_control_renderer(resize_control),
            ));
            flow.set_panels(vec![PanelItem::new(
                "hint",
                PanelPosition::TopLeft,
                std::sync::Arc::new(|_window, _cx| {
                    div()
                        .px(px(10.0))
                        .py(px(6.0))
                        .rounded_md()
                        .bg(rgba(0xffffffcc))
                        .text_size(px(12.0))
                        .child("拖拽节点 · 滚轮缩放 · 点击手柄连线")
                        .into_any_element()
                }),
            )]);

            let weak_flow = cx.weak_entity();
            flow.set_node_toolbar(
                Some(NodeToolbarProps::default()),
                Some(std::sync::Arc::new(
                    move |node: &Node<String>, _window: &mut Window, _cx: &mut App| {
                        let weak_flow = weak_flow.clone();
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(6.0))
                            .bg(rgb(0xffffff))
                            .border_1()
                            .border_color(rgb(0xdddddd))
                            .text_size(px(11.0))
                            .child(node.id.clone())
                            .child(
                                div()
                                    .id(format!("gallery-toolbar-clear-{}", node.id))
                                    .px(px(4.0))
                                    .cursor_pointer()
                                    .child("✕")
                                    .on_click(move |_, _, cx| {
                                        if let Some(flow) = weak_flow.upgrade() {
                                            flow.update(cx, |flow, cx| {
                                                flow.store_mut().unselect_nodes_and_edges();
                                                cx.notify();
                                            });
                                        }
                                    }),
                            )
                            .into_any_element()
                    },
                )),
            );
            flow
        });
        Gallery { flow }
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let flow = self.flow.clone();

        div().size_full().relative().child(flow).child(
            div()
                .absolute()
                .top(px(12.0))
                .left(px(12.0))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .bg(rgb(0xffffff))
                .text_size(px(12.0))
                .text_color(rgb(0x333333))
                .child("drag nodes, scroll to zoom, drag pane to pan, Del deletes selection"),
        )
    }
}

fn handle(node_id: &str, handle_type: HandleType, position: Position, x: f64, y: f64) -> Handle {
    Handle::new(node_id, handle_type, position, x, y)
}

fn gallery_nodes(dark: bool) -> Vec<Node<String>> {
    let _ = dark;
    let mut nodes = Vec::new();

    let mut input = Node::new("input", XYPosition::new(80.0, 120.0), "input".to_string())
        .with_type("input")
        .with_size(150.0, 54.0)
        .with_measured(150.0, 54.0)
        .with_source_position(Position::Bottom);
    input.handles = vec![handle(
        "input",
        HandleType::Source,
        Position::Bottom,
        0.5,
        1.0,
    )];
    nodes.push(input);

    let mut bezier = Node::new(
        "bezier",
        XYPosition::new(320.0, 120.0),
        "bezier".to_string(),
    )
    .with_type("default")
    .with_size(160.0, 54.0)
    .with_measured(160.0, 54.0)
    .with_source_position(Position::Right)
    .with_target_position(Position::Left);
    bezier.handles = vec![
        handle("bezier", HandleType::Target, Position::Left, 0.0, 0.5),
        handle("bezier", HandleType::Source, Position::Right, 1.0, 0.5),
    ];
    nodes.push(bezier);

    let mut step = Node::new("step", XYPosition::new(320.0, 260.0), "step".to_string())
        .with_type("default")
        .with_size(160.0, 54.0)
        .with_measured(160.0, 54.0)
        .with_source_position(Position::Bottom)
        .with_target_position(Position::Top);
    step.handles = vec![
        handle("step", HandleType::Target, Position::Top, 0.5, 0.0),
        handle("step", HandleType::Source, Position::Bottom, 0.5, 1.0),
    ];
    nodes.push(step);

    let mut smooth = Node::new(
        "smooth",
        XYPosition::new(600.0, 260.0),
        "smooth".to_string(),
    )
    .with_type("default")
    .with_size(170.0, 54.0)
    .with_measured(170.0, 54.0)
    .with_target_position(Position::Left);
    smooth.handles = vec![handle(
        "smooth",
        HandleType::Target,
        Position::Left,
        0.0,
        0.5,
    )];
    nodes.push(smooth);

    let mut output = Node::new(
        "output",
        XYPosition::new(600.0, 120.0),
        "output".to_string(),
    )
    .with_type("output")
    .with_size(150.0, 54.0)
    .with_measured(150.0, 54.0)
    .with_target_position(Position::Left);
    output.handles = vec![handle(
        "output",
        HandleType::Target,
        Position::Left,
        0.0,
        0.5,
    )];
    nodes.push(output);

    let mut group = Node::new("group", XYPosition::new(60.0, 400.0), "group".to_string())
        .with_type("group")
        .with_size(720.0, 180.0)
        .with_measured(720.0, 180.0);
    group.handles = Vec::new();
    group.z_index = Some(-1.0);
    nodes.push(group);

    nodes
}

fn gallery_edges() -> Vec<Edge<String>> {
    let marker = || {
        Some(EdgeMarkerType::Marker(EdgeMarker::new(
            MarkerType::ArrowClosed,
        )))
    };

    let e1 = Edge {
        id: "input-bezier".to_string(),
        source: "input".to_string(),
        target: "bezier".to_string(),
        marker_end: marker(),
        ..Edge::default()
    };

    let e2 = Edge {
        id: "bezier-output".to_string(),
        source: "bezier".to_string(),
        target: "output".to_string(),
        marker_end: marker(),
        label: Some("bezier".to_string()),
        ..Edge::default()
    };

    let e3 = Edge {
        id: "bezier-step".to_string(),
        source: "bezier".to_string(),
        target: "step".to_string(),
        edge_type: Some("step".to_string()),
        marker_end: marker(),
        ..Edge::default()
    };

    let e4 = Edge {
        id: "step-smooth".to_string(),
        source: "step".to_string(),
        target: "smooth".to_string(),
        edge_type: Some("smoothstep".to_string()),
        animated: true,
        marker_end: marker(),
        label: Some("smoothstep".to_string()),
        ..Edge::default()
    };

    let e5 = Edge {
        id: "output-smooth".to_string(),
        source: "output".to_string(),
        target: "smooth".to_string(),
        edge_type: Some("straight".to_string()),
        marker_end: marker(),
        ..Edge::default()
    };

    vec![e1, e2, e3, e4, e5]
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);

        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|cx| Gallery::build(cx, false))
        })
        .expect("failed to open window");
    });
}
