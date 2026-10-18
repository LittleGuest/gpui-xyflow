use gpui_xyflow::{
    core::{
        connections::{add_edge, connection_exists, get_edge_id},
        edge::Edge,
        geometry::{Rect, Viewport, XYPosition},
        node::Node,
    },
    ui::api::{ConnectionApi, FlowHandle, FlowHandleState, NodeDataApi, ViewportApi},
};

fn node(id: &str, x: f64, y: f64) -> Node<String> {
    Node::<String> {
        id: id.to_string(),
        position: XYPosition::new(x, y),
        width: Some(80.0),
        height: Some(40.0),
        ..Node::<String>::default()
    }
}

fn edge(id: &str, source: &str, target: &str) -> Edge<String> {
    Edge::<String> {
        id: id.to_string(),
        source: source.to_string(),
        target: target.to_string(),
        ..Edge::<String>::default()
    }
}

#[test]
fn flow_handle_state_lookup() {
    let state = FlowHandleState {
        nodes: vec![node("a", 0.0, 0.0)],
        edges: vec![edge("e1", "a", "b")],
    };
    assert!(state.node("a").is_some());
    assert!(state.node("zzz").is_none());
    assert!(state.edge("e1").is_some());
    assert_eq!(state.node_ids(), vec!["a"]);
    assert_eq!(state.edge_ids(), vec!["e1"]);
}

#[test]
fn viewport_api_zoom_clamped() {
    let mut api = ViewportApi::new(Viewport::IDENTITY, 800.0, 600.0);
    api.set_scale_extent(0.5, 2.0);
    api.scale_to(10.0);
    assert_eq!(api.zoom(), 2.0);
    api.scale_to(0.01);
    assert_eq!(api.zoom(), 0.5);
}

#[test]
fn viewport_api_scale_by() {
    let mut api = ViewportApi::new(Viewport::new(0.0, 0.0, 1.0), 800.0, 600.0);
    api.scale_by(1.5);
    assert_eq!(api.zoom(), 1.5);
}

#[test]
fn viewport_api_set_center() {
    let mut api = ViewportApi::new(Viewport::new(0.0, 0.0, 1.0), 800.0, 600.0);
    api.set_center(100.0, 100.0, Some(1.0));
    let viewport = api.viewport();
    assert_eq!(viewport.x, 300.0);
    assert_eq!(viewport.y, 200.0);
}

#[test]
fn viewport_api_pan_by() {
    let mut api = ViewportApi::new(Viewport::new(10.0, 20.0, 1.0), 800.0, 600.0);
    api.pan_by(XYPosition::new(5.0, -5.0));
    assert_eq!(api.viewport().x, 15.0);
    assert_eq!(api.viewport().y, 15.0);
}

#[test]
fn viewport_api_coordinate_conversion_roundtrip() {
    let api = ViewportApi::new(Viewport::new(30.0, 40.0, 2.0), 800.0, 600.0);
    let flow = XYPosition::new(10.0, 20.0);
    let screen = api.flow_to_screen(flow);
    let back = api.screen_to_flow(screen);
    assert!((back.x - flow.x).abs() < 1e-9);
    assert!((back.y - flow.y).abs() < 1e-9);
}

#[test]
fn viewport_api_fit_bounds_sets_zoom() {
    let mut api = ViewportApi::new(Viewport::IDENTITY, 800.0, 600.0);
    api.fit_bounds(Rect::new(0.0, 0.0, 100.0, 100.0), 0.1);
    assert!(api.zoom() > 0.0);
}

#[test]
fn node_data_api_connections() {
    let api = NodeDataApi::new(
        vec![node("a", 0.0, 0.0), node("b", 0.0, 0.0)],
        vec![edge("e1", "a", "b")],
    );
    let connections = api.get_node_connections("a");
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].edge_id, "e1");
    assert_eq!(api.get_connected_node_ids("a"), vec!["b"]);
}

#[test]
fn node_data_api_handle_connections() {
    let mut e = edge("e1", "a", "b");
    e.source_handle = Some("out".to_string());
    let api = NodeDataApi::new(vec![node("a", 0.0, 0.0)], vec![e]);
    assert_eq!(api.get_handle_connections("a", Some("out")).len(), 1);
    assert_eq!(api.get_handle_connections("a", Some("other")).len(), 0);
}

#[test]
fn node_data_api_update_node_data() {
    let mut api = NodeDataApi::new(vec![node("a", 0.0, 0.0)], Vec::new());
    assert!(api.update_node_data("a", |data| *data = "changed".to_string()));
    assert_eq!(api.get_node("a").unwrap().data, "changed");
    assert!(!api.update_node_data("ghost", |_| {}));
}

#[test]
fn node_data_api_update_edge_data() {
    let mut api = NodeDataApi::new(Vec::new(), vec![edge("e1", "a", "b")]);
    assert!(api.update_edge_data("e1", |data| *data = Some("payload".to_string())));
    assert_eq!(api.get_edge("e1").unwrap().data.as_deref(), Some("payload"));
    assert!(!api.update_edge_data("ghost", |_| {}));
}

#[test]
fn node_data_api_to_object() {
    let api = NodeDataApi::new(
        vec![node("a", 0.0, 0.0), node("b", 0.0, 0.0)],
        vec![edge("e1", "a", "b")],
    );
    let object = api.to_object();
    assert_eq!(object["a"], vec!["b"]);
    assert_eq!(object["b"], vec!["a"]);
}

