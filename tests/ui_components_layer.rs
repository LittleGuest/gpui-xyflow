use gpui_xyflow::{
    core::{
        change::NodeChange,
        edge::{Edge, EdgeLookup},
        geometry::{Rect, Transform, Viewport, XYPosition},
        node::{Node, NodeLookup, ParentLookup},
        store::{AdoptOptions, adopt_user_nodes},
    },
    ui::{components::*, store::FlowProps},
};

fn node(id: &str, x: f64, y: f64, selected: bool) -> Node<String> {
    Node::<String> {
        id: id.to_string(),
        position: XYPosition::new(x, y),
        selected,
        width: Some(80.0),
        height: Some(40.0),
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
fn aria_descriptions_default_uses_keyboard_disabled_text() {
    let props = FlowProps::<String>::default();
    let desc = a11y_descriptions(&props, "react-flow", "moved");
    assert_eq!(desc.node_desc_id, "react-flow__node-desc-react-flow");
    assert_eq!(desc.edge_desc_id, "react-flow__edge-desc-react-flow");
    assert!(desc.node_desc.contains("arrow keys"));
    assert!(desc.live_message_id.is_some());
    assert_eq!(desc.live_message.as_deref(), Some("moved"));
}

#[test]
fn aria_descriptions_disable_keyboard_switches_text_and_hides_live() {
    let props = FlowProps::<String> {
        disable_keyboard_a11y: true,
        ..FlowProps::default()
    };
    let desc = a11y_descriptions(&props, "rf", "moved");
    assert!(!desc.node_desc.contains("arrow keys"));
    assert!(desc.live_message_id.is_none());
    assert!(desc.live_message.is_none());
}

#[test]
fn attribution_visible_by_default() {
    let attr = attribution("react", false, "bottom-right");
    assert!(attr.visible);
    assert_eq!(attr.rf_id, "react-flow");
    assert_eq!(attr.label, "React Flow");
    assert_eq!(attr.class_name(), "react-flow__attribution");
}

#[test]
fn attribution_hidden_when_flagged() {
    let attr = attribution("react", true, "bottom-right");
    assert!(!attr.visible);
}

#[test]
fn attribution_capitalizes_library_name() {
    let attr = attribution("svelte", false, "bottom-left");
    assert_eq!(attr.label, "Svelte Flow");
    assert_eq!(attr.panel_position, "bottom-left");
}

#[test]
fn portal_targets_report_availability() {
    assert!(edge_label_portal("rf", true).available);
    assert!(!edge_label_portal("rf", false).available);
    assert!(viewport_portal("rf", true).available);
}

#[test]
fn user_selection_rect_visibility_rules() {
    let hidden = UserSelectionRect::from_state(false, Some(Rect::new(0.0, 0.0, 10.0, 10.0)));
    assert!(!hidden.is_visible());

    let no_rect = UserSelectionRect::from_state(true, None);
    assert!(!no_rect.is_visible());

    let visible = UserSelectionRect::from_state(true, Some(Rect::new(5.0, 6.0, 20.0, 30.0)));
    assert!(visible.is_visible());
    assert_eq!(visible.transform().as_deref(), Some("translate(5px, 6px)"));
}

#[test]
fn nodes_selection_computes_selected_bounds() {
    let lookup = build_lookup(vec![
        node("a", 0.0, 0.0, true),
        node("b", 100.0, 50.0, true),
        node("c", 500.0, 500.0, false),
    ]);
    let selection = nodes_selection(&lookup, Transform::IDENTITY, false);
    assert!(selection.should_render());
    assert_eq!(selection.width, Some(180.0));
    assert_eq!(selection.height, Some(90.0));
    assert_eq!(selection.offset, XYPosition::new(0.0, 0.0));
}

#[test]
fn nodes_selection_hidden_during_user_selection() {
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, true)]);
    let selection = nodes_selection(&lookup, Transform::IDENTITY, true);
    assert!(!selection.should_render());
}

#[test]
fn nodes_selection_transform_string_includes_scale() {
    let lookup = build_lookup(vec![node("a", 10.0, 20.0, true)]);
    let selection = nodes_selection(&lookup, Transform::new(5.0, 6.0, 2.0), false);
    assert_eq!(
        selection.transform_string(),
        "translate(5px,6px) scale(2) translate(10px,20px)"
    );
}

#[test]
fn arrow_key_diff_maps_all_directions() {
    assert_eq!(arrow_key_diff("ArrowUp"), Some(XYPosition::new(0.0, -1.0)));
    assert_eq!(arrow_key_diff("ArrowDown"), Some(XYPosition::new(0.0, 1.0)));
    assert_eq!(
        arrow_key_diff("ArrowLeft"),
        Some(XYPosition::new(-1.0, 0.0))
    );
    assert_eq!(
        arrow_key_diff("ArrowRight"),
        Some(XYPosition::new(1.0, 0.0))
    );
    assert_eq!(arrow_key_diff("Escape"), None);
}

