use gpui_kit::{
    div, App, Context, IntoElement, ParentElement, Render, Styled, Window, WindowOptions,
};
use gpui_xyflow::core::edge::{Edge, EdgeMarker, EdgeMarkerType, MarkerType};
use gpui_xyflow::core::geometry::{Position, XYPosition};
use gpui_xyflow::core::node::{Handle, HandleType, Node};
use gpui_xyflow::ui::background::BackgroundVariant;
use gpui_xyflow::ui::flow::Flow;
use gpui_xyflow::ui::store::{FlowProps, FlowStore};
use gpui_xyflow::ui::theme::FlowTheme;

fn sample_nodes() -> Vec<Node<String>> {
    let mut a = Node::new("a", XYPosition::new(100.0, 100.0), "a".to_string())
        .with_type("input")
        .with_size(120.0, 50.0)
        .with_measured(120.0, 50.0)
        .with_source_position(Position::Bottom);
    a.handles = vec![Handle::new("a", HandleType::Source, Position::Bottom, 0.5, 1.0)];

    let mut b = Node::new("b", XYPosition::new(400.0, 300.0), "b".to_string())
        .with_type("output")
        .with_size(120.0, 50.0)
        .with_measured(120.0, 50.0)
        .with_target_position(Position::Top);
    b.handles = vec![Handle::new("b", HandleType::Target, Position::Top, 0.5, 0.0)];

    vec![a, b]
}

fn sample_edges() -> Vec<Edge<String>> {
    let mut edge = Edge::default();
    edge.id = "a-b".to_string();
    edge.source = "a".to_string();
    edge.target = "b".to_string();
    edge.marker_end = Some(EdgeMarkerType::Marker(EdgeMarker::new(MarkerType::ArrowClosed)));
    vec![edge]
}

struct FlowHost {
    flow: gpui_kit::Entity<Flow>,
}

impl Render for FlowHost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.flow.clone())
    }
}

#[gpui_kit::test]
fn flow_opens_a_window_and_renders(cx: &mut gpui_kit::TestAppContext) {
    use gpui_kit::AppContext;

    cx.update(|cx| {
        let _window = cx.open_window(WindowOptions::default(), |_window, cx| {
            let flow = cx.new(|cx| {
                let mut flow = Flow::with_props(FlowProps::default(), cx);
                flow.set_nodes(sample_nodes());
                flow.set_edges(sample_edges());
                flow
            });
            cx.new(|_cx| FlowHost { flow })
        });
        assert_eq!(cx.windows().len(), 1, "expected exactly one window");
    });
}

#[gpui_kit::test]
fn render_data_contains_visible_geometry(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_size(900.0, 700.0);
    store.set_nodes(sample_nodes());
    store.set_edges(sample_edges());
    store.update_nodes();

    let bounds = gpui_kit::Bounds {
        origin: gpui_kit::point(gpui_kit::px(0.0), gpui_kit::px(0.0)),
        size: gpui_kit::size(gpui_kit::px(900.0), gpui_kit::px(700.0)),
    };
    let data = gpui_xyflow::ui::pane::build_render_data(&store, bounds);

    assert_eq!(data.nodes.len(), 2);
    assert_eq!(data.edges.len(), 1);

    let edge = &data.edges.edges[0];
    assert!(
        edge.path.starts_with('M'),
        "edge path must be an svg command, got {:?}",
        edge.path
    );
    assert!(edge.label_x > 0.0 && edge.label_y > 0.0);

    let node = data.nodes.find("a").expect("node a present");
    assert_eq!(node.rect.width, 120.0);
    assert_eq!(node.rect.height, 50.0);
    assert!(node.contains(XYPosition::new(150.0, 120.0)));
    assert!(!node.contains(XYPosition::new(50.0, 50.0)));

    let viewport = gpui_xyflow::ui::viewport::ScreenMapping::new(store.transform());
    assert!(viewport.is_rect_visible(node.rect, 900.0, 700.0));
}

#[gpui_kit::test]
fn flow_view_reports_nodes_after_attach(cx: &mut gpui_kit::TestAppContext) {
    use gpui_kit::AppContext;

    cx.update(|cx| {
        let entity = cx.new(|cx| {
            let mut flow = Flow::with_props(
                FlowProps {
                    theme: FlowTheme::dark(),
                    ..FlowProps::default()
                },
                cx,
            );
            flow.set_nodes(sample_nodes());
            flow.set_edges(sample_edges());
            flow
        });
        cx.read_entity(&entity, |flow, _cx| {
            assert_eq!(flow.store().nodes().len(), 2);
            assert_eq!(flow.store().edges().len(), 1);
            assert!(flow.store().props().theme.is_dark);
        });
    });
}

#[gpui_kit::test]
fn store_adopts_nodes_and_produces_edge_graphics(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_nodes(sample_nodes());
    store.set_edges(sample_edges());
    store.update_nodes();

    assert_eq!(store.node_lookup().len(), 2);
    assert!(store.node_lookup().contains_key("a"));
    assert!(store.node_lookup().contains_key("b"));

    let edge = store.edges()[0].clone();
    let graphics = store.edge_path_for(&edge).expect("edge graphics");
    assert!(
        graphics.path.starts_with('M'),
        "expected an svg path, got {:?}",
        graphics.path
    );
    assert!(graphics.path.contains('C') || graphics.path.contains('L'));
}

