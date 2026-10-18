use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use gpui_xyflow::core::change::{NodeChange, NodePositionChange};
use gpui_xyflow::core::edge::Edge;
use gpui_xyflow::core::geometry::{CoordinateExtent, XYPosition};
use gpui_xyflow::core::node::Node;
use gpui_xyflow::ui::store::{FlowProps, FlowStore};

fn node(id: &str, x: f64, y: f64) -> Node<String> {
    let mut node = Node::<String>::default();
    node.id = id.to_string();
    node.position = XYPosition::new(x, y);
    node.width = Some(80.0);
    node.height = Some(40.0);
    node
}

fn store_with(nodes: Vec<Node<String>>) -> FlowStore<String> {
    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_size(800.0, 600.0);
    store.set_default_nodes_and_edges(nodes, Vec::new());
    store.update_nodes();
    store
}

#[test]
fn trigger_node_changes_invokes_callback() {
    let seen = Arc::new(AtomicUsize::new(0));
    let counter = seen.clone();
    let mut props = FlowProps::<String>::default();
    props.callbacks.on_nodes_change = Some(Arc::new(move |changes: &[NodeChange<String>]| {
        counter.fetch_add(changes.len(), Ordering::SeqCst);
    }));
    props.has_default_nodes = true;
    let mut store = FlowStore::<String>::new(props);
    store.set_nodes(vec![node("a", 0.0, 0.0)]);
    store.trigger_node_changes(vec![NodeChange::Select(
        gpui_xyflow::core::change::SelectionChange {
            id: "a".to_string(),
            selected: true,
        },
    )]);
    assert_eq!(seen.load(Ordering::SeqCst), 1);
    assert!(store.nodes()[0].selected);
}

#[test]
fn trigger_node_changes_without_default_nodes_leaves_state() {
    let seen = Arc::new(AtomicUsize::new(0));
    let counter = seen.clone();
    let mut props = FlowProps::<String>::default();
    props.has_default_nodes = false;
    props.callbacks.on_nodes_change = Some(Arc::new(move |changes: &[NodeChange<String>]| {
        counter.fetch_add(changes.len(), Ordering::SeqCst);
    }));
    let mut store = FlowStore::<String>::new(props);
    store.set_nodes(vec![node("a", 0.0, 0.0)]);
    store.trigger_node_changes(vec![NodeChange::Select(
        gpui_xyflow::core::change::SelectionChange {
            id: "a".to_string(),
            selected: true,
        },
    )]);
    assert_eq!(seen.load(Ordering::SeqCst), 1);
    assert!(!store.nodes()[0].selected);
}

#[test]
fn trigger_node_changes_empty_is_noop() {
    let seen = Arc::new(AtomicUsize::new(0));
    let counter = seen.clone();
    let mut props = FlowProps::<String>::default();
    props.callbacks.on_nodes_change = Some(Arc::new(move |_: &[NodeChange<String>]| {
        counter.fetch_add(1, Ordering::SeqCst);
    }));
    let mut store = FlowStore::<String>::new(props);
    store.trigger_node_changes(Vec::new());
    assert_eq!(seen.load(Ordering::SeqCst), 0);
}

#[test]
fn trigger_edge_changes_invokes_callback() {
    let seen = Arc::new(AtomicUsize::new(0));
    let counter = seen.clone();
    let mut props = FlowProps::<String>::default();
    props.has_default_edges = true;
    props.callbacks.on_edges_change = Some(Arc::new(
        move |changes: &[gpui_xyflow::core::change::EdgeChange<String>]| {
            counter.fetch_add(changes.len(), Ordering::SeqCst);
        },
    ));
    let mut store = FlowStore::<String>::new(props);
    let mut edge = Edge::<String>::default();
    edge.id = "e1".to_string();
    edge.source = "a".to_string();
    edge.target = "b".to_string();
    store.set_edges(vec![edge]);
    store.trigger_edge_changes(vec![gpui_xyflow::core::change::EdgeChange::Select(
        gpui_xyflow::core::change::SelectionChange {
            id: "e1".to_string(),
            selected: true,
        },
    )]);
    assert_eq!(seen.load(Ordering::SeqCst), 1);
    assert!(store.edges()[0].selected);
}

