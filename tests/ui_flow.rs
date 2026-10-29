use gpui_kit::{Context, IntoElement, ParentElement, Render, Styled, Window, WindowOptions, div};
use gpui_xyflow::{
    core::{
        edge::{Edge, EdgeMarker, EdgeMarkerType, MarkerType},
        geometry::{Position, XYPosition},
        node::{Handle, HandleType, Node},
    },
    ui::{
        background::BackgroundVariant,
        flow::Flow,
        store::{FlowProps, FlowStore},
        theme::FlowTheme,
    },
};

#[gpui_kit::test]
fn background_pattern_supports_colors_and_xy_gap(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    use gpui_xyflow::{core::geometry::Transform, ui::background::BackgroundPattern};

    let theme = FlowTheme::light();
    let pattern = BackgroundPattern::default()
        .with_gap_xy([40.0, 20.0])
        .with_offset_xy([5.0, 7.0])
        .with_color(gpui_kit::rgb(0x123456))
        .with_bg_color(gpui_kit::rgb(0xf0f0f0));
    assert_eq!(pattern.color(&theme), gpui_kit::rgb(0x123456));
    assert_eq!(pattern.background_color(&theme), gpui_kit::rgb(0xf0f0f0));
    assert_eq!(pattern.scaled_gap(Transform::IDENTITY), [40.0, 20.0]);
    assert_eq!(pattern.color(&FlowTheme::light()), gpui_kit::rgb(0x123456));

    let default = BackgroundPattern::default();
    assert_eq!(default.color(&theme), theme.pattern_dots_color);
    assert_eq!(default.background_color(&theme), theme.background_color);
}

#[gpui_kit::test]
fn minimap_props_expose_colors_and_callbacks(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    use std::sync::Arc;

    use gpui_xyflow::ui::minimap::{MinimapCallbacks, MinimapProps};

    let props = MinimapProps::default()
        .with_colors(
            gpui_kit::rgb(0xeeeeee),
            gpui_kit::rgb(0x000000),
            gpui_kit::rgb(0xff0000),
        )
        .with_node_style(gpui_kit::rgb(0x00ff00), gpui_kit::rgb(0x0000ff), 8.0)
        .with_inverse_pan(true)
        .with_offset_scale(12.0)
        .with_aria_label("minimap");
    assert_eq!(props.bg_color, Some(gpui_kit::rgb(0xeeeeee)));
    assert_eq!(props.mask_color, Some(gpui_kit::rgb(0x000000)));
    assert_eq!(props.node_color, Some(gpui_kit::rgb(0x00ff00)));
    assert_eq!(props.node_stroke_color, Some(gpui_kit::rgb(0x0000ff)));
    assert_eq!(props.node_border_radius, 8.0);
    assert!(props.inverse_pan);
    assert_eq!(props.offset_scale, 12.0);
    assert_eq!(props.aria_label.as_deref(), Some("minimap"));

    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = calls.clone();
    let callbacks = MinimapCallbacks {
        on_click: Some(Arc::new(move |_| {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })),
        on_node_click: Some(Arc::new(|_| {})),
    };
    assert!(callbacks.on_click.is_some());
    (callbacks.on_click.as_ref().unwrap())(gpui_xyflow::core::geometry::XYPosition::new(1.0, 2.0));
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    let cloned = callbacks.clone();
    assert_eq!(callbacks, cloned);
}

#[gpui_kit::test]
fn controls_fit_view_options_are_configurable(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    use gpui_xyflow::ui::controls::{ControlButton, ControlsProps, FitViewOptions};

    let options = FitViewOptions {
        padding: 0.3,
        min_zoom: Some(0.8),
        max_zoom: Some(1.5),
        include_hidden_nodes: true,
    };
    let controls = ControlsProps::default().with_fit_view_options(options);
    assert_eq!(controls.fit_view_options, options);
    assert_eq!(ControlsProps::default().fit_view_options.padding, 0.1);

    let icon: gpui_xyflow::ui::controls::ControlContent =
        std::sync::Arc::new(|_, _| gpui_kit::div().child("i").into_any_element());
    let controls = ControlsProps::default().with_button_content(ControlButton::FitView, icon);
    assert!(controls.button_content(ControlButton::FitView).is_some());
    assert!(controls.button_content(ControlButton::ZoomIn).is_none());
}

