use gpui_kit::*;
use gpui_xyflow::core::edge::{Edge, EdgeMarker, EdgeMarkerType, MarkerType};
use gpui_xyflow::core::geometry::{Position, XYPosition};
use gpui_xyflow::core::node::{Handle, HandleType, Node};
use gpui_xyflow::ui::background::BackgroundVariant;
use gpui_xyflow::ui::controls::{ControlButton, ControlsProps};
use gpui_xyflow::ui::flow::Flow;
use gpui_xyflow::ui::store::FlowProps;
use gpui_xyflow::ui::theme::FlowTheme;

struct Gallery {
    flow: Entity<Flow>,
    zoom_label: SharedString,
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
                    ..FlowProps::default()
                },
                cx,
            );
            flow.set_pattern(BackgroundVariant::Dots);
            flow.set_nodes(gallery_nodes(dark));
            flow.set_edges(gallery_edges());
            flow
        });
        Gallery {
            flow,
            zoom_label: SharedString::from("100%"),
        }
    }

    fn sync_zoom(&mut self, cx: &mut App) {
        let zoom = cx.read_entity(&self.flow, |flow: &Flow, _| flow.store().get_zoom());
        self.zoom_label = SharedString::from(format!("{}%", (zoom * 100.0).round() as i64));
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let flow = self.flow.clone();
        let zoom_label = self.zoom_label.clone();
        let entity = cx.entity().clone();
        let controls = ControlsProps::default();

        div()
            .size_full()
            .relative()
            .child(flow.clone())
            .child(
                div()
                    .absolute()
                    .top(px(12.0))
                    .left(px(12.0))
                    .flex()
                    .gap(px(6.0))
                    .child(
                        div()
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(rgb(0xffffff))
                            .text_size(px(12.0))
                            .text_color(rgb(0x333333))
                            .child(zoom_label),
                    )
                    .child(
                        div()
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(rgb(0xffffff))
                            .text_size(px(12.0))
                            .text_color(rgb(0x333333))
                            .child("drag nodes, scroll to zoom, drag pane to pan"),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .bottom(px(12.0))
                    .left(px(12.0))
                    .flex()
                    .gap(px(6.0))
                    .children(controls.buttons().into_iter().map(|button| {
                        let flow = flow.clone();
                        let entity = entity.clone();
                        div()
                            .w(px(controls.button_size))
                            .h(px(controls.button_size))
                            .rounded(px(4.0))
                            .bg(rgb(0xfefefe))
                            .border_1()
                            .border_color(rgb(0xeeeeee))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(13.0))
                            .text_color(rgb(0x333333))
                            .cursor_pointer()
                            .child(button.label())
                            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                                match button {
                                    ControlButton::ZoomIn => {
                                        flow.update(cx, |flow, _| flow.store_mut().zoom_in());
                                    }
                                    ControlButton::ZoomOut => {
                                        flow.update(cx, |flow, _| flow.store_mut().zoom_out());
                                    }
                                    ControlButton::FitView => {
                                        flow.update(cx, |flow, _| {
                                            flow.store_mut().set_size(1200.0, 800.0);
                                            flow.store_mut().fit_view();
                                        });
                                    }
                                    ControlButton::Interactive => {}
                                }
                                entity.update(cx, |this, cx| {
                                    this.sync_zoom(cx);
                                    cx.notify();
                                });
                                window.request_animation_frame();
                            })
                    })),
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
    input.handles = vec![handle("input", HandleType::Source, Position::Bottom, 0.5, 1.0)];
    nodes.push(input);

    let mut bezier = Node::new("bezier", XYPosition::new(320.0, 120.0), "bezier".to_string())
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

    let mut smooth = Node::new("smooth", XYPosition::new(600.0, 260.0), "smooth".to_string())
        .with_type("default")
        .with_size(170.0, 54.0)
        .with_measured(170.0, 54.0)
        .with_target_position(Position::Left);
    smooth.handles = vec![handle("smooth", HandleType::Target, Position::Left, 0.0, 0.5)];
    nodes.push(smooth);

    let mut output = Node::new("output", XYPosition::new(600.0, 120.0), "output".to_string())
        .with_type("output")
        .with_size(150.0, 54.0)
        .with_measured(150.0, 54.0)
        .with_target_position(Position::Left);
    output.handles = vec![handle("output", HandleType::Target, Position::Left, 0.0, 0.5)];
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

    let mut e1 = Edge::default();
    e1.id = "input-bezier".to_string();
    e1.source = "input".to_string();
    e1.target = "bezier".to_string();
    e1.marker_end = marker();

    let mut e2 = Edge::default();
    e2.id = "bezier-output".to_string();
    e2.source = "bezier".to_string();
    e2.target = "output".to_string();
    e2.marker_end = marker();
    e2.label = Some("bezier".to_string());

    let mut e3 = Edge::default();
    e3.id = "bezier-step".to_string();
    e3.source = "bezier".to_string();
    e3.target = "step".to_string();
    e3.edge_type = Some("step".to_string());
    e3.marker_end = marker();

    let mut e4 = Edge::default();
    e4.id = "step-smooth".to_string();
    e4.source = "step".to_string();
    e4.target = "smooth".to_string();
    e4.edge_type = Some("smoothstep".to_string());
    e4.marker_end = marker();
    e4.label = Some("smoothstep".to_string());

    let mut e5 = Edge::default();
    e5.id = "output-smooth".to_string();
    e5.source = "output".to_string();
    e5.target = "smooth".to_string();
    e5.edge_type = Some("straight".to_string());
    e5.marker_end = marker();

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