#[test]
fn node_change_middleware_can_drop_changes() {
    let mut store = store_with(vec![node("a", 0.0, 0.0)]);
    store.add_nodes_change_middleware(|changes: Vec<NodeChange<String>>, _| {
        changes
            .into_iter()
            .filter(|change| !matches!(change, NodeChange::Select(_)))
            .collect()
    });
    let filtered = store.apply_nodes_change_middleware(vec![NodeChange::Select(
        gpui_xyflow::core::change::SelectionChange {
            id: "a".to_string(),
            selected: true,
        },
    )]);
    assert!(filtered.is_empty());
}

#[test]
fn node_change_middleware_chain_is_ordered() {
    let mut store = store_with(vec![node("a", 0.0, 0.0)]);
    store.add_nodes_change_middleware(|mut changes: Vec<NodeChange<String>>, _| {
        changes.push(NodeChange::Remove(
            gpui_xyflow::core::change::RemoveChange {
                id: "m1".to_string(),
            },
        ));
        changes
    });
    store.add_nodes_change_middleware(|mut changes: Vec<NodeChange<String>>, _| {
        changes.push(NodeChange::Remove(
            gpui_xyflow::core::change::RemoveChange {
                id: "m2".to_string(),
            },
        ));
        changes
    });
    let out = store.apply_nodes_change_middleware(Vec::new());
    assert_eq!(out.len(), 2);
    match (&out[0], &out[1]) {
        (NodeChange::Remove(a), NodeChange::Remove(b)) => {
            assert_eq!(a.id, "m1");
            assert_eq!(b.id, "m2");
        }
        _ => panic!("middleware order broken"),
    }
}

#[test]
fn edge_change_middleware_runs_against_edges() {
    let mut store = store_with(vec![node("a", 0.0, 0.0)]);
    let mut edge = Edge::<String>::default();
    edge.id = "e1".to_string();
    edge.source = "a".to_string();
    edge.target = "a".to_string();
    store.set_edges(vec![edge]);
    store.add_edges_change_middleware(|changes: Vec<gpui_xyflow::core::change::EdgeChange<String>>, edges| {
        assert_eq!(edges.len(), 1);
        changes
    });
    let out = store.apply_edges_change_middleware(vec![]);
    assert!(out.is_empty());
}

#[test]
fn update_node_positions_emits_position_changes() {
    let mut store = store_with(vec![node("a", 10.0, 10.0)]);
    let items = {
        let stored = store.node_lookup().get("a").unwrap();
        vec![gpui_xyflow::core::node::NodeDragItem::from_internal(
            stored,
            XYPosition::new(10.0, 10.0),
        )]
    };
    let mut moved = items[0].clone();
    moved.position = XYPosition::new(30.0, 40.0);
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = received.clone();
    store.props_mut().callbacks.on_nodes_change = Some(Arc::new(
        move |changes: &[NodeChange<String>]| {
            for change in changes {
                if let NodeChange::Position(position) = change {
                    sink.lock().unwrap().push(position.clone());
                }
            }
        },
    ));
    store.props_mut().has_default_nodes = true;
    store.update_node_positions(&[moved], true);
    let captured = received.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].id, "a");
    assert_eq!(captured[0].position.unwrap(), XYPosition::new(30.0, 40.0));
    assert_eq!(captured[0].dragging, Some(true));
}

#[test]
fn update_node_positions_clamps_negative_for_expand_parent() {
    let mut store = store_with(vec![node("a", 10.0, 10.0)]);
    let mut item = {
        let stored = store.node_lookup().get("a").unwrap();
        gpui_xyflow::core::node::NodeDragItem::from_internal(stored, XYPosition::new(10.0, 10.0))
    };
    item.position = XYPosition::new(-50.0, -20.0);
    item.expand_parent = true;
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = received.clone();
    store.props_mut().callbacks.on_nodes_change = Some(Arc::new(
        move |changes: &[NodeChange<String>]| {
            for change in changes {
                if let NodeChange::Position(position) = change {
                    sink.lock().unwrap().push(position.clone());
                }
            }
        },
    ));
    store.update_node_positions(&[item], false);
    let captured = received.lock().unwrap();
    assert_eq!(captured[0].position.unwrap(), XYPosition::new(0.0, 0.0));
}

#[test]
fn set_min_zoom_updates_extent() {
    let mut store = store_with(vec![node("a", 0.0, 0.0)]);
    store.set_min_zoom(0.25);
    assert_eq!(store.props().min_zoom, 0.25);
    assert_eq!(store.pan_zoom().scale_extent(), [0.25, 2.0]);
}

