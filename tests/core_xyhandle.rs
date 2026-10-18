use gpui_xyflow::core::geometry::{Position, Transform, XYPosition};
use gpui_xyflow::core::graph::ConnectionMode;
use gpui_xyflow::NodeHandleBounds;
use gpui_xyflow::core::node::{Handle, HandleType, InternalNode, Node, NodeLookup};
use gpui_xyflow::core::store::{adopt_user_nodes, AdoptOptions};
use gpui_xyflow::core::xyhandle::{
    get_handle, get_handle_type, handle_for_node, XYHandle, XYHandleOptions, XYHandlePhase,
    XYHandleStartParams,
};

fn handle(node_id: &str, handle_type: HandleType, id: Option<&str>, x: f64, y: f64) -> Handle {
    Handle {
        id: id.map(|s| s.to_string()),
        node_id: node_id.to_string(),
        x,
        y,
        position: if handle_type == HandleType::Source {
            Position::Right
        } else {
            Position::Left
        },
        handle_type,
        width: 0.0,
        height: 0.0,
    }
}

fn build_lookup(nodes: Vec<Node<String>>) -> NodeLookup<String> {
    let mut lookup = NodeLookup::new();
    let mut parent_lookup = std::collections::BTreeMap::new();
    let options = AdoptOptions {
        node_origin: [0.0, 0.0],
        node_extent: [[f64::NEG_INFINITY, f64::NEG_INFINITY], [f64::INFINITY, f64::INFINITY]],
        elevate_nodes_on_select: false,
        z_index_mode: gpui_xyflow::core::geometry::ZIndexMode::Auto,
        check_equality: false,
    };
    adopt_user_nodes(&nodes, &mut lookup, &mut parent_lookup, &options);
    lookup
}

fn node_with_handles(id: &str, x: f64, y: f64, sources: usize, targets: usize) -> Node<String> {
    let mut node = Node::<String>::default();
    node.id = id.to_string();
    node.position = XYPosition::new(x, y);
    node.width = Some(100.0);
    node.height = Some(40.0);
    node.handles = (0..sources)
        .map(|i| handle(id, HandleType::Source, Some(&format!("s{i}")), 100.0, 20.0 + i as f64))
        .chain(
            (0..targets)
                .map(|i| handle(id, HandleType::Target, Some(&format!("t{i}")), 0.0, 20.0 + i as f64)),
        )
        .collect();
    node
}

#[test]
fn get_handle_strict_source_only() {
    let lookup = build_lookup(vec![node_with_handles("a", 0.0, 0.0, 2, 2)]);
    let source = get_handle("a", HandleType::Source, None, &lookup, ConnectionMode::Strict, false);
    assert_eq!(source.unwrap().handle_type, HandleType::Source);
    let target = get_handle("a", HandleType::Target, None, &lookup, ConnectionMode::Strict, false);
    assert_eq!(target.unwrap().handle_type, HandleType::Target);
}

#[test]
fn get_handle_strict_cannot_cross_type() {
    let lookup = build_lookup(vec![node_with_handles("a", 0.0, 0.0, 1, 0)]);
    let target = get_handle("a", HandleType::Target, None, &lookup, ConnectionMode::Strict, false);
    assert!(target.is_none());
}

#[test]
fn get_handle_loose_merges_types() {
    let lookup = build_lookup(vec![node_with_handles("a", 0.0, 0.0, 0, 1)]);
    let source = get_handle("a", HandleType::Source, None, &lookup, ConnectionMode::Loose, false);
    assert!(source.is_some());
}

#[test]
fn get_handle_by_id() {
    let lookup = build_lookup(vec![node_with_handles("a", 0.0, 0.0, 3, 0)]);
    let handle = get_handle("a", HandleType::Source, Some("s1"), &lookup, ConnectionMode::Strict, false);
    assert_eq!(handle.unwrap().id.as_deref(), Some("s1"));
}

#[test]
fn get_handle_missing_id_returns_none() {
    let lookup = build_lookup(vec![node_with_handles("a", 0.0, 0.0, 1, 0)]);
    assert!(get_handle("a", HandleType::Source, Some("nope"), &lookup, ConnectionMode::Strict, false).is_none());
}

