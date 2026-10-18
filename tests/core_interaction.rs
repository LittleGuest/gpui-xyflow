use gpui_xyflow::core::*;

fn handle_node(id: &str, x: f64, y: f64) -> Node<String> {
    let mut node = Node::new(id, XYPosition::new(x, y), String::new())
        .with_size(100.0, 50.0)
        .with_measured(100.0, 50.0);
    node.handles = vec![
        Handle::new(id, HandleType::Source, Position::Bottom, 50.0, 50.0).with_id("out"),
        Handle::new(id, HandleType::Target, Position::Top, 50.0, 0.0).with_id("in"),
    ];
    node
}

fn lookup(nodes: &[Node<String>]) -> NodeLookup {
    let mut node_lookup = NodeLookup::new();
    let mut parent_lookup = ParentLookup::new();
    adopt_user_nodes(nodes, &mut node_lookup, &mut parent_lookup, &AdoptOptions::default());
    node_lookup
}

#[test]
fn closest_handle_picks_nearby_handle() {
    let nodes = vec![
        handle_node("a", 0.0, 0.0),
        handle_node("b", 0.0, 200.0),
    ];
    let node_lookup = lookup(&nodes);
    let from = node_lookup["a"]
        .internals
        .handle_bounds
        .as_ref()
        .unwrap()
        .source[0]
        .clone();

    let closest = get_closest_handle(
        XYPosition::new(50.0, 200.0),
        20.0,
        &node_lookup,
        &from,
    );

    assert_eq!(closest.map(|handle| handle.node_id), Some("b".to_string()));
}

#[test]
fn closest_handle_returns_none_out_of_radius() {
    let nodes = vec![
        handle_node("a", 0.0, 0.0),
        handle_node("b", 0.0, 2000.0),
    ];
    let node_lookup = lookup(&nodes);
    let from = node_lookup["a"]
        .internals
        .handle_bounds
        .as_ref()
        .unwrap()
        .source[0]
        .clone();

    assert!(get_closest_handle(XYPosition::new(50.0, 1900.0), 20.0, &node_lookup, &from).is_none());
}

#[test]
fn strict_mode_rejects_same_handle_type() {
    let nodes = vec![handle_node("a", 0.0, 0.0), handle_node("b", 0.0, 200.0)];
    let node_lookup = lookup(&nodes);
    let source_handle = node_lookup["b"]
        .internals
        .handle_bounds
        .as_ref()
        .unwrap()
        .source[0]
        .clone();

    let always = |_: &Connection| true;
    let result = is_valid_handle(ValidHandleParams {
        handle: Some(&source_handle),
        connection_mode: ConnectionMode::Strict,
        from_node_id: "a",
        from_handle_id: Some("out"),
        from_type: HandleType::Source,
        is_valid_connection: &always,
        node_lookup: &node_lookup,
    });

    assert!(!result.is_valid);
}

#[test]
fn strict_mode_accepts_source_to_target() {
    let nodes = vec![handle_node("a", 0.0, 0.0), handle_node("b", 0.0, 200.0)];
    let node_lookup = lookup(&nodes);
    let target_handle = node_lookup["b"]
        .internals
        .handle_bounds
        .as_ref()
        .unwrap()
        .target[0]
        .clone();

    let always = |_: &Connection| true;
    let result = is_valid_handle(ValidHandleParams {
        handle: Some(&target_handle),
        connection_mode: ConnectionMode::Strict,
        from_node_id: "a",
        from_handle_id: Some("out"),
        from_type: HandleType::Source,
        is_valid_connection: &always,
        node_lookup: &node_lookup,
    });

    assert!(result.is_valid);
    let connection = result.connection.expect("connection");
    assert_eq!(connection.source, "a");
    assert_eq!(connection.target, "b");
    assert_eq!(connection.source_handle.as_deref(), Some("out"));
    assert_eq!(connection.target_handle.as_deref(), Some("in"));
}

#[test]
fn loose_mode_allows_same_handle_type_across_nodes() {
    let nodes = vec![handle_node("a", 0.0, 0.0), handle_node("b", 0.0, 200.0)];
    let node_lookup = lookup(&nodes);
    let source_handle = node_lookup["b"]
        .internals
        .handle_bounds
        .as_ref()
        .unwrap()
        .source[0]
        .clone();

    let always = |_: &Connection| true;
    let result = is_valid_handle(ValidHandleParams {
        handle: Some(&source_handle),
        connection_mode: ConnectionMode::Loose,
        from_node_id: "a",
        from_handle_id: Some("out"),
        from_type: HandleType::Source,
        is_valid_connection: &always,
        node_lookup: &node_lookup,
    });

    assert!(result.is_valid);
}

