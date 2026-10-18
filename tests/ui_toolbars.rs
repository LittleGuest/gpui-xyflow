use gpui_xyflow::{
    core::{
        edge::Edge,
        geometry::{Alignment, Position, Viewport, XYPosition},
        node::{Node, NodeLookup, ParentLookup},
        store::{AdoptOptions, adopt_user_nodes},
        utils_extra::{ToolbarAlignX, ToolbarAlignY},
    },
    ui::toolbars::{EdgeToolbarProps, NodeToolbarProps, edge_toolbar, node_toolbar},
};

fn node(id: &str, x: f64, y: f64, selected: bool) -> Node<String> {
    Node::<String> {
        id: id.to_string(),
        position: XYPosition::new(x, y),
        selected,
        width: Some(80.0),
        height: Some(40.0),
        measured: gpui_xyflow::core::node::MeasuredDimensions::new(80.0, 40.0),
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
fn node_toolbar_visible_for_single_selected_node() {
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, true)]);
    let props = NodeToolbarProps {
        node_ids: vec!["a".to_string()],
        ..NodeToolbarProps::default()
    };
    let state = node_toolbar(&props, &lookup, Viewport::IDENTITY, 1);
    assert!(state.active);
    assert!(state.transform.is_some());
    assert_eq!(state.data_id, "a");
}

#[test]
fn node_toolbar_hidden_with_multiple_selected() {
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, true), node("b", 100.0, 0.0, true)]);
    let props = NodeToolbarProps {
        node_ids: vec!["a".to_string()],
        ..NodeToolbarProps::default()
    };
    let state = node_toolbar(&props, &lookup, Viewport::IDENTITY, 2);
    assert!(!state.active);
}

#[test]
fn node_toolbar_forced_visible_ignores_selection() {
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, false)]);
    let props = NodeToolbarProps {
        node_ids: vec!["a".to_string()],
        is_visible: Some(true),
        ..NodeToolbarProps::default()
    };
    let state = node_toolbar(&props, &lookup, Viewport::IDENTITY, 0);
    assert!(state.active);
}

#[test]
fn node_toolbar_hidden_for_unknown_node() {
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, true)]);
    let props = NodeToolbarProps {
        node_ids: vec!["zzz".to_string()],
        ..NodeToolbarProps::default()
    };
    let state = node_toolbar(&props, &lookup, Viewport::IDENTITY, 1);
    assert!(!state.active);
}

#[test]
fn node_toolbar_z_index_exceeds_node_z() {
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, true)]);
    let props = NodeToolbarProps {
        node_ids: vec!["a".to_string()],
        ..NodeToolbarProps::default()
    };
    let state = node_toolbar(&props, &lookup, Viewport::IDENTITY, 1);
    let node_z = lookup.get("a").unwrap().internals.z;
    assert!(state.z_index > node_z);
}

#[test]
fn node_toolbar_transform_reflects_position() {
    let lookup = build_lookup(vec![node("a", 0.0, 0.0, true)]);
    let props = NodeToolbarProps {
        node_ids: vec!["a".to_string()],
        position: Position::Bottom,
        align: Alignment::Start,
        ..NodeToolbarProps::default()
    };
    let state = node_toolbar(&props, &lookup, Viewport::IDENTITY, 1);
    assert!(state.transform.unwrap().contains("translate"));
}

#[test]
fn edge_toolbar_visible_when_edge_selected() {
    let edge = Edge::<String> {
        id: "e1".to_string(),
        selected: true,
        ..Edge::default()
    };
    let props = EdgeToolbarProps {
        edge_id: "e1".to_string(),
        x: 10.0,
        y: 20.0,
        ..EdgeToolbarProps::default()
    };
    let state = edge_toolbar(&props, Some(&edge), 1.0);
    assert!(state.active);
    assert!(state.transform.is_some());
}

#[test]
fn edge_toolbar_hidden_when_edge_not_selected() {
    let edge = Edge::<String> {
        id: "e1".to_string(),
        selected: false,
        ..Edge::default()
    };
    let props = EdgeToolbarProps {
        edge_id: "e1".to_string(),
        x: 10.0,
        y: 20.0,
        ..EdgeToolbarProps::default()
    };
    let state = edge_toolbar(&props, Some(&edge), 1.0);
    assert!(!state.active);
}

#[test]
fn edge_toolbar_hidden_without_edge() {
    let props = EdgeToolbarProps {
        edge_id: "e1".to_string(),
        x: 10.0,
        y: 20.0,
        ..EdgeToolbarProps::default()
    };
    let state = edge_toolbar::<String>(&props, None, 1.0);
    assert!(!state.active);
}

#[test]
fn edge_toolbar_z_index_uses_edge_z() {
    let edge = Edge::<String> {
        id: "e1".to_string(),
        selected: true,
        z_index: Some(5.0),
        ..Edge::default()
    };
    let props = EdgeToolbarProps {
        edge_id: "e1".to_string(),
        x: 10.0,
        y: 20.0,
        ..EdgeToolbarProps::default()
    };
    let state = edge_toolbar(&props, Some(&edge), 1.0);
    assert_eq!(state.z_index, 6.0);
}

#[test]
fn edge_toolbar_forced_visible_ignores_selection() {
    let edge = Edge::<String> {
        id: "e1".to_string(),
        selected: false,
        ..Edge::default()
    };
    let props = EdgeToolbarProps {
        edge_id: "e1".to_string(),
        x: 10.0,
        y: 20.0,
        is_visible: Some(true),
        align_x: ToolbarAlignX::Left,
        align_y: ToolbarAlignY::Top,
    };
    let state = edge_toolbar(&props, Some(&edge), 2.0);
    assert!(state.active);
    assert!(state.transform.unwrap().contains("scale(0.5)"));
}
