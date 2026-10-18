use gpui_xyflow::core::{
    connections::{
        ConnectionLookup, add_connection_to_lookup, are_connection_maps_equal, connection_status,
        get_connection_key, handle_connection_change, update_connection_lookup,
    },
    edge::{Connection, Edge, EdgeLookup, HandleConnection},
    geometry::{Rect, XYPosition},
    graph::{
        FitViewOptions, fit_viewport, get_elements_to_remove, get_fit_view_nodes,
        get_node_position_with_origin,
    },
    node::{Node, NodeLookup, ParentLookup},
    store::{AdoptOptions, adopt_user_nodes},
};

fn node(id: &str, x: f64, y: f64, w: f64, h: f64) -> Node<String> {
    Node::<String> {
        id: id.to_string(),
        position: XYPosition::new(x, y),
        width: Some(w),
        height: Some(h),
        measured: gpui_xyflow::core::node::MeasuredDimensions::new(w, h),
        ..Node::default()
    }
}

fn build_lookup(nodes: Vec<Node<String>>) -> NodeLookup<String> {
    let mut lookup = NodeLookup::new();
    let mut parent_lookup = ParentLookup::new();
    adopt_user_nodes(
        &nodes,
        &mut lookup,
        &mut parent_lookup,
        &AdoptOptions::default(),
    );
    lookup
}

#[test]
fn connection_key_uses_double_dash_separator() {
    assert_eq!(get_connection_key("a", Some("h1"), "b", None), "a-h1--b-");
    assert_eq!(get_connection_key("a", None, "b", None), "a---b-");
}

#[test]
fn add_connection_populates_three_levels() {
    let mut lookup = ConnectionLookup::new();
    let connection = HandleConnection {
        connection: Connection::new("a", "b"),
        edge_id: "e1".to_string(),
    };
    let key = get_connection_key("a", Some("h1"), "b", Some("h2"));

    add_connection_to_lookup("source", &connection, &key, &mut lookup, "a", Some("h1"));

    assert!(lookup.contains_key("a"));
    assert!(lookup.contains_key("a-source"));
    assert!(lookup.contains_key("a-source-h1"));
    assert_eq!(lookup["a-source-h1"][&key].edge_id, "e1");
}

#[test]
fn update_connection_lookup_rebuilds_from_edges() {
    let mut lookup = ConnectionLookup::new();
    let mut edge_lookup = EdgeLookup::new();
    let edges = vec![Edge::<String> {
        id: "e1".to_string(),
        source: "a".to_string(),
        target: "b".to_string(),
        source_handle: Some("sh".to_string()),
        ..Edge::default()
    }];

    update_connection_lookup(&mut lookup, &mut edge_lookup, &edges);

    assert_eq!(edge_lookup.len(), 1);
    assert!(lookup.contains_key("a-source-sh"));
    assert!(lookup.contains_key("b-target"));
    assert!(!lookup.contains_key("b-target-sh"));
    assert!(lookup.contains_key("a"));
    assert!(lookup.contains_key("b"));
}

#[test]
fn connection_maps_equal_compares_key_sets() {
    let make = |id: &str| {
        let mut map = std::collections::BTreeMap::new();
        map.insert(
            "key".to_string(),
            HandleConnection {
                connection: Connection::new("a", "b"),
                edge_id: id.to_string(),
            },
        );
        map
    };
    let a = make("e1");
    let b = make("e2");
    assert!(are_connection_maps_equal(Some(&a), Some(&b)));
    assert!(are_connection_maps_equal(None, None));
    assert!(!are_connection_maps_equal(Some(&a), None));
}

#[test]
fn handle_connection_change_reports_removed_keys() {
    let a = {
        let mut map = std::collections::BTreeMap::new();
        map.insert(
            "keep".to_string(),
            HandleConnection {
                connection: Connection::new("a", "b"),
                edge_id: "e1".to_string(),
            },
        );
        map.insert(
            "drop".to_string(),
            HandleConnection {
                connection: Connection::new("a", "c"),
                edge_id: "e2".to_string(),
            },
        );
        map
    };
    let b = {
        let mut map = std::collections::BTreeMap::new();
        map.insert(
            "keep".to_string(),
            HandleConnection {
                connection: Connection::new("a", "b"),
                edge_id: "e1".to_string(),
            },
        );
        map
    };

    let mut collected: Vec<String> = Vec::new();
    handle_connection_change(&a, &b, |diff| {
        collected = diff.iter().map(|c| c.edge_id.clone()).collect();
    });
    assert_eq!(collected, vec!["e2"]);
}