#[gpui_kit::test]
fn store_transforms_between_screen_and_flow_space(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_size(800.0, 600.0);
    store.set_nodes(sample_nodes());
    store.set_edges(sample_edges());
    store.update_nodes();
    store.set_viewport(gpui_xyflow::core::geometry::Viewport::new(20.0, 10.0, 1.5));

    let flow_point = XYPosition::new(100.0, 50.0);
    let screen = store.flow_to_screen_position(flow_point);
    let round_trip = store.screen_to_flow_position(screen);
    assert!((round_trip.x - flow_point.x).abs() < 1e-9);
    assert!((round_trip.y - flow_point.y).abs() < 1e-9);
    assert!((store.get_zoom() - 1.5).abs() < 1e-9);
}

#[gpui_kit::test]
fn store_zooms_within_limits(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let props = FlowProps::default().with_min_max_zoom(0.5, 2.0);
    let mut store = FlowStore::<String>::new(props);
    store.set_size(800.0, 600.0);
    store.set_nodes(sample_nodes());
    store.update_nodes();

    for _ in 0..40 {
        store.zoom_in();
    }
    assert!(store.get_zoom() <= 2.0 + 1e-9);

    for _ in 0..80 {
        store.zoom_out();
    }
    assert!(store.get_zoom() >= 0.5 - 1e-9);
}

#[gpui_kit::test]
fn store_fits_view_to_nodes(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_size(800.0, 600.0);
    store.set_nodes(sample_nodes());
    store.update_nodes();
    store.fit_view();

    let zoom = store.get_zoom();
    assert!(zoom >= 0.5 && zoom <= 2.0, "zoom out of range: {zoom}");

    let bounds = store.tight_bounds();
    assert!(bounds.width > 0.0);
    assert!(bounds.height > 0.0);
}

#[gpui_kit::test]
fn store_selects_and_unselects_nodes(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_nodes(sample_nodes());
    store.update_nodes();

    let change = gpui_xyflow::core::change::NodeChange::Select(
        gpui_xyflow::core::change::SelectionChange {
            id: "a".to_string(),
            selected: true,
        },
    );
    store.apply_node_changes(&[change]);
    store.update_nodes();
    assert!(store.node_lookup().get("a").unwrap().selected);

    store.unselect_nodes_and_edges();
    assert!(store.selected_node_ids().is_empty());
}

#[gpui_kit::test]
fn store_reports_selection_rect_changes(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_nodes(sample_nodes());
    store.update_nodes();

    let full = gpui_xyflow::core::geometry::Rect::new(90.0, 90.0, 440.0, 270.0);
    let changes = store.selection_changes_for_rect(full);
    assert_eq!(changes.len(), 2, "both nodes are fully inside the selection");

    store.apply_node_changes(&changes);
    store.update_nodes();
    assert_eq!(store.selected_node_ids().len(), 2);

    let partial = gpui_xyflow::core::geometry::Rect::new(120.0, 120.0, 10.0, 10.0);
    let partial_changes = store.selection_changes_for_rect(partial);
    assert_eq!(
        partial_changes.len(),
        2,
        "full selection mode does not select partially overlapped nodes"
    );
    for change in &partial_changes {
        match change {
            gpui_xyflow::core::change::NodeChange::Select(selection) => {
                assert!(!selection.selected, "partial overlap must deselect");
            }
            _ => panic!("expected only selection changes"),
        }
    }

    let none = gpui_xyflow::core::geometry::Rect::new(2000.0, 2000.0, 10.0, 10.0);
    let changes = store.selection_changes_for_rect(none);
    assert_eq!(changes.len(), 2, "both nodes must be deselected");
    store.apply_node_changes(&changes);
    store.update_nodes();
    assert!(store.selected_node_ids().is_empty());
}

#[gpui_kit::test]
fn theme_tokens_change_with_mode(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let light = FlowTheme::light();
    let dark = FlowTheme::dark();
    assert!(!light.is_dark);
    assert!(dark.is_dark);
    assert_ne!(light.background_color, dark.background_color);
    assert_eq!(light.edge_stroke_width, dark.edge_stroke_width);
}

#[gpui_kit::test]
fn background_patterns_produce_primitives(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let transform = gpui_xyflow::core::geometry::Transform::IDENTITY;
    let dots = gpui_xyflow::ui::background::BackgroundPattern::new(BackgroundVariant::Dots);
    assert!(!dots.dot_list(transform, 200.0, 200.0).is_empty());

    let lines = gpui_xyflow::ui::background::BackgroundPattern::new(BackgroundVariant::Lines);
    assert!(!lines.line_segments(transform, 200.0, 200.0).is_empty());

    let cross = gpui_xyflow::ui::background::BackgroundPattern::new(BackgroundVariant::Cross);
    assert!(!cross.cross_segments(transform, 200.0, 200.0).is_empty());
}

#[gpui_kit::test]
fn svg_path_round_trips_through_parser(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let path = gpui_xyflow::ui::paths::parse_svg_path("M0,0 C0,100 100,100 100,200");
    assert_eq!(path.commands.len(), 2);
    assert_eq!(path.start().unwrap().x, 0.0);
    assert_eq!(path.end().unwrap().y, 200.0);

    let straight = gpui_xyflow::ui::paths::parse_svg_path("M0,0L 100,100");
    assert_eq!(straight.commands.len(), 2);
}