fn sample_nodes() -> Vec<Node<String>> {
    let mut a = Node::new("a", XYPosition::new(100.0, 100.0), "a".to_string())
        .with_type("input")
        .with_size(120.0, 50.0)
        .with_measured(120.0, 50.0)
        .with_source_position(Position::Bottom);
    a.handles = vec![Handle::new(
        "a",
        HandleType::Source,
        Position::Bottom,
        0.5,
        1.0,
    )];

    let mut b = Node::new("b", XYPosition::new(400.0, 300.0), "b".to_string())
        .with_type("output")
        .with_size(120.0, 50.0)
        .with_measured(120.0, 50.0)
        .with_target_position(Position::Top);
    b.handles = vec![Handle::new(
        "b",
        HandleType::Target,
        Position::Top,
        0.5,
        0.0,
    )];

    vec![a, b]
}

fn sample_edges() -> Vec<Edge<String>> {
    let mut edge = Edge {
        id: "a-b".to_string(),
        source: "a".to_string(),
        target: "b".to_string(),
        ..Edge::default()
    };
    edge.marker_end = Some(EdgeMarkerType::Marker(EdgeMarker::new(
        MarkerType::ArrowClosed,
    )));
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
fn flow_accepts_custom_renderers_and_node_toolbar(cx: &mut gpui_kit::TestAppContext) {
    use gpui_kit::AppContext;

    cx.update(|cx| {
        let _window = cx.open_window(WindowOptions::default(), |_window, cx| {
            let flow = cx.new(|cx| {
                let mut flow = Flow::with_props(FlowProps::default(), cx);
                flow.set_nodes(sample_nodes());
                flow.set_edges(sample_edges());
                flow.store_mut().set_node_selected("a", true);
                flow.set_node_content_renderer(Some(std::sync::Arc::new(
                    |_node: &Node<String>,
                     _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
                     _window: &mut Window,
                     _cx: &mut gpui_kit::App| {},
                )));
                flow.set_node_toolbar(
                    Some(gpui_xyflow::ui::toolbars::NodeToolbarProps::default()),
                    Some(std::sync::Arc::new(
                        |node: &Node<String>, _window: &mut Window, _cx: &mut gpui_kit::App| {
                            div().child(node.id.clone()).into_any_element()
                        },
                    )),
                );
                flow.set_viewport_portal(vec![gpui_xyflow::ui::flow::ViewportPortalItem {
                    x: 40.0,
                    y: 30.0,
                    content: std::sync::Arc::new(
                        |_window: &mut Window, _cx: &mut gpui_kit::App| {
                            div().child("annotation").into_any_element()
                        },
                    ),
                }]);
                flow.set_panels(vec![
                    gpui_xyflow::ui::flow::PanelItem::new(
                        "legend",
                        gpui_xyflow::core::geometry::PanelPosition::TopLeft,
                        std::sync::Arc::new(|_window: &mut Window, _cx: &mut gpui_kit::App| {
                            div().child("legend").into_any_element()
                        }),
                    )
                    .with_margin(20.0),
                ]);
                flow.set_edge_label_renderer(Some(std::sync::Arc::new(
                    |edge: &Edge<String>, _window: &mut Window, _cx: &mut gpui_kit::App| {
                        div().child(edge.id.clone()).into_any_element()
                    },
                )));
                flow.set_node_element_renderer(Some(std::sync::Arc::new(
                    |node: &gpui_xyflow::core::node::Node<String>,
                     _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
                     _zoom: f64,
                     _window: &mut Window,
                     _cx: &mut gpui_kit::App| {
                        div().child(node.id.clone()).into_any_element()
                    },
                )));
                flow.set_edge_element_renderer(Some(std::sync::Arc::new(
                    |edge: &gpui_xyflow::core::edge::Edge<String>,
                     _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
                     _transform: gpui_xyflow::core::geometry::Transform,
                     _window: &mut Window,
                     _cx: &mut gpui_kit::App| {
                        div().child(edge.id.clone()).into_any_element()
                    },
                )));
                flow.set_handle_renderer(Some(std::sync::Arc::new(
                    |_node: &gpui_xyflow::core::node::Node<String>,
                     handle: &gpui_xyflow::core::node::Handle,
                     _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
                     _zoom: f64,
                     _window: &mut Window,
                     _cx: &mut gpui_kit::App| {
                        div()
                            .child(handle.handle_type.as_str().to_string())
                            .into_any_element()
                    },
                )));
                flow.set_connection_line_renderer(Some(std::sync::Arc::new(
                    |_props: &gpui_xyflow::ui::flow::ConnectionLineProps,
                     _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
                     _transform: gpui_xyflow::core::geometry::Transform,
                     _window: &mut Window,
                     _cx: &mut gpui_kit::App| {
                        div().child("line").into_any_element()
                    },
                )));
                flow.set_node_resizer(Some(
                    gpui_xyflow::ui::resizer::NodeResizerProps::default()
                        .with_visibility(Some(true))
                        .with_control_renderer(std::sync::Arc::new(
                            |_control: &gpui_xyflow::ui::resizer::ResizeControl,
                             _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
                             _zoom: f64,
                             _window: &mut Window,
                             _cx: &mut gpui_kit::App| {
                                div().child("resize").into_any_element()
                            },
                        )),
                ));
                flow
            });
            cx.new(|_cx| FlowHost { flow })
        });
        assert_eq!(cx.windows().len(), 1, "expected exactly one window");
    });
}

#[gpui_kit::test]
fn attribution_overlay_is_visible_unless_hidden(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let visible = gpui_xyflow::ui::components::attribution("react", false, "bottom-right");
    assert!(visible.visible);
    assert_eq!(visible.label, "React Flow");
    assert_eq!(visible.panel_position, "bottom-right");

    let hidden = gpui_xyflow::ui::components::attribution("react", true, "bottom-right");
    assert!(!hidden.visible);

    let props = FlowProps::<String>::default();
    assert!(!props.hide_attribution);
}

#[gpui_kit::test]
fn render_data_exposes_reconnect_handles_for_selected_edges(cx: &mut gpui_kit::TestAppContext) {
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
    assert!(data.reconnect_handles.is_empty());

    store.set_edge_selected("a-b", true);
    let data = gpui_xyflow::ui::pane::build_render_data(&store, bounds);
    assert_eq!(data.reconnect_handles.len(), 2);

    store.props_mut().edges_reconnectable = false;
    let data = gpui_xyflow::ui::pane::build_render_data(&store, bounds);
    assert!(data.reconnect_handles.is_empty());
}

#[gpui_kit::test]
fn only_render_visible_elements_culls_offscreen_geometry(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut nodes = sample_nodes();
    let mut far = Node::new("far", XYPosition::new(5000.0, 5000.0), "far".to_string())
        .with_size(120.0, 50.0)
        .with_measured(120.0, 50.0)
        .with_source_position(Position::Bottom);
    far.handles = vec![Handle::new(
        "far",
        HandleType::Source,
        Position::Bottom,
        0.5,
        1.0,
    )];
    nodes.push(far);
    let mut far_target = Node::new("far2", XYPosition::new(5200.0, 5000.0), "far2".to_string())
        .with_size(120.0, 50.0)
        .with_measured(120.0, 50.0)
        .with_target_position(Position::Top);
    far_target.handles = vec![Handle::new(
        "far2",
        HandleType::Target,
        Position::Top,
        0.5,
        0.0,
    )];
    nodes.push(far_target);
    let mut edges = sample_edges();
    let mut far_edge = edges[0].clone();
    far_edge.id = "far-far2".to_string();
    far_edge.source = "far".to_string();
    far_edge.target = "far2".to_string();
    edges.push(far_edge);

    let bounds = gpui_kit::Bounds {
        origin: gpui_kit::point(gpui_kit::px(0.0), gpui_kit::px(0.0)),
        size: gpui_kit::size(gpui_kit::px(900.0), gpui_kit::px(700.0)),
    };

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_size(900.0, 700.0);
    store.set_nodes(nodes.clone());
    store.set_edges(edges.clone());
    store.update_nodes();

    let data = gpui_xyflow::ui::pane::build_render_data(&store, bounds);
    assert_eq!(data.nodes.len(), 4);
    assert_eq!(data.edges.len(), 2);

    store.props_mut().only_render_visible_elements = true;
    let data = gpui_xyflow::ui::pane::build_render_data(&store, bounds);
    assert_eq!(data.nodes.len(), 2);
    assert_eq!(data.edges.len(), 1);
    assert_eq!(data.edges.edges[0].id, "a-b");
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
    assert!((0.5..=2.0).contains(&zoom), "zoom out of range: {zoom}");

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

    let change =
        gpui_xyflow::core::change::NodeChange::Select(gpui_xyflow::core::change::SelectionChange {
            id: "a".to_string(),
            selected: true,
        });
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
    assert_eq!(
        changes.len(),
        2,
        "both nodes are fully inside the selection"
    );

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
fn edge_hit_testing_finds_the_closest_edge(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_size(900.0, 700.0);
    store.set_nodes(sample_nodes());
    store.set_edges(sample_edges());
    store.update_nodes();

    let graphics = store
        .edge_path_for(&store.edges()[0])
        .expect("edge graphics");
    let on_edge = XYPosition::new(graphics.label_x, graphics.label_y);
    assert_eq!(
        gpui_xyflow::ui::edge_renderer::edge_at_point(&store, on_edge, 1.0),
        Some("a-b".to_string())
    );
    assert_eq!(
        gpui_xyflow::ui::edge_renderer::edge_at_point(&store, XYPosition::new(50.0, 50.0), 1.0),
        None
    );
}

#[gpui_kit::test]
fn edge_labels_are_rendered_in_render_data(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_size(900.0, 700.0);
    store.set_nodes(sample_nodes());
    let mut edges = sample_edges();
    edges[0].label = Some("hello".to_string());
    store.set_edges(edges);
    store.update_nodes();

    let bounds = gpui_kit::Bounds {
        origin: gpui_kit::point(gpui_kit::px(0.0), gpui_kit::px(0.0)),
        size: gpui_kit::size(gpui_kit::px(900.0), gpui_kit::px(700.0)),
    };
    let data = gpui_xyflow::ui::pane::build_render_data(&store, bounds);
    assert_eq!(data.edges.edges[0].label.as_deref(), Some("hello"));
}

#[gpui_kit::test]
fn arrow_keys_accept_gpui_key_names(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    assert_eq!(
        gpui_xyflow::ui::components::arrow_key_diff("up"),
        Some(XYPosition::new(0.0, -1.0))
    );
    assert_eq!(
        gpui_xyflow::ui::components::arrow_key_diff("ArrowRight"),
        Some(XYPosition::new(1.0, 0.0))
    );
    assert_eq!(gpui_xyflow::ui::components::arrow_key_diff("Escape"), None);
}

#[gpui_kit::test]
fn store_selection_helpers_update_lookup_and_edges(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_nodes(sample_nodes());
    store.set_edges(sample_edges());
    store.update_nodes();

    store.set_node_selected("a", true);
    store.set_edge_selected("a-b", true);
    assert_eq!(store.selected_node_ids(), vec!["a".to_string()]);
    assert_eq!(store.selected_edge_ids(), vec!["a-b".to_string()]);
    assert!(store.node_lookup().get("a").unwrap().selected);
    assert!(store.edge_lookup().get("a-b").unwrap().selected);

    store.unselect_nodes_and_edges();
    assert!(store.selected_node_ids().is_empty());
    assert!(store.selected_edge_ids().is_empty());
}

#[gpui_kit::test]
fn connection_validator_can_be_installed_on_props(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut props = FlowProps::<String>::default();
    assert!(props.callbacks.is_empty());
    props.callbacks.is_valid_connection = Some(std::sync::Arc::new(|connection| {
        connection.source != "blocked"
    }));
    assert!(!props.callbacks.is_empty());
    let validator = props.callbacks.is_valid_connection.clone().unwrap();
    assert!(!validator(&gpui_xyflow::core::edge::Connection::new(
        "blocked", "b"
    )));
    assert!(validator(&gpui_xyflow::core::edge::Connection::new(
        "a", "b"
    )));
}

#[gpui_kit::test]
fn dashed_path_offsets_dash_pattern(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    use gpui_xyflow::ui::{
        paint::dashed_path,
        paths::{PathCommand, parse_svg_path},
    };

    let path = parse_svg_path("M 0 0 L 100 0");
    let dashed = dashed_path(&path, 10.0, 10.0, 0.0);
    let PathCommand::MoveTo(start) = dashed.commands[0] else {
        panic!("expected first dash to start with a move");
    };
    assert_eq!(start.x, 0.0);
    let PathCommand::LineTo(end) = dashed.commands[1] else {
        panic!("expected first dash segment");
    };
    assert!((end.x - 10.0).abs() < 1e-3);

    let shifted = dashed_path(&path, 10.0, 10.0, 5.0);
    let PathCommand::MoveTo(start) = shifted.commands[0] else {
        panic!("expected shifted dash to start with a move");
    };
    assert_eq!(start.x, 0.0);
    let PathCommand::LineTo(end) = shifted.commands[1] else {
        panic!("expected shifted dash segment");
    };
    assert!((end.x - 5.0).abs() < 1e-3);

    assert!(dashed_path(&parse_svg_path(""), 10.0, 10.0, 0.0).is_empty());
}

#[gpui_kit::test]
fn connection_line_props_mirror_in_progress_state(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    use gpui_xyflow::{
        core::connect::{ConnectionInProgress, ConnectionState},
        ui::flow::ConnectionLineProps,
    };

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_size(900.0, 700.0);
    store.set_nodes(sample_nodes());
    store.update_nodes();

    let from_node = store.node_lookup().get("a").cloned().expect("node a");
    let from_handle = from_node.internals.user_node.handles[0].clone();
    store.set_connection(ConnectionState::InProgress(ConnectionInProgress {
        is_valid: Some(false),
        from: XYPosition::new(10.0, 20.0),
        from_handle: from_handle.clone(),
        from_position: Position::Bottom,
        from_node,
        to: XYPosition::new(30.0, 40.0),
        to_handle: None,
        to_position: Position::Bottom,
        to_node: None,
        pointer: XYPosition::new(30.0, 40.0),
    }));

    let ConnectionState::InProgress(progress) = store.connection_state() else {
        panic!("expected connection in progress");
    };
    let props = ConnectionLineProps::from_progress(progress);
    assert_eq!(props.from_x, 10.0);
    assert_eq!(props.from_y, 20.0);
    assert_eq!(props.to_x, 30.0);
    assert_eq!(props.to_y, 40.0);
    assert_eq!(props.from_position, Position::Bottom);
    // 未命中目标手柄时，目标方向取起点反方向。
    assert_eq!(props.to_position, Position::Top);
    assert_eq!(props.connection_status, Some(false));
    assert_eq!(props.from_node.id, "a");
    assert_eq!(props.from_handle.id, from_handle.id);
    assert!(props.to_node.is_none());
    assert!(props.to_handle.is_none());
}

#[gpui_kit::test]
fn node_resizer_builders_configure_boundaries(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let props = gpui_xyflow::ui::resizer::NodeResizerProps::default()
        .with_min_size(20.0, 30.0)
        .with_max_size(200.0, 300.0)
        .with_keep_aspect_ratio(true)
        .with_node_ids(vec!["a".to_string()]);
    assert_eq!(
        props.boundaries(),
        gpui_xyflow::core::xyresizer::Boundaries::new(20.0, 30.0, 200.0, 300.0)
    );
    assert!(props.keep_aspect_ratio);
    assert_eq!(props.node_ids, vec!["a".to_string()]);
    assert!(props.control_renderer.is_none());

    let renderer: gpui_xyflow::ui::resizer::ResizeControlRenderer = std::sync::Arc::new(
        |_control: &gpui_xyflow::ui::resizer::ResizeControl,
         _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
         _zoom: f64,
         _window: &mut Window,
         _cx: &mut gpui_kit::App| { div().child("handle").into_any_element() },
    );
    let with_renderer = gpui_xyflow::ui::resizer::NodeResizerProps::default()
        .with_control_renderer(renderer.clone());
    assert!(with_renderer.control_renderer.is_some());
    assert_eq!(with_renderer, with_renderer.clone());
    assert_ne!(
        with_renderer,
        gpui_xyflow::ui::resizer::NodeResizerProps::default().with_control_renderer(
            std::sync::Arc::new(
                |_control: &gpui_xyflow::ui::resizer::ResizeControl,
                 _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
                 _zoom: f64,
                 _window: &mut Window,
                 _cx: &mut gpui_kit::App| {
                    div().child("other").into_any_element()
                },
            )
        )
    );

    let info = gpui_xyflow::ui::resizer::NodeResizeInfo::from_rect(
        gpui_xyflow::core::geometry::Rect::new(1.0, 2.0, 30.0, 40.0),
    );
    assert_eq!(info.x, 1.0);
    assert_eq!(info.y, 2.0);
    assert_eq!(info.width, 30.0);
    assert_eq!(info.height, 40.0);
}

#[gpui_kit::test]
fn controls_layout_accounts_for_custom_items(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let base = gpui_xyflow::ui::controls::ControlsProps::default();
    assert_eq!(base.item_count(), 4);
    let custom = base.clone().with_custom_item(std::sync::Arc::new(|_, _| {
        gpui_kit::div().into_any_element()
    }));
    assert_eq!(custom.item_count(), 5);
    assert_eq!(custom.height(), custom.button_size * 5.0);
    assert_eq!(custom.width(), custom.button_size);

    let horizontal = gpui_xyflow::ui::controls::ControlsProps {
        orientation: gpui_xyflow::ui::controls::ControlsOrientation::Horizontal,
        ..custom
    };
    assert_eq!(horizontal.width(), horizontal.button_size * 5.0);
    assert_eq!(horizontal.height(), horizontal.button_size);
}

#[gpui_kit::test]
fn controls_callbacks_compare_by_handler_identity(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    use std::sync::Arc;

    use gpui_xyflow::ui::controls::ControlsCallbacks;

    let handler: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let callbacks = ControlsCallbacks {
        on_zoom_in: Some(handler.clone()),
        ..Default::default()
    };
    assert_eq!(callbacks, callbacks.clone());
    assert_ne!(callbacks, ControlsCallbacks::default());
}

#[gpui_kit::test]
fn minimap_origin_follows_panel_position(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let bounds = gpui_kit::Bounds {
        origin: gpui_kit::point(gpui_kit::px(10.0), gpui_kit::px(20.0)),
        size: gpui_kit::size(gpui_kit::px(800.0), gpui_kit::px(600.0)),
    };
    let props = gpui_xyflow::ui::minimap::MinimapProps {
        width: 200.0,
        height: 150.0,
        margin: 15.0,
        ..Default::default()
    };
    assert_eq!(
        gpui_xyflow::ui::minimap::minimap_origin(bounds, &props),
        XYPosition::new(10.0 + 800.0 - 15.0 - 200.0, 20.0 + 600.0 - 15.0 - 150.0)
    );
    assert_eq!(
        gpui_xyflow::ui::minimap::minimap_origin(
            bounds,
            &props.with_position(gpui_xyflow::core::geometry::PanelPosition::TopLeft)
        ),
        XYPosition::new(25.0, 35.0)
    );
}

#[gpui_kit::test]
fn minimap_interaction_flags_default_off_and_can_be_enabled(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let props = gpui_xyflow::ui::minimap::MinimapProps::default();
    assert!(!props.pannable);
    assert!(!props.zoomable);
    let enabled = props.with_pannable(true).with_zoomable(true);
    assert!(enabled.pannable);
    assert!(enabled.zoomable);
}

#[gpui_kit::test]
fn selected_nodes_render_on_top_when_elevation_is_enabled(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let bounds = gpui_kit::Bounds {
        origin: gpui_kit::point(gpui_kit::px(0.0), gpui_kit::px(0.0)),
        size: gpui_kit::size(gpui_kit::px(900.0), gpui_kit::px(700.0)),
    };

    let mut plain = FlowStore::<String>::new(FlowProps::default());
    plain.set_size(900.0, 700.0);
    plain.set_nodes(sample_nodes());
    plain.update_nodes();
    plain.set_node_selected("a", true);
    let plain_order: Vec<String> = gpui_xyflow::ui::pane::build_render_data(&plain, bounds)
        .nodes
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect();
    assert_eq!(plain_order, vec!["a".to_string(), "b".to_string()]);

    let props = FlowProps::<String> {
        elevate_nodes_on_select: true,
        ..FlowProps::default()
    };
    let mut elevated = FlowStore::<String>::new(props);
    elevated.set_size(900.0, 700.0);
    elevated.set_nodes(sample_nodes());
    elevated.update_nodes();
    elevated.set_node_selected("a", true);
    let elevated_order: Vec<String> = gpui_xyflow::ui::pane::build_render_data(&elevated, bounds)
        .nodes
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect();
    assert_eq!(elevated_order, vec!["b".to_string(), "a".to_string()]);
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
fn node_toolbar_targets_follow_selection_and_overrides(cx: &mut gpui_kit::TestAppContext) {
    let _ = cx;

    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_nodes(sample_nodes());
    store.update_nodes();

    let defaults = gpui_xyflow::ui::toolbars::NodeToolbarProps::default();
    let default_targets = gpui_xyflow::ui::toolbars::node_toolbar_targets(
        &defaults,
        store.node_lookup(),
        &store.selected_node_ids(),
    );
    assert!(default_targets.is_empty());

    store.set_node_selected("a", true);
    let selected_targets = gpui_xyflow::ui::toolbars::node_toolbar_targets(
        &defaults,
        store.node_lookup(),
        &store.selected_node_ids(),
    );
    assert_eq!(selected_targets, vec!["a".to_string()]);

    let pinned = gpui_xyflow::ui::toolbars::NodeToolbarProps {
        node_ids: vec!["b".to_string()],
        is_visible: Some(true),
        ..Default::default()
    };
    assert_eq!(
        gpui_xyflow::ui::toolbars::node_toolbar_targets(
            &pinned,
            store.node_lookup(),
            &store.selected_node_ids()
        ),
        vec!["b".to_string()]
    );

    let hidden = gpui_xyflow::ui::toolbars::NodeToolbarProps {
        node_ids: vec!["b".to_string()],
        is_visible: Some(false),
        ..Default::default()
    };
    assert!(
        gpui_xyflow::ui::toolbars::node_toolbar_targets(
            &hidden,
            store.node_lookup(),
            &store.selected_node_ids()
        )
        .is_empty()
    );
}

#[gpui_kit::test]
fn render_data_attaches_custom_renderer_sources(cx: &mut gpui_kit::TestAppContext) {
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
    let mut data = gpui_xyflow::ui::pane::build_render_data(&store, bounds);
    assert!(data.node_content_renderer.is_none());
    assert!(data.node_sources.is_empty());
    assert!(data.edge_sources.is_empty());

    let node_renderer: gpui_xyflow::ui::pane::NodeContentRenderer = std::sync::Arc::new(
        |_node: &Node<String>,
         _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
         _window: &mut Window,
         _cx: &mut gpui_kit::App| {},
    );
    data.attach_custom_renderers(&store, Some(node_renderer), None);
    assert!(data.node_content_renderer.is_some());
    assert_eq!(data.node_sources.len(), 2);
    assert!(data.node_sources.contains_key("a"));
    assert!(data.edge_sources.is_empty());

    let edge_renderer: gpui_xyflow::ui::pane::EdgeContentRenderer = std::sync::Arc::new(
        |_edge: &Edge<String>,
         _bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
         _transform: gpui_xyflow::core::geometry::Transform,
         _window: &mut Window,
         _cx: &mut gpui_kit::App| {},
    );
    data.attach_custom_renderers(&store, None, Some(edge_renderer));
    assert!(data.edge_content_renderer.is_some());
    assert!(data.node_sources.is_empty());
    assert_eq!(data.edge_sources.len(), 1);
    assert!(data.edge_sources.contains_key("a-b"));
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
fn node_components_render_nodes_in_the_element_layer(cx: &mut gpui_kit::TestAppContext) {
    use gpui_kit::AppContext;

    cx.update(|cx| {
        let _window = cx.open_window(WindowOptions::default(), |_window, cx| {
            let flow = cx.new(|cx| {
                // 默认关闭元素层节点组件，保持画布绘制路径兼容。
                let mut flow = Flow::with_props(FlowProps::default(), cx);
                assert!(!flow.node_components());
                flow.set_nodes(sample_nodes());
                flow.set_edges(sample_edges());
                flow.store_mut().set_node_selected("a", true);
                flow.set_node_components(true);
                assert!(flow.node_components());
                flow
            });
            cx.new(|_cx| FlowHost { flow })
        });
        assert_eq!(cx.windows().len(), 1, "expected exactly one window");
    });
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
