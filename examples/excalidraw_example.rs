use gpui_kit::*;
use gpui_xyflow::core::edge::{Edge, MarkerType};
use gpui_xyflow::core::geometry::{Position, XYPosition};
use gpui_xyflow::core::node::{Handle, HandleType, Node};
use gpui_xyflow::ui::flow::Flow;
use gpui_xyflow::ui::store::FlowProps;
use gpui_xyflow::ui::theme::FlowTheme;

struct Demo {
    flow: Entity<Flow>,
}

impl Demo {
    fn new(cx: &mut App) -> Self {
        let flow = cx.new(|cx| {
            let mut flow = Flow::with_props(
                FlowProps {
                    theme: FlowTheme::light(),
                    snap_to_grid: true,
                    snap_grid: [10.0, 10.0],
                    ..FlowProps::default()
                },
                cx,
            );
            flow.set_nodes(build_nodes());
            flow.set_edges(build_edges());
            flow
        });
        Demo { flow }
    }
}

impl Render for Demo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0xf7f7f7))
            .child(div().size_full().child(self.flow.clone()))
    }
}

fn build_nodes() -> Vec<Node<String>> {
    let mut nodes = Vec::new();

    let mut input = Node::new("1", XYPosition::new(120.0, 120.0), "input".to_string())
        .with_type("input")
        .with_size(150.0, 54.0)
        .with_measured(150.0, 54.0)
        .with_source_position(Position::Bottom);
    input.handles = vec![Handle::new("1", HandleType::Source, Position::Bottom, 0.5, 1.0)];
    nodes.push(input);

    let mut middle = Node::new("2", XYPosition::new(400.0, 280.0), "default".to_string())
        .with_type("default")
        .with_size(160.0, 54.0)
        .with_measured(160.0, 54.0)
        .with_source_position(Position::Bottom)
        .with_target_position(Position::Top);
    middle.handles = vec![
        Handle::new("2", HandleType::Target, Position::Top, 0.5, 0.0),
        Handle::new("2", HandleType::Source, Position::Bottom, 0.5, 1.0),
    ];
    nodes.push(middle);

    let mut output = Node::new("3", XYPosition::new(660.0, 440.0), "output".to_string())
        .with_type("output")
        .with_size(150.0, 54.0)
        .with_measured(150.0, 54.0)
        .with_target_position(Position::Top);
    output.handles = vec![Handle::new("3", HandleType::Target, Position::Top, 0.5, 0.0)];
    nodes.push(output);

    nodes
}

fn build_edges() -> Vec<Edge<String>> {
    let mut first = Edge::default();
    first.id = "e1-2".to_string();
    first.source = "1".to_string();
    first.target = "2".to_string();
    first.marker_end = Some(gpui_xyflow::core::edge::EdgeMarkerType::Marker(
        gpui_xyflow::core::edge::EdgeMarker::new(MarkerType::ArrowClosed),
    ));

    let mut second = Edge::default();
    second.id = "e2-3".to_string();
    second.source = "2".to_string();
    second.target = "3".to_string();
    second.marker_end = Some(gpui_xyflow::core::edge::EdgeMarkerType::Marker(
        gpui_xyflow::core::edge::EdgeMarker::new(MarkerType::ArrowClosed),
    ));

    vec![first, second]
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);

        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|cx| {
                let demo = Demo::new(cx);
                demo
            })
        })
        .expect("failed to open window");
    });
}