#[test]
fn loose_mode_rejects_identical_handle() {
    let nodes = vec![handle_node("a", 0.0, 0.0)];
    let node_lookup = lookup(&nodes);
    let source_handle = node_lookup["a"]
        .internals
        .handle_bounds
        .as_ref()
        .unwrap()
        .source[0]
        .clone();

    let always = |_: &Connection| true;
    let result = is_valid_handle(ValidHandleParams {
        handle: Some(&source_handle),
        connection_mode: ConnectionMode::Loose,
        from_node_id: "a",
        from_handle_id: Some("out"),
        from_type: HandleType::Source,
        is_valid_connection: &always,
        node_lookup: &node_lookup,
    });

    assert!(!result.is_valid);
}

#[test]
fn custom_is_valid_connection_can_reject() {
    let nodes = vec![handle_node("a", 0.0, 0.0), handle_node("b", 0.0, 200.0)];
    let node_lookup = lookup(&nodes);
    let target_handle = node_lookup["b"]
        .internals
        .handle_bounds
        .as_ref()
        .unwrap()
        .target[0]
        .clone();

    let never = |_: &Connection| false;
    let result = is_valid_handle(ValidHandleParams {
        handle: Some(&target_handle),
        connection_mode: ConnectionMode::Strict,
        from_node_id: "a",
        from_handle_id: Some("out"),
        from_type: HandleType::Source,
        is_valid_connection: &never,
        node_lookup: &node_lookup,
    });

    assert!(!result.is_valid);
    assert!(result.connection.is_none());
}

#[test]
fn connection_validity_three_state() {
    assert_eq!(is_connection_valid(true, true), Some(true));
    assert_eq!(is_connection_valid(true, false), Some(false));
    assert_eq!(is_connection_valid(false, false), None);
}

#[test]
fn start_connection_initialises_state() {
    let nodes = vec![handle_node("a", 0.0, 0.0), handle_node("b", 0.0, 200.0)];
    let node_lookup = lookup(&nodes);

    let state = start_connection(StartConnectionParams {
        node_id: "a",
        handle_id: Some("out"),
        handle_type: HandleType::Source,
        pointer: XYPosition::new(50.0, 60.0),
        transform: Transform::IDENTITY,
        connection_mode: ConnectionMode::Strict,
        connection_radius: 20.0,
        node_lookup: &node_lookup,
    })
    .expect("started");

    assert_eq!(state.from, XYPosition::new(50.5, 50.5));
    assert_eq!(state.to, XYPosition::new(50.0, 60.0));
    assert_eq!(state.from_position, Position::Bottom);
    assert_eq!(state.to_position, Position::Top);
    assert_eq!(state.is_valid, None);
}

#[test]
fn update_connection_snaps_to_target_handle() {
    let nodes = vec![handle_node("a", 0.0, 0.0), handle_node("b", 0.0, 200.0)];
    let node_lookup = lookup(&nodes);

    let state = start_connection(StartConnectionParams {
        node_id: "a",
        handle_id: Some("out"),
        handle_type: HandleType::Source,
        pointer: XYPosition::new(50.0, 60.0),
        transform: Transform::IDENTITY,
        connection_mode: ConnectionMode::Strict,
        connection_radius: 20.0,
        node_lookup: &node_lookup,
    })
    .expect("started");

    let always = |_: &Connection| true;
    let updated = update_connection(UpdateConnectionParams {
        state: &state,
        pointer: XYPosition::new(52.0, 199.0),
        transform: Transform::IDENTITY,
        connection_mode: ConnectionMode::Strict,
        connection_radius: 20.0,
        is_valid_connection: &always,
        node_lookup: &node_lookup,
    });

    assert_eq!(updated.is_valid, Some(true));
    assert_eq!(updated.to, XYPosition::new(50.5, 200.5));
    assert_eq!(updated.to_node.map(|node| node.id), Some("b".to_string()));
    assert_eq!(updated.to_position, Position::Top);
}

