use gpui_xyflow::core::*;

fn node(id: &str, x: f64, y: f64) -> Node<String> {
    Node::new(id, XYPosition::new(x, y), String::new()).with_size(150.0, 40.0)
}

#[test]
fn node_dimensions_prefer_measured_then_width_then_initial() {
    let node = Node::new("a", XYPosition::ZERO, String::new())
        .with_initial_size(10.0, 20.0)
        .with_size(30.0, 40.0);
    assert_eq!(node.size(), Dimensions::new(30.0, 40.0));

    let measured = node.with_measured(50.0, 60.0);
    assert_eq!(measured.size(), Dimensions::new(50.0, 60.0));

    let initial_only = Node::new("b", XYPosition::ZERO, String::new()).with_initial_size(7.0, 9.0);
    assert_eq!(initial_only.size(), Dimensions::new(7.0, 9.0));
}

#[test]
fn node_position_with_origin_centers_for_half_origin() {
    let node = node("a", 100.0, 100.0).with_origin([0.5, 0.5]);
    let position = node.position_with_origin(DEFAULT_NODE_ORIGIN);
    assert_eq!(position, XYPosition::new(25.0, 80.0));
}

#[test]
fn node_flags_fall_back_to_flow_defaults() {
    let node = node("a", 0.0, 0.0);
    assert!(node.is_draggable(true));
    assert!(!node.is_draggable(false));
    assert!(node.is_deletable());

    let mut fixed = node;
    fixed.draggable = Some(false);
    assert!(!fixed.is_draggable(true));
}

#[test]
fn apply_node_changes_handles_position_select_and_dimensions() {
    let nodes = vec![node("a", 0.0, 0.0), node("b", 10.0, 10.0)];

    let changes = vec![
        NodeChange::position("a", 50.0, 60.0, true),
        NodeChange::select("a", true),
        NodeChange::dimensions("b", 20.0, 30.0),
    ];

    let updated = apply_node_changes(&changes, &nodes);

    assert_eq!(updated.len(), 2);
    assert_eq!(updated[0].position, XYPosition::new(50.0, 60.0));
    assert!(updated[0].selected);
    assert!(updated[0].dragging);
    assert_eq!(updated[1].measured.width, Some(20.0));
    assert_eq!(updated[1].size(), Dimensions::new(20.0, 30.0));
}

#[test]
fn apply_node_changes_removes_and_adds_with_index() {
    let nodes = vec![node("a", 0.0, 0.0), node("b", 10.0, 10.0)];
    let changes = vec![
        NodeChange::remove("a"),
        NodeChange::add(node("c", 5.0, 5.0), Some(0)),
        NodeChange::add(node("d", 6.0, 6.0), None),
    ];

    let updated = apply_node_changes(&changes, &nodes);
    let ids: Vec<&str> = updated.iter().map(|node| node.id.as_str()).collect();
    assert_eq!(ids, vec!["c", "b", "d"]);
}

#[test]
fn apply_node_changes_ignores_later_changes_after_remove() {
    let nodes = vec![node("a", 0.0, 0.0)];
    let changes = vec![
        NodeChange::select("a", true),
        NodeChange::remove("a"),
        NodeChange::position("a", 99.0, 99.0, false),
    ];
    let updated = apply_node_changes(&changes, &nodes);
    assert!(updated.is_empty());
}

#[test]
fn apply_node_changes_replace_wins() {
    let nodes = vec![node("a", 0.0, 0.0)];
    let changes = vec![NodeChange::replace("a", node("a", 42.0, 43.0))];
    let updated = apply_node_changes(&changes, &nodes);
    assert_eq!(updated[0].position, XYPosition::new(42.0, 43.0));
}

#[test]
fn apply_node_changes_dimensions_set_attributes() {
    let nodes = vec![node("a", 0.0, 0.0)];
    let change = NodeChange::Dimensions(NodeDimensionChange {
        id: "a".to_string(),
        dimensions: Some(Dimensions::new(120.0, 80.0)),
        resizing: Some(true),
        set_attributes: Some(SetAttributes::Width),
    });

    let updated = apply_node_changes(&[change], &nodes);
    assert_eq!(updated[0].width, Some(120.0));
    assert_eq!(updated[0].height, Some(40.0));
}

#[test]
fn apply_edge_changes_selection_and_removal() {
    let edges: Vec<Edge<String>> = vec![
        Edge::new("e1", "a", "b"),
        Edge::new("e2", "b", "c"),
        Edge::new("e3", "c", "d"),
    ];

    let changes = vec![
        EdgeChange::select("e1", true),
        EdgeChange::remove("e2"),
        EdgeChange::add(Edge::new("e4", "d", "e"), Some(0)),
    ];

    let updated = apply_edge_changes(&changes, &edges);
    let ids: Vec<&str> = updated.iter().map(|edge| edge.id.as_str()).collect();
    assert_eq!(ids, vec!["e4", "e1", "e3"]);
    assert!(updated[1].selected);
}