#[test]
fn node_data_api_get_edge_id_by_handles() {
    let mut e = edge("e1", "a", "b");
    e.source_handle = Some("s".to_string());
    let api = NodeDataApi::new(Vec::new(), vec![e]);
    assert_eq!(
        api.get_edge_id("a", "b", Some("s"), None).as_deref(),
        Some("e1")
    );
    assert!(api.get_edge_id("a", "b", None, None).is_none());
}

#[test]
fn connection_api_outgoing_and_incoming() {
    let api = ConnectionApi::new(FlowHandleState {
        nodes: Vec::new(),
        edges: vec![edge("e1", "a", "b"), edge("e2", "a", "c")],
    });
    assert_eq!(api.outgoing("a").len(), 2);
    assert_eq!(api.incoming("b").len(), 1);
    assert!(api.has_connection("a", "b"));
    assert!(!api.has_connection("b", "a"));
}

#[test]
fn connection_api_can_connect_rejects_self_and_duplicate() {
    let api = ConnectionApi::new(FlowHandleState {
        nodes: Vec::new(),
        edges: vec![edge("e1", "a", "b")],
    });
    assert!(!api.can_connect("a", "a"));
    assert!(!api.can_connect("a", "b"));
    assert!(api.can_connect("b", "c"));
}

#[test]
fn flow_handle_keeps_views_in_sync() {
    let mut handle = FlowHandle::new(
        vec![node("a", 0.0, 0.0)],
        vec![edge("e1", "a", "b")],
        Viewport::IDENTITY,
        800.0,
        600.0,
    );
    assert_eq!(handle.connected_edges_of("a"), vec!["e1"]);
    handle.state.edges.push(edge("e2", "a", "c"));
    handle.sync();
    assert_eq!(handle.connected_edges_of("a").len(), 2);
}

#[test]
fn flow_handle_reports_selection() {
    let mut a = node("a", 0.0, 0.0);
    a.selected = true;
    let handle = FlowHandle::new(vec![a], Vec::new(), Viewport::IDENTITY, 800.0, 600.0);
    assert_eq!(handle.selected_node_ids(), vec!["a"]);
    assert!(handle.selected_edge_ids().is_empty());
}

#[test]
fn get_edge_id_matches_official_format() {
    assert_eq!(get_edge_id("a", None, "b", None), "xy-edge__a-b");
    assert_eq!(
        get_edge_id("a", Some("s"), "b", Some("t")),
        "xy-edge__as-bt"
    );
}

#[test]
fn connection_exists_matches_handles() {
    let a = edge("e1", "x", "y");
    let mut b = edge("e2", "x", "y");
    b.source_handle = Some("s".to_string());
    assert!(connection_exists(&a, &[edge("e0", "x", "y")]));
    assert!(!connection_exists(&b, &[edge("e0", "x", "y")]));
}

#[test]
fn add_edge_generates_id() {
    let mut e = edge("", "a", "b");
    e.id.clear();
    let result = add_edge(e, &[], None).unwrap();
    assert_eq!(result[0].id, "xy-edge__a-b");
}

#[test]
fn add_edge_rejects_self_loop() {
    let mut e = edge("", "a", "a");
    e.id.clear();
    assert!(add_edge(e, &[], None).is_none());
}

#[test]
fn add_edge_rejects_duplicate() {
    let e = edge("", "a", "b");
    assert!(add_edge(e, &[edge("e1", "a", "b")], None).is_none());
}

#[test]
fn add_edge_applies_default_options() {
    let mut e = edge("", "a", "b");
    e.id.clear();
    let defaults = Edge::<String> {
        edge_type: Some("smoothstep".to_string()),
        animated: true,
        ..Edge::<String>::default()
    };
    let result = add_edge(e, &[], Some(&defaults)).unwrap();
    assert_eq!(result[0].edge_type.as_deref(), Some("smoothstep"));
    assert!(result[0].animated);
}

#[test]
fn handles_of_empty_when_no_bounds() {
    let lookup_node = Node::<String>::default();
    assert!(lookup_node.handles.is_empty());
}

#[test]
fn get_intersecting_nodes_finds_overlapping_peers() {
    let api = NodeDataApi::new(
        vec![
            node("a", 0.0, 0.0),
            node("b", 40.0, 0.0),
            node("c", 900.0, 900.0),
        ],
        vec![],
    );
    let target = api.get_node("a").unwrap().clone();
    let matches = api.get_intersecting_nodes(&target, true, [0.0, 0.0]);
    let ids: Vec<&str> = matches.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, vec!["b"]);
}

#[test]
fn get_intersecting_nodes_ignores_self() {
    let api = NodeDataApi::new(vec![node("a", 0.0, 0.0)], vec![]);
    let target = api.get_node("a").unwrap().clone();
    assert!(
        api.get_intersecting_nodes(&target, true, [0.0, 0.0])
            .is_empty()
    );
}

#[test]
fn node_rect_uses_node_position_without_parent() {
    let api = NodeDataApi::new(vec![node("a", 10.0, 20.0)], vec![]);
    let target = api.get_node("a").unwrap().clone();
    let rect = api.node_rect(&target, [0.0, 0.0]);
    assert_eq!(rect, Rect::new(10.0, 20.0, 80.0, 40.0));
}

#[test]
fn is_node_intersecting_reports_partial_overlap() {
    let api = NodeDataApi::new(vec![node("a", 0.0, 0.0)], vec![]);
    let target = api.get_node("a").unwrap().clone();
    assert!(api.is_node_intersecting(&target, Rect::new(0.0, 0.0, 10.0, 10.0), true, [0.0, 0.0]));
    assert!(!api.is_node_intersecting(
        &target,
        Rect::new(500.0, 500.0, 10.0, 10.0),
        true,
        [0.0, 0.0]
    ));
}