#[test]
fn update_connection_without_target_uses_pointer() {
    let nodes = vec![handle_node("a", 0.0, 0.0)];
    let node_lookup = lookup(&nodes);

    let state = start_connection(StartConnectionParams {
        node_id: "a",
        handle_id: Some("out"),
        handle_type: HandleType::Source,
        pointer: XYPosition::new(50.0, 60.0),
        transform: Transform::IDENTITY,
        connection_mode: ConnectionMode::Strict,
        connection_radius: 20.0,
        node_lookup: &node_lookup,
    })
    .expect("started");

    let always = |_: &Connection| true;
    let updated = update_connection(UpdateConnectionParams {
        state: &state,
        pointer: XYPosition::new(900.0, 900.0),
        transform: Transform::IDENTITY,
        connection_mode: ConnectionMode::Strict,
        connection_radius: 20.0,
        is_valid_connection: &always,
        node_lookup: &node_lookup,
    });

    assert_eq!(updated.is_valid, None);
    assert_eq!(updated.to, XYPosition::new(900.0, 900.0));
    assert_eq!(updated.to_handle, None);
    assert_eq!(updated.to_position, Position::Top);
}

#[test]
fn click_connection_matches_drag_result() {
    let nodes = vec![handle_node("a", 0.0, 0.0), handle_node("b", 0.0, 200.0)];
    let node_lookup = lookup(&nodes);
    let always = |_: &Connection| true;

    let result = check_click_connection(CheckHandleParams {
        node_id: "b",
        handle_id: Some("in"),
        handle_type: HandleType::Target,
        from_node_id: "a",
        from_handle_id: Some("out"),
        from_type: HandleType::Source,
        connection_mode: ConnectionMode::Strict,
        is_valid_connection: &always,
        node_lookup: &node_lookup,
    });

    assert!(result.is_valid);
    assert_eq!(
        result.connection.expect("connection").target,
        "b".to_string()
    );
}

#[test]
fn connection_line_reports_idle_when_not_connecting() {
    let state: ConnectionState = ConnectionState::default();
    assert!(!state.is_in_progress());
    assert!(connection_line(&state).is_none());
}

#[test]
fn calculate_node_position_applies_origin_and_extent() {
    let mut nodes = vec![
        Node::new("a", XYPosition::new(0.0, 0.0), String::new())
            .with_size(100.0, 50.0)
            .with_measured(100.0, 50.0)
            .with_origin([0.5, 0.5]),
    ];
    nodes[0].extent = Some(NodeExtent::Coordinates([[0.0, 0.0], [200.0, 200.0]]));
    let node_lookup = lookup(&nodes);

    let update = calculate_node_position(CalculateNodePositionParams {
        node_id: "a",
        next_position: XYPosition::new(500.0, 500.0),
        node_lookup: &node_lookup,
        node_origin: DEFAULT_NODE_ORIGIN,
        node_extent: INFINITE_EXTENT,
    })
    .expect("update");

    assert_eq!(update.position_absolute, XYPosition::new(100.0, 150.0));
    assert_eq!(update.position, XYPosition::new(150.0, 175.0));
}

#[test]
fn calculate_node_position_respects_parent() {
    let parent = Node::new("parent", XYPosition::new(100.0, 100.0), String::new())
        .with_size(400.0, 400.0)
        .with_measured(400.0, 400.0);
    let child = Node::new("child", XYPosition::new(10.0, 10.0), String::new())
        .with_size(50.0, 50.0)
        .with_measured(50.0, 50.0)
        .with_parent("parent")
        .with_extent(NodeExtent::Parent);
    let node_lookup = lookup(&[parent, child]);

    let update = calculate_node_position(CalculateNodePositionParams {
        node_id: "child",
        next_position: XYPosition::new(-500.0, -500.0),
        node_lookup: &node_lookup,
        node_origin: DEFAULT_NODE_ORIGIN,
        node_extent: INFINITE_EXTENT,
    })
    .expect("update");

    assert_eq!(update.position_absolute, XYPosition::new(100.0, 100.0));
    assert_eq!(update.position, XYPosition::new(0.0, 0.0));
}