#[test]
fn edge_change_kinds_are_reported() {
    let edge_change: EdgeChange<String> = EdgeChange::select("a", true);
    assert_eq!(edge_change.kind(), ChangeKind::Select);
    assert_eq!(edge_change.id(), "a");

    let node_change: NodeChange<String> = NodeChange::position("n", 1.0, 2.0, true);
    assert_eq!(node_change.kind(), ChangeKind::Position);
    assert_eq!(node_change.id(), "n");
}

#[test]
fn adopt_user_nodes_computes_absolute_positions() {
    let nodes = vec![node("a", 100.0, 200.0)];
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();

    let result = adopt_user_nodes(&nodes, &mut lookup, &mut parents, &AdoptOptions::default());

    assert!(!result.nodes_initialized);
    assert!(!result.has_selected_nodes);
    assert_eq!(
        lookup["a"].internals.position_absolute,
        XYPosition::new(100.0, 200.0)
    );
}

#[test]
fn adopt_user_nodes_reports_initialized_when_measured() {
    let nodes = vec![node("a", 0.0, 0.0).with_measured(150.0, 40.0)];
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    let result = adopt_user_nodes(&nodes, &mut lookup, &mut parents, &AdoptOptions::default());
    assert!(result.nodes_initialized);
}

#[test]
fn adopt_user_nodes_clamps_to_node_extent() {
    let nodes = vec![node("a", -500.0, 900.0)];
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    let options = AdoptOptions {
        node_extent: [[0.0, 0.0], [1000.0, 1000.0]],
        ..AdoptOptions::default()
    };

    adopt_user_nodes(&nodes, &mut lookup, &mut parents, &options);

    assert_eq!(
        lookup["a"].internals.position_absolute,
        XYPosition::new(0.0, 900.0)
    );
}

#[test]
fn adopt_user_nodes_resolves_child_absolute_position() {
    let parent = node("parent", 100.0, 50.0).with_measured(400.0, 300.0);
    let child = node("child", 20.0, 30.0)
        .with_measured(100.0, 20.0)
        .with_parent("parent");
    let nodes = vec![parent, child];

    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    adopt_user_nodes(&nodes, &mut lookup, &mut parents, &AdoptOptions::default());

    assert_eq!(
        lookup["child"].internals.position_absolute,
        XYPosition::new(120.0, 80.0)
    );
    assert!(parents.contains_key("parent"));
    assert_eq!(parents["parent"].len(), 1);
}

#[test]
fn child_z_index_is_above_parent() {
    let parent = node("parent", 0.0, 0.0).with_measured(400.0, 300.0);
    let child = node("child", 10.0, 10.0)
        .with_measured(50.0, 50.0)
        .with_parent("parent");
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    adopt_user_nodes(
        &[parent, child],
        &mut lookup,
        &mut parents,
        &AdoptOptions::default(),
    );

    assert!(lookup["child"].internals.z > lookup["parent"].internals.z);
}

#[test]
fn selected_node_is_elevated_by_default() {
    let plain = node("a", 0.0, 0.0);
    let selected = node("b", 0.0, 0.0).selected(true);
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    adopt_user_nodes(
        &[plain, selected],
        &mut lookup,
        &mut parents,
        &AdoptOptions::default(),
    );

    assert_eq!(lookup["a"].internals.z, 0.0);
    assert_eq!(lookup["b"].internals.z, SELECTED_NODE_Z);
}

#[test]
fn manual_z_index_mode_ignores_selection() {
    let selected = node("b", 0.0, 0.0).selected(true).with_z_index(7.0);
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    adopt_user_nodes(
        &[selected],
        &mut lookup,
        &mut parents,
        &AdoptOptions {
            z_index_mode: ZIndexMode::Manual,
            ..AdoptOptions::default()
        },
    );

    assert_eq!(lookup["b"].internals.z, 7.0);
}

#[test]
fn expand_parent_grows_parent_for_child() {
    let parent = node("parent", 0.0, 0.0).with_measured(200.0, 200.0);
    let child = node("child", 150.0, 150.0)
        .with_measured(100.0, 100.0)
        .with_parent("parent");
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    adopt_user_nodes(
        &[parent, child],
        &mut lookup,
        &mut parents,
        &AdoptOptions::default(),
    );

    let child_rect = Rect::new(150.0, 150.0, 100.0, 100.0);
    let changes = handle_expand_parent(
        &[("child".to_string(), "parent".to_string(), child_rect)],
        &lookup,
        &parents,
        DEFAULT_NODE_ORIGIN,
    );

    let dimensions = changes.iter().find_map(|change| match change {
        NodeChange::Dimensions(change) => change.dimensions,
        _ => None,
    });
    assert_eq!(dimensions, Some(Dimensions::new(250.0, 250.0)));
}

