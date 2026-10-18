use std::collections::BTreeMap;

use gpui_xyflow::core::{
    drag::{DragPhase, XYDrag, XYDragOptions, get_event_handler_params, has_selector},
    geometry::{XYPosition, ZIndexMode},
    node::{Node, NodeDragItem, NodeLookup},
    store::{AdoptOptions, adopt_user_nodes},
};

fn node(id: &str, x: f64, y: f64, selected: bool) -> Node<String> {
    Node::<String> {
        id: id.to_string(),
        position: XYPosition::new(x, y),
        width: Some(80.0),
        height: Some(40.0),
        selected,
        ..Node::<String>::default()
    }
}

fn lookup(nodes: Vec<Node<String>>) -> NodeLookup<String> {
    let mut lookup = NodeLookup::new();
    let mut parent_lookup = BTreeMap::new();
    adopt_user_nodes(
        &nodes,
        &mut lookup,
        &mut parent_lookup,
        &AdoptOptions {
            node_origin: [0.0, 0.0],
            node_extent: [
                [f64::NEG_INFINITY, f64::NEG_INFINITY],
                [f64::INFINITY, f64::INFINITY],
            ],
            elevate_nodes_on_select: false,
            z_index_mode: ZIndexMode::Auto,
            check_equality: false,
        },
    );
    lookup
}

#[test]
fn has_selector_matches_target_class() {
    let classes = vec!["nodrag".to_string()];
    assert!(has_selector(&classes, ".nodrag", &[]));
    assert!(!has_selector(&classes, ".other", &[]));
}

#[test]
fn has_selector_matches_ancestor_class() {
    let ancestors = vec![vec!["nodrag".to_string()]];
    assert!(has_selector(&[], ".nodrag", &ancestors));
}

#[test]
fn has_selector_empty_is_false() {
    assert!(!has_selector(&["nodrag".to_string()], "", &[]));
}

#[test]
fn get_event_handler_params_returns_first() {
    let lookup = lookup(vec![node("a", 10.0, 20.0, true)]);
    let mut items = BTreeMap::new();
    let mut item = NodeDragItem::from_internal(&lookup["a"], XYPosition::new(10.0, 20.0));
    item.position = XYPosition::new(15.0, 25.0);
    items.insert("a".to_string(), item);

    let (first, all) = get_event_handler_params(Some("a"), &items, &lookup, true);
    assert_eq!(all.len(), 1);
    assert_eq!(first.unwrap().position, XYPosition::new(15.0, 25.0));
}

#[test]
fn get_event_handler_params_without_node_id() {
    let lookup = lookup(vec![node("a", 0.0, 0.0, true), node("b", 50.0, 0.0, true)]);
    let mut items = BTreeMap::new();
    for id in ["a", "b"] {
        items.insert(
            id.to_string(),
            NodeDragItem::from_internal(&lookup[id], XYPosition::new(0.0, 0.0)),
        );
    }
    let (first, all) = get_event_handler_params(None, &items, &lookup, false);
    assert_eq!(all.len(), 2);
    assert_eq!(first.unwrap().id, "a");
}

#[test]
fn xydrag_requires_press_to_start() {
    let lookup = lookup(vec![node("a", 0.0, 0.0, false)]);
    let mut drag = XYDrag::new(XYDragOptions::default());
    assert!(drag.on_pointer_down(XYPosition::new(10.0, 10.0), Some("a"), &lookup));
    assert_eq!(drag.phase(), DragPhase::Pressed);
    assert!(!drag.has_started());
}

#[test]
fn xydrag_no_items_fails() {
    let lookup = lookup(vec![]);
    let mut drag = XYDrag::new(XYDragOptions::default());
    assert!(!drag.on_pointer_down(XYPosition::new(0.0, 0.0), Some("ghost"), &lookup));
    assert_eq!(drag.phase(), DragPhase::Idle);
}