#[test]
fn get_handle_missing_node_returns_none() {
    let lookup = build_lookup(vec![]);
    assert!(get_handle("ghost", HandleType::Source, None, &lookup, ConnectionMode::Strict, false).is_none());
}

#[test]
fn get_handle_absolute_position_adds_node_offset() {
    let lookup = build_lookup(vec![node_with_handles("a", 50.0, 100.0, 1, 0)]);
    let relative = get_handle("a", HandleType::Source, None, &lookup, ConnectionMode::Strict, false).unwrap();
    let absolute = get_handle("a", HandleType::Source, None, &lookup, ConnectionMode::Strict, true).unwrap();
    assert!((relative.x - 100.0).abs() < 1.0);
    assert!(absolute.x >= 150.0);
    assert!(absolute.y > relative.y);
}

#[test]
fn get_handle_type_prefers_explicit() {
    let classes = vec!["target".to_string()];
    assert_eq!(
        get_handle_type(Some(HandleType::Source), &classes),
        Some(HandleType::Source)
    );
}

#[test]
fn get_handle_type_reads_class_names() {
    assert_eq!(
        get_handle_type(None, &["gpui-flow__handle target".to_string()]),
        Some(HandleType::Target)
    );
    assert_eq!(
        get_handle_type(None, &["gpui-flow__handle source".to_string()]),
        Some(HandleType::Source)
    );
    assert_eq!(get_handle_type(None, &[]), None);
}

#[test]
fn handle_for_node_finds_by_type() {
    let lookup = build_lookup(vec![node_with_handles("a", 0.0, 0.0, 1, 1)]);
    let node = &lookup["a"];
    assert!(handle_for_node(node, HandleType::Source, None).is_some());
    assert!(handle_for_node(node, HandleType::Target, Some("t0")).is_some());
    assert!(handle_for_node(node, HandleType::Target, Some("s0")).is_none());
}

#[test]
fn xyhandle_starts_connection_on_pointer_down() {
    let lookup = build_lookup(vec![
        node_with_handles("a", 0.0, 0.0, 1, 0),
        node_with_handles("b", 300.0, 0.0, 0, 1),
    ]);
    let mut handle = XYHandle::<String>::new(XYHandleOptions::default());
    let started = handle.on_pointer_down(
        XYHandleStartParams {
            node_id: "a",
            handle_id: Some("s0"),
            handle_type: HandleType::Source,
            pointer: XYPosition::new(100.0, 20.0),
            transform: Transform::IDENTITY,
        },
        &lookup,
    );
    assert!(started);
    assert!(handle.is_connecting());
    assert_eq!(handle.phase(), &XYHandlePhase::Dragging);
    assert_eq!(handle.connection().from_handle().unwrap().node_id, "a");
}

#[test]
fn xyhandle_pointer_down_unknown_node_fails() {
    let lookup = build_lookup(vec![]);
    let mut handle = XYHandle::<String>::new(XYHandleOptions::default());
    let started = handle.on_pointer_down(
        XYHandleStartParams {
            node_id: "ghost",
            handle_id: None,
            handle_type: HandleType::Source,
            pointer: XYPosition::new(0.0, 0.0),
            transform: Transform::IDENTITY,
        },
        &lookup,
    );
    assert!(!started);
    assert!(!handle.is_connecting());
}

#[test]
fn xyhandle_pointer_up_returns_state_and_resets() {
    let lookup = build_lookup(vec![node_with_handles("a", 0.0, 0.0, 1, 0)]);
    let mut handle = XYHandle::<String>::new(XYHandleOptions::default());
    handle.on_pointer_down(
        XYHandleStartParams {
            node_id: "a",
            handle_id: Some("s0"),
            handle_type: HandleType::Source,
            pointer: XYPosition::new(100.0, 20.0),
            transform: Transform::IDENTITY,
        },
        &lookup,
    );
    let state = handle.on_pointer_up();
    assert!(state.is_in_progress());
    assert!(!handle.is_connecting());
    assert_eq!(handle.phase(), &XYHandlePhase::Idle);
}