#[test]
fn move_selected_nodes_scales_by_zoom_and_shift() {
    assert_eq!(
        move_selected_nodes_delta("ArrowRight", false, 2.0),
        Some(XYPosition::new(0.5, 0.0))
    );
    assert_eq!(
        move_selected_nodes_delta("ArrowDown", true, 1.0),
        Some(XYPosition::new(0.0, 4.0))
    );
    assert_eq!(
        move_selected_nodes_delta("ArrowUp", false, 0.0),
        Some(XYPosition::new(0.0, -1.0))
    );
}

#[test]
fn selected_ids_partitions_nodes_and_edges() {
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, true), node("b", 0.0, 0.0, false)]);
    let edges = vec![
        Edge::<String> {
            id: "e1".to_string(),
            selected: true,
            ..Edge::default()
        },
        Edge::<String> {
            id: "e2".to_string(),
            selected: false,
            ..Edge::default()
        },
    ];
    let (node_ids, edge_ids) = selected_ids(&lookup, &edges);
    assert_eq!(node_ids, vec!["a"]);
    assert_eq!(edge_ids, vec!["e1"]);
}

#[test]
fn selection_changed_detects_id_diff() {
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, true)]);
    let edges: Vec<Edge<String>> = vec![];
    let before = selection_snapshot(&lookup, &edges);
    assert!(!selection_changed(&before, &lookup, &edges));
}

#[test]
fn selection_snapshot_captures_user_nodes() {
    let lookup = build_lookup(vec![node("a", 10.0, 20.0, true)]);
    let edges: Vec<Edge<String>> = vec![];
    let snapshot = selection_snapshot(&lookup, &edges);
    assert_eq!(snapshot.nodes.len(), 1);
    assert_eq!(snapshot.nodes[0].id, "a");
    assert!(snapshot.edges.is_empty());
}

#[test]
fn should_mount_selection_listener_rules() {
    assert!(!should_mount_selection_listener(false, 0));
    assert!(should_mount_selection_listener(true, 0));
    assert!(should_mount_selection_listener(false, 2));
}

#[test]
fn batch_queue_drains_nodes() {
    let mut queue = BatchQueue::<String>::default();
    queue.push_nodes(vec![node("a", 0.0, 0.0, false)]);
    queue.push_nodes(vec![node("a", 0.0, 0.0, false), node("b", 0.0, 0.0, false)]);
    assert!(queue.has_pending());
    assert_eq!(queue.pending_node_count(), 2);

    let current: Vec<Node<String>> = vec![];
    let (next, changed) = queue.drain_nodes(&current);
    assert!(changed);
    assert_eq!(next.len(), 2);
    assert!(!queue.has_pending());
}

#[test]
fn batch_queue_drains_edges() {
    let mut queue = BatchQueue::<String>::default();
    queue.push_edges(vec![Edge::<String>::new("e1", "a", "b")]);
    let (next, changed) = queue.drain_edges(&[]);
    assert!(changed);
    assert_eq!(next.len(), 1);
}

#[test]
fn batch_queue_empty_drain_returns_current() {
    let mut queue = BatchQueue::<String>::default();
    let current = vec![node("a", 0.0, 0.0, false)];
    let (next, changed) = queue.drain_nodes(&current);
    assert!(!changed);
    assert_eq!(next.len(), 1);
}

#[test]
fn process_node_queue_produces_add_changes() {
    let mut queue = BatchQueue::<String>::default();
    queue.push_nodes(vec![node("new", 0.0, 0.0, false)]);
    let lookup = build_lookup(vec![]);
    let (next, changes) = process_node_queue(&mut queue, &[], &lookup);
    assert_eq!(next.len(), 1);
    assert_eq!(changes.len(), 1);
    assert!(matches!(changes[0], NodeChange::Add(_)));
}

#[test]
fn process_edge_queue_produces_remove_changes() {
    let mut lookup = EdgeLookup::new();
    lookup.insert("old".to_string(), Edge::<String>::new("old", "a", "b"));

    let mut queue = BatchQueue::<String>::default();
    queue.push_edges(vec![]);
    let (_, changes) = process_edge_queue(&mut queue, &[], &lookup);
    assert_eq!(changes.len(), 1);
    assert!(matches!(
        changes[0],
        gpui_xyflow::core::change::EdgeChange::Remove(_)
    ));
}

#[test]
fn transform_helpers_are_stable() {
    let viewport = Viewport::new(1.0, 2.0, 3.0);
    assert_eq!(viewport.transform(), Transform::new(1.0, 2.0, 3.0));
    let _ = Rect::new(0.0, 0.0, 1.0, 1.0);
}

#[test]
fn aria_live_message_key_is_stable() {
    assert_eq!(ARIA_LIVE_MESSAGE, "react-flow__aria-live");
    assert_eq!(
        A11yDescriptions::live_message_id("rf"),
        "react-flow__aria-live-rf"
    );
}