#[test]
fn set_min_zoom_ignores_non_finite() {
    let mut store = store_with(vec![node("a", 0.0, 0.0)]);
    store.set_min_zoom(f64::NAN);
    assert_eq!(store.props().min_zoom, 0.5);
}

#[test]
fn set_max_zoom_updates_extent() {
    let mut store = store_with(vec![node("a", 0.0, 0.0)]);
    store.set_max_zoom(4.0);
    assert_eq!(store.props().max_zoom, 4.0);
    assert_eq!(store.pan_zoom().scale_extent(), [0.5, 4.0]);
}

#[test]
fn set_translate_extent_updates_pan_zoom() {
    let mut store = store_with(vec![node("a", 0.0, 0.0)]);
    let extent = [[0.0, 0.0], [100.0, 100.0]];
    store.set_translate_extent(extent);
    assert_eq!(store.props().translate_extent, extent);
    assert_eq!(store.props().translate_extent, extent);
}

#[test]
fn set_node_extent_updates_absolute_positions() {
    let mut store = store_with(vec![node("a", 100.0, 100.0)]);
    let before = store.node_lookup()["a"].internals.position_absolute;
    store.set_node_extent([[0.0, 0.0], [50.0, 50.0]]);
    let after = store.node_lookup()["a"].internals.position_absolute;
    assert_ne!(before, after);
    assert!(after.x <= 50.0);
}

#[test]
fn set_node_extent_same_value_is_noop() {
    let mut store = store_with(vec![node("a", 100.0, 100.0)]);
    let before = store.node_lookup()["a"].internals.position_absolute;
    let current = store.props().node_extent;
    store.set_node_extent(current);
    assert_eq!(store.node_lookup()["a"].internals.position_absolute, before);
}

#[test]
fn set_default_nodes_and_edges_marks_defaults() {
    let mut store = FlowStore::<String>::new(FlowProps::default());
    store.set_size(800.0, 600.0);
    store.set_default_nodes_and_edges(vec![node("a", 0.0, 0.0)], Vec::new());
    assert!(store.has_default_nodes());
    assert!(store.has_default_edges());
    assert_eq!(store.nodes().len(), 1);
}

#[test]
fn resolve_fit_view_centers_content() {
    let mut store = store_with(vec![node("a", 0.0, 0.0), node("b", 400.0, 300.0)]);
    store.request_fit_view();
    assert!(store.queued_fit_view());
    assert!(store.resolve_fit_view());
    assert!(!store.queued_fit_view());
    let viewport = store.viewport();
    assert!(viewport.zoom > 0.0);
}

#[test]
fn maybe_resolve_fit_view_requires_initialized_nodes() {
    let mut props = FlowProps::<String>::default();
    props.fit_view_queued = true;
    let mut store = FlowStore::<String>::new(props);
    store.set_size(800.0, 600.0);
    store.maybe_resolve_fit_view();
    assert!(store.queued_fit_view());
    store.set_nodes(vec![node("a", 0.0, 0.0)]);
    store.update_nodes();
    store.maybe_resolve_fit_view();
    assert!(!store.queued_fit_view());
}

#[test]
fn callbacks_default_is_empty() {
    let props = FlowProps::<String>::default();
    assert!(props.callbacks.is_empty());
}

#[test]
fn flow_props_new_fields_have_official_defaults() {
    let props = FlowProps::<String>::default();
    assert_eq!(props.rf_id, "1");
    assert_eq!(props.no_pan_class_name, "nopan");
    assert_eq!(props.node_drag_threshold, 1.0);
    assert_eq!(props.connection_drag_threshold, 1.0);
    assert_eq!(props.auto_pan_speed, 15.0);
    assert!(props.auto_pan_on_connect);
    assert!(props.auto_pan_on_drag);
    assert!(!props.auto_pan_on_node_focus);
    assert!(props.select_nodes_on_drag);
    assert!(!props.multi_selection_active);
    assert!(!props.pane_dragging);
    assert!(!props.fit_view_queued);
    assert!(!props.debug);
    assert_eq!(props.selection_key_code.as_deref(), Some("Shift"));
}

#[test]
fn position_change_carries_absolute_position() {
    let change = NodePositionChange {
        id: "a".to_string(),
        position: Some(XYPosition::new(1.0, 2.0)),
        position_absolute: Some(XYPosition::new(3.0, 4.0)),
        dragging: Some(true),
    };
    assert_eq!(change.position_absolute.unwrap(), XYPosition::new(3.0, 4.0));
}