#[test]
fn update_nodes_moves_selection_by_pointer_delta() {
    let mut nodes = vec![
        Node::new("a", XYPosition::new(10.0, 10.0), String::new())
            .with_size(100.0, 50.0)
            .with_measured(100.0, 50.0)
            .selected(true),
        Node::new("b", XYPosition::new(200.0, 10.0), String::new())
            .with_size(100.0, 50.0)
            .with_measured(100.0, 50.0)
            .selected(true),
    ];
    nodes[0].draggable = Some(true);
    nodes[1].draggable = Some(true);
    let node_lookup = lookup(&nodes);

    let store = DragStore {
        node_lookup: node_lookup.clone(),
        ..DragStore::default()
    };
    let mut drag_items = get_drag_items(&node_lookup, true, XYPosition::new(20.0, 20.0), None);
    assert_eq!(drag_items.len(), 2);

    let update = update_nodes(
        UpdateNodesParams {
            pointer: XYPosition::new(70.0, 60.0),
            store: &store,
        },
        &mut drag_items,
    );

    assert!(update.changed);
    assert_eq!(
        update.items["a"].position_absolute,
        XYPosition::new(60.0, 50.0)
    );
    assert_eq!(
        update.items["b"].position_absolute,
        XYPosition::new(250.0, 50.0)
    );
}

#[test]
fn update_nodes_snaps_to_grid() {
    let nodes = vec![Node::new("a", XYPosition::new(0.0, 0.0), String::new())
        .with_size(100.0, 50.0)
        .with_measured(100.0, 50.0)
        .selected(true)];
    let node_lookup = lookup(&nodes);

    let store = DragStore {
        node_lookup: node_lookup.clone(),
        snap_to_grid: true,
        snap_grid: [15.0, 15.0],
        ..DragStore::default()
    };
    let mut drag_items = get_drag_items(&node_lookup, true, XYPosition::new(0.0, 0.0), Some("a"));

    let update = update_nodes(
        UpdateNodesParams {
            pointer: XYPosition::new(22.0, 23.0),
            store: &store,
        },
        &mut drag_items,
    );

    assert_eq!(
        update.items["a"].position_absolute,
        XYPosition::new(15.0, 30.0)
    );
}

#[test]
fn update_nodes_reports_no_change_when_pointer_static() {
    let nodes = vec![Node::new("a", XYPosition::new(0.0, 0.0), String::new())
        .with_size(100.0, 50.0)
        .with_measured(100.0, 50.0)
        .selected(true)];
    let node_lookup = lookup(&nodes);
    let store = DragStore {
        node_lookup: node_lookup.clone(),
        ..DragStore::default()
    };
    let mut drag_items = get_drag_items(&node_lookup, true, XYPosition::new(10.0, 10.0), Some("a"));

    let update = update_nodes(
        UpdateNodesParams {
            pointer: XYPosition::new(10.0, 10.0),
            store: &store,
        },
        &mut drag_items,
    );

    assert!(!update.changed);
}

#[test]
fn update_nodes_records_parent_expansion() {
    let parent = Node::new("parent", XYPosition::new(0.0, 0.0), String::new())
        .with_size(100.0, 100.0)
        .with_measured(100.0, 100.0);
    let child = Node::new("child", XYPosition::new(50.0, 50.0), String::new())
        .with_size(50.0, 50.0)
        .with_measured(50.0, 50.0)
        .with_parent("parent")
        .with_expand_parent(true);
    let node_lookup = lookup(&[parent, child]);
    let store = DragStore {
        node_lookup: node_lookup.clone(),
        ..DragStore::default()
    };
    let mut drag_items = get_drag_items(&node_lookup, true, XYPosition::new(60.0, 60.0), Some("child"));

    let update = update_nodes(
        UpdateNodesParams {
            pointer: XYPosition::new(200.0, 200.0),
            store: &store,
        },
        &mut drag_items,
    );

    assert_eq!(update.parent_expansions.len(), 1);
    assert_eq!(update.parent_expansions[0].1, "parent".to_string());
}

#[test]
fn snap_offset_aligns_multiple_nodes() {
    let nodes = vec![
        Node::new("a", XYPosition::new(3.0, 4.0), String::new())
            .with_size(10.0, 10.0)
            .with_measured(10.0, 10.0)
            .selected(true),
        Node::new("b", XYPosition::new(100.0, 100.0), String::new())
            .with_size(10.0, 10.0)
            .with_measured(10.0, 10.0)
            .selected(true),
    ];
    let node_lookup = lookup(&nodes);
    let drag_items = get_drag_items(&node_lookup, true, XYPosition::new(5.0, 5.0), None);

    let offset = snap_offset_for_selection(&drag_items, [15.0, 15.0], XYPosition::new(8.0, 9.0))
        .expect("offset");
    let reference = XYPosition::new(8.0 - 2.0, 9.0 - 1.0);
    let snapped = snap_position(reference, [15.0, 15.0]);
    assert_eq!(offset, XYPosition::new(snapped.x - reference.x, snapped.y - reference.y));
}