#[test]
fn connection_status_maps_option_bool() {
    assert_eq!(connection_status(None), None);
    assert_eq!(connection_status(Some(true)), Some("valid"));
    assert_eq!(connection_status(Some(false)), Some("invalid"));
}

#[test]
fn node_position_with_origin_subtracts_dimension_scaled_offset() {
    let mut n = node("a", 100.0, 200.0, 80.0, 40.0);
    n.origin = Some([0.5, 0.5]);
    let pos = get_node_position_with_origin(&n, [0.0, 0.0]);
    assert_eq!(pos.x, 60.0);
    assert_eq!(pos.y, 180.0);
}

#[test]
fn fit_view_nodes_excludes_unmeasured_by_default() {
    let mut unmeasured = node("b", 500.0, 500.0, 80.0, 40.0);
    unmeasured.measured = gpui_xyflow::core::node::MeasuredDimensions::default();
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, 80.0, 40.0), unmeasured]);
    let fit = get_fit_view_nodes(&lookup, &FitViewOptions::default());
    assert_eq!(fit.len(), 1);
    assert_eq!(fit[0].id, "a");
}

#[test]
fn fit_view_nodes_includes_hidden_when_requested() {
    let mut hidden = node("b", 500.0, 500.0, 80.0, 40.0);
    hidden.hidden = true;
    hidden.measured = gpui_xyflow::core::node::MeasuredDimensions::default();
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, 80.0, 40.0), hidden]);
    let options = FitViewOptions {
        include_hidden_nodes: true,
        ..FitViewOptions::default()
    };
    let fit = get_fit_view_nodes(&lookup, &options);
    assert_eq!(fit.len(), 2);
}

#[test]
fn fit_viewport_returns_none_for_empty_lookup() {
    let lookup = build_lookup(vec![]);
    assert!(fit_viewport(&lookup, 800.0, 600.0, 0.5, 2.0, &FitViewOptions::default()).is_none());
}

#[test]
fn fit_viewport_produces_bounded_viewport() {
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, 100.0, 100.0)]);
    let vp = fit_viewport(&lookup, 800.0, 600.0, 0.5, 2.0, &FitViewOptions::default())
        .expect("viewport computed");
    assert!(vp.zoom >= 0.5);
    assert!(vp.zoom <= 2.0);
    assert!(vp.x.is_finite());
    assert!(vp.y.is_finite());
}

#[test]
fn get_elements_to_remove_cascades_to_children() {
    let parent = node("p", 0.0, 0.0, 100.0, 100.0);
    let mut child = node("c", 10.0, 10.0, 50.0, 50.0);
    child.parent_id = Some("p".to_string());
    let other = node("o", 500.0, 500.0, 50.0, 50.0);

    let (removed_nodes, _) = get_elements_to_remove(
        std::slice::from_ref(&parent),
        &[],
        &[parent.clone(), child, other],
        &[],
    );
    let ids: Vec<&str> = removed_nodes.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&"p"));
    assert!(ids.contains(&"c"));
    assert!(!ids.contains(&"o"));
}

#[test]
fn get_elements_to_remove_skips_undeletable() {
    let mut locked = node("a", 0.0, 0.0, 100.0, 100.0);
    locked.deletable = Some(false);
    let (removed_nodes, _) =
        get_elements_to_remove(std::slice::from_ref(&locked), &[], &[locked.clone()], &[]);
    assert!(removed_nodes.is_empty());
}

#[test]
fn get_elements_to_remove_collects_connected_edges() {
    let a = node("a", 0.0, 0.0, 50.0, 50.0);
    let b = node("b", 100.0, 0.0, 50.0, 50.0);
    let c = node("c", 200.0, 0.0, 50.0, 50.0);
    let edges = vec![
        Edge::<String>::new("e1", "a", "b"),
        Edge::<String>::new("e2", "b", "c"),
    ];

    let (_, removed_edges) =
        get_elements_to_remove(std::slice::from_ref(&a), &[], &[a.clone(), b, c], &edges);
    let ids: Vec<&str> = removed_edges.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["e1"]);
}

#[test]
fn rect_helpers_remain_usable() {
    let _ = Rect::new(0.0, 0.0, 1.0, 1.0);
}