#[test]
fn xyhandle_move_finds_target_handle() {
    let lookup = build_lookup(vec![
        node_with_handles("a", 0.0, 0.0, 1, 0),
        node_with_handles("b", 120.0, 0.0, 0, 1),
    ]);
    let mut handle = XYHandle::<String>::new(XYHandleOptions::default());
    handle.on_pointer_down(
        XYHandleStartParams {
            node_id: "a",
            handle_id: Some("s0"),
            handle_type: HandleType::Source,
            pointer: XYPosition::new(100.0, 20.0),
            transform: Transform::IDENTITY,
        },
        &lookup,
    );
    let always_valid = |_: &gpui_xyflow::core::edge::Connection| true;
    let state = handle
        .on_pointer_move(XYPosition::new(120.0, 20.0), Transform::IDENTITY, &lookup, &always_valid)
        .unwrap();
    assert_eq!(state.to_handle.as_ref().unwrap().node_id, "b");
    assert_eq!(state.is_valid, Some(true));
}

#[test]
fn xyhandle_move_without_target_is_invalid() {
    let lookup = build_lookup(vec![node_with_handles("a", 0.0, 0.0, 1, 0)]);
    let mut handle = XYHandle::<String>::new(XYHandleOptions::default());
    handle.on_pointer_down(
        XYHandleStartParams {
            node_id: "a",
            handle_id: Some("s0"),
            handle_type: HandleType::Source,
            pointer: XYPosition::new(100.0, 20.0),
            transform: Transform::IDENTITY,
        },
        &lookup,
    );
    let always_valid = |_: &gpui_xyflow::core::edge::Connection| true;
    let state = handle
        .on_pointer_move(XYPosition::new(5000.0, 5000.0), Transform::IDENTITY, &lookup, &always_valid)
        .unwrap();
    assert!(state.to_handle.is_none());
    assert_eq!(state.is_valid, None);
}

#[test]
fn xyhandle_move_before_down_is_none() {
    let lookup = build_lookup(vec![]);
    let mut handle = XYHandle::<String>::new(XYHandleOptions::default());
    let always_valid = |_: &gpui_xyflow::core::edge::Connection| true;
    assert!(handle
        .on_pointer_move(XYPosition::new(0.0, 0.0), Transform::IDENTITY, &lookup, &always_valid)
        .is_none());
}

#[test]
fn xyhandle_cancel_clears_state() {
    let lookup = build_lookup(vec![node_with_handles("a", 0.0, 0.0, 1, 0)]);
    let mut handle = XYHandle::<String>::new(XYHandleOptions::default());
    handle.on_pointer_down(
        XYHandleStartParams {
            node_id: "a",
            handle_id: Some("s0"),
            handle_type: HandleType::Source,
            pointer: XYPosition::new(100.0, 20.0),
            transform: Transform::IDENTITY,
        },
        &lookup,
    );
    handle.cancel();
    assert!(!handle.is_connecting());
    assert!(handle.click_start_handle().is_none());
}

#[test]
fn xyhandle_click_connection_lifecycle() {
    let mut instance = XYHandle::<String>::new(XYHandleOptions::default());
    instance.begin_click_connection(handle("a", HandleType::Source, Some("s0"), 100.0, 20.0));
    assert_eq!(instance.phase(), &XYHandlePhase::ClickPending);
    assert!(instance.click_start_handle().is_some());
    instance.clear_click_connection();
    assert!(instance.click_start_handle().is_none());
    assert_eq!(instance.phase(), &XYHandlePhase::Idle);
}

#[test]
fn xyhandle_options_are_configurable() {
    let mut options = XYHandleOptions::default();
    options.connection_radius = 40.0;
    options.connection_mode = ConnectionMode::Loose;
    let handle = XYHandle::<String>::new(options);
    assert_eq!(handle.options().connection_radius, 40.0);
    assert_eq!(handle.options().connection_mode, ConnectionMode::Loose);
}