#[test]
fn xydrag_threshold_blocks_small_moves() {
    let lookup = lookup(vec![node("a", 0.0, 0.0, false)]);
    let mut drag = XYDrag::new(XYDragOptions::default());
    drag.on_pointer_down(XYPosition::new(10.0, 10.0), Some("a"), &lookup);
    let update = drag.on_pointer_move(XYPosition::new(10.0, 10.0), &lookup, |item, _| {
        item.position = XYPosition::new(99.0, 99.0);
    });
    assert!(!update.changed);
    assert_eq!(drag.phase(), DragPhase::Pressed);
}

#[test]
fn xydrag_move_past_threshold_starts_drag() {
    let lookup = lookup(vec![node("a", 0.0, 0.0, false)]);
    let mut drag = XYDrag::new(XYDragOptions::default());
    drag.on_pointer_down(XYPosition::new(10.0, 10.0), Some("a"), &lookup);
    let update = drag.on_pointer_move(XYPosition::new(30.0, 30.0), &lookup, |item, _| {
        item.position = XYPosition::new(20.0, 20.0);
    });
    assert!(update.changed);
    assert!(drag.has_started());
    assert_eq!(drag.phase(), DragPhase::Dragging);
    assert_eq!(update.positions["a"].position, XYPosition::new(20.0, 20.0));
}

#[test]
fn xydrag_move_when_idle_is_noop() {
    let lookup = lookup(vec![node("a", 0.0, 0.0, false)]);
    let mut drag = XYDrag::new(XYDragOptions::default());
    let update = drag.on_pointer_move(XYPosition::new(50.0, 50.0), &lookup, |item, _| {
        item.position = XYPosition::new(1.0, 1.0);
    });
    assert!(!update.changed);
}

#[test]
fn xydrag_pointer_up_reports_dragged() {
    let lookup = lookup(vec![node("a", 0.0, 0.0, false)]);
    let mut drag = XYDrag::new(XYDragOptions::default());
    drag.on_pointer_down(XYPosition::new(10.0, 10.0), Some("a"), &lookup);
    drag.on_pointer_move(XYPosition::new(40.0, 40.0), &lookup, |item, _| {
        item.position = XYPosition::new(30.0, 30.0);
    });
    let (dragged, items) = drag.on_pointer_up();
    assert!(dragged);
    assert_eq!(items.len(), 1);
    assert_eq!(drag.phase(), DragPhase::Idle);
}

#[test]
fn xydrag_cancel_resets() {
    let lookup = lookup(vec![node("a", 0.0, 0.0, false)]);
    let mut drag = XYDrag::new(XYDragOptions::default());
    drag.on_pointer_down(XYPosition::new(0.0, 0.0), Some("a"), &lookup);
    drag.cancel();
    assert_eq!(drag.phase(), DragPhase::Idle);
    assert!(drag.drag_items().is_empty());
}

#[test]
fn xydrag_respects_nodrag_class() {
    let options = XYDragOptions {
        no_drag_class_name: Some("nodrag".to_string()),
        ..XYDragOptions::default()
    };
    let drag = XYDrag::new(options);
    assert!(!drag.can_start(&["nodrag".to_string()], &[]));
    assert!(drag.can_start(&["other".to_string()], &[]));
}

#[test]
fn xydrag_respects_handle_selector() {
    let options = XYDragOptions {
        handle_selector: Some(".drag-handle".to_string()),
        ..XYDragOptions::default()
    };
    let drag = XYDrag::new(options);
    assert!(drag.can_start(&["drag-handle".to_string()], &[]));
    assert!(!drag.can_start(&["other".to_string()], &[]));
}

#[test]
fn xydrag_multi_selection_drags_all() {
    let lookup = lookup(vec![node("a", 0.0, 0.0, true), node("b", 100.0, 0.0, true)]);
    let mut drag = XYDrag::new(XYDragOptions::default());
    assert!(drag.on_pointer_down(XYPosition::new(10.0, 10.0), None, &lookup));
    assert_eq!(drag.drag_items().len(), 2);
    let update = drag.on_pointer_move(XYPosition::new(50.0, 50.0), &lookup, |item, _| {
        item.position.x += 10.0;
    });
    assert_eq!(update.positions.len(), 2);
}