#[test]
fn get_drag_items_only_includes_draggable_selection() {
    let nodes = vec![
        node("a", 0.0, 0.0).selected(true).with_measured(150.0, 40.0),
        node("b", 100.0, 0.0).with_measured(150.0, 40.0),
    ];
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    adopt_user_nodes(&nodes, &mut lookup, &mut parents, &AdoptOptions::default());

    let items = get_drag_items(&lookup, true, XYPosition::new(50.0, 50.0), None);
    assert_eq!(items.len(), 1);
    assert!(items.contains_key("a"));
    assert_eq!(items["a"].distance, XYPosition::new(50.0, 50.0));

    let items_with_target = get_drag_items(&lookup, true, XYPosition::ZERO, Some("b"));
    assert!(items_with_target.contains_key("b"));
}

#[test]
fn get_drag_items_skips_children_of_selected_parent() {
    let parent = node("parent", 0.0, 0.0)
        .selected(true)
        .with_measured(400.0, 400.0);
    let child = node("child", 10.0, 10.0)
        .with_measured(50.0, 50.0)
        .with_parent("parent");
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    adopt_user_nodes(
        &[parent, child],
        &mut lookup,
        &mut parents,
        &AdoptOptions::default(),
    );

    let items = get_drag_items(&lookup, true, XYPosition::ZERO, None);
    assert_eq!(items.len(), 1);
    assert!(items.contains_key("parent"));
}

#[test]
fn update_node_internals_measures_and_reports_change() {
    let nodes = vec![node("a", 10.0, 20.0)];
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    adopt_user_nodes(&nodes, &mut lookup, &mut parents, &AdoptOptions::default());

    let result = update_node_internals(
        &[NodeInternalsUpdate {
            id: "a".to_string(),
            dimensions: Dimensions::new(150.0, 40.0),
            handle_bounds: None,
            force: false,
        }],
        &mut lookup,
        &mut parents,
        &AdoptOptions::default(),
    );

    assert!(result.updated_internals);
    assert_eq!(result.changes.len(), 1);
    assert_eq!(lookup["a"].size(), Dimensions::new(150.0, 40.0));
}

#[test]
fn update_node_internals_ignores_zero_dimensions() {
    let nodes = vec![node("a", 10.0, 20.0)];
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    adopt_user_nodes(&nodes, &mut lookup, &mut parents, &AdoptOptions::default());

    let result = update_node_internals(
        &[NodeInternalsUpdate {
            id: "a".to_string(),
            dimensions: Dimensions::new(0.0, 0.0),
            handle_bounds: None,
            force: true,
        }],
        &mut lookup,
        &mut parents,
        &AdoptOptions::default(),
    );

    assert!(!result.updated_internals);
    assert!(result.changes.is_empty());
}

#[test]
fn update_absolute_positions_reclamps_root_nodes() {
    let nodes = vec![node("a", 500.0, 500.0).with_measured(150.0, 40.0)];
    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    adopt_user_nodes(&nodes, &mut lookup, &mut parents, &AdoptOptions::default());

    let options = AdoptOptions {
        node_extent: [[0.0, 0.0], [300.0, 300.0]],
        ..AdoptOptions::default()
    };
    update_absolute_positions(&mut lookup, &mut parents, &options);

    assert_eq!(
        lookup["a"].internals.position_absolute,
        XYPosition::new(150.0, 260.0)
    );
}

#[test]
fn handles_are_parsed_into_bounds() {
    let mut node = node("a", 0.0, 0.0).with_measured(150.0, 40.0);
    node.handles = vec![
        Handle::new("a", HandleType::Source, Position::Bottom, 75.0, 40.0).with_id("out"),
        Handle::new("a", HandleType::Target, Position::Top, 75.0, 0.0).with_id("in"),
    ];

    let mut lookup = NodeLookup::new();
    let mut parents = ParentLookup::new();
    adopt_user_nodes(&[node], &mut lookup, &mut parents, &AdoptOptions::default());

    let bounds = lookup["a"].internals.handle_bounds.as_ref().expect("bounds");
    assert_eq!(bounds.source.len(), 1);
    assert_eq!(bounds.target.len(), 1);
    assert_eq!(bounds.source[0].id.as_deref(), Some("out"));
    assert_eq!(bounds.source[0].width, 1.0);
}
