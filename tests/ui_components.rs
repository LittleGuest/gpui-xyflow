use gpui_xyflow::core::edge::{BezierPathOptions, EdgeMarker, EdgeMarkerType, PathOptions};
use gpui_xyflow::core::geometry::Position;
use gpui_xyflow::core::node::{HandleType, Node};
use gpui_xyflow::ui::edges::{
    base_edge, bezier_edge_path, default_edge_interaction_width, edge_component_output,
    edge_center_for, edge_markers, marker_ref, simple_bezier_edge_path, smooth_step_edge_path,
    step_edge_path, straight_edge_path, BaseEdgeProps, EdgeComponentKind, EdgeLabelStyle,
    EdgePathParams,
};
use gpui_xyflow::ui::nodes::{
    built_in_node_output, default_node_handles, group_node_handles, input_node_handles,
    node_handles_for, node_label_default, output_node_handles, BuiltInNodeKind, NodeProps,
};

fn params() -> EdgePathParams {
    EdgePathParams {
        source_x: 0.0,
        source_y: 0.0,
        source_position: Some(Position::Bottom),
        target_x: 100.0,
        target_y: 100.0,
        target_position: Some(Position::Top),
        path_options: None,
    }
}

#[test]
fn bezier_path_is_non_empty() {
    let output = bezier_edge_path(&params());
    assert!(output.path.starts_with('M'));
    assert_eq!(output.kind, EdgeComponentKind::Bezier);
    assert!(output.label_x.is_some());
}

#[test]
fn straight_path_is_line() {
    let output = straight_edge_path(&params());
    assert!(output.path.contains('L'));
    assert_eq!(output.kind, EdgeComponentKind::Straight);
}

#[test]
fn simple_bezier_path_uses_curve() {
    let output = simple_bezier_edge_path(&params());
    assert!(output.path.contains('C'));
    assert_eq!(output.kind, EdgeComponentKind::SimpleBezier);
}

#[test]
fn step_path_has_segments() {
    let output = step_edge_path(&params());
    assert!(output.path.contains('L'));
    assert_eq!(output.kind, EdgeComponentKind::Step);
}

#[test]
fn smooth_step_path_has_curves() {
    let output = smooth_step_edge_path(&params());
    assert!(output.path.contains('Q') || output.path.contains('C'));
    assert_eq!(output.kind, EdgeComponentKind::SmoothStep);
}

#[test]
fn edge_kind_roundtrip() {
    for kind in [
        EdgeComponentKind::Bezier,
        EdgeComponentKind::SimpleBezier,
        EdgeComponentKind::Straight,
        EdgeComponentKind::Step,
        EdgeComponentKind::SmoothStep,
    ] {
        assert_eq!(EdgeComponentKind::from_str(kind.as_str()), Some(kind));
    }
}

#[test]
fn edge_kind_from_bezier_alias() {
    assert_eq!(
        EdgeComponentKind::from_str("bezier"),
        Some(EdgeComponentKind::Bezier)
    );
    assert_eq!(EdgeComponentKind::from_str("unknown"), None);
}

#[test]
fn edge_component_output_dispatches() {
    let output = edge_component_output(EdgeComponentKind::Straight, &params());
    assert_eq!(output.kind, EdgeComponentKind::Straight);
    let output = edge_component_output(EdgeComponentKind::SmoothStep, &params());
    assert_eq!(output.kind, EdgeComponentKind::SmoothStep);
}

#[test]
fn bezier_path_options_are_applied() {
    let mut base = params();
    base.source_x = 200.0;
    base.source_y = 0.0;
    base.source_position = Some(Position::Right);
    base.target_x = 0.0;
    base.target_y = 60.0;
    base.target_position = Some(Position::Left);
    let mut custom = base.clone();
    custom.path_options = Some(PathOptions::Bezier(BezierPathOptions { curvature: 0.8 }));
    let a = bezier_edge_path(&base);
    let b = bezier_edge_path(&custom);
    assert_ne!(a.path, b.path);
}

#[test]
fn base_edge_default_interaction_width() {
    let props = BaseEdgeProps::default();
    assert_eq!(props.interaction_width(), 20.0);
    assert!(props.has_interaction_path());
    assert_eq!(default_edge_interaction_width(), 20.0);
}

#[test]
fn base_edge_zero_interaction_width_disables_path() {
    let props = BaseEdgeProps {
        path: "M 0,0L 1,1".to_string(),
        interaction_width: Some(0.0),
        ..BaseEdgeProps::default()
    };
    let render = base_edge(props);
    assert!(render.interaction_path.is_none());
}

#[test]
fn base_edge_emits_interaction_path() {
    let props = BaseEdgeProps {
        path: "M 0,0L 10,10".to_string(),
        ..BaseEdgeProps::default()
    };
    let render = base_edge(props);
    assert_eq!(render.interaction_path.as_deref(), Some("M 0,0L 10,10"));
}

#[test]
fn base_edge_requires_label_coordinates() {
    let props = BaseEdgeProps {
        path: "M 0,0L 1,1".to_string(),
        label: Some("hi".to_string()),
        ..BaseEdgeProps::default()
    };
    let render = base_edge(props);
    assert!(render.label.is_none());

    let props = BaseEdgeProps {
        path: "M 0,0L 1,1".to_string(),
        label: Some("hi".to_string()),
        label_x: Some(5.0),
        label_y: Some(6.0),
        label_show_bg: Some(true),
        label_style: Some(EdgeLabelStyle {
            font_size: Some(14.0),
            ..EdgeLabelStyle::default()
        }),
        ..BaseEdgeProps::default()
    };
    let render = base_edge(props);
    let label = render.label.unwrap();
    assert_eq!(label.label, "hi");
    assert!(label.show_bg);
    assert_eq!(label.x, 5.0);
    assert_eq!(label.style.font_size, Some(14.0));
}

#[test]
fn base_edge_class_name_includes_modifier() {
    let props = BaseEdgeProps {
        class_name: Some("custom".to_string()),
        ..BaseEdgeProps::default()
    };
    assert_eq!(props.path_class_name(), "gpui-flow__edge-path custom");
    assert_eq!(
        BaseEdgeProps::default().path_class_name(),
        "gpui-flow__edge-path"
    );
}

#[test]
fn edge_markers_extract_ids() {
    let name = EdgeMarkerType::Name("arrow".to_string());
    let markers = edge_markers(Some(&name), None);
    assert!(markers.start.is_some());
    assert!(markers.end.is_none());
    assert_eq!(marker_ref(Some(&name)), markers.start);
}

#[test]
fn edge_markers_none_stays_none() {
    let markers = edge_markers(None, None);
    assert!(markers.start.is_none());
    assert!(markers.end.is_none());
    assert!(marker_ref(None).is_none());
}

#[test]
fn edge_center_for_returns_finite() {
    let position = gpui_xyflow::core::geometry::EdgePosition {
        source_x: 0.0,
        source_y: 0.0,
        source_position: Position::Bottom,
        target_x: 100.0,
        target_y: 100.0,
        target_position: Position::Top,
    };
    for kind in [
        EdgeComponentKind::Bezier,
        EdgeComponentKind::Straight,
        EdgeComponentKind::Step,
    ] {
        let center = edge_center_for(kind, &position);
        assert!(center.x.is_finite());
        assert!(center.y.is_finite());
    }
}

#[test]
fn node_kind_roundtrip() {
    for kind in [
        BuiltInNodeKind::Default,
        BuiltInNodeKind::Input,
        BuiltInNodeKind::Output,
        BuiltInNodeKind::Group,
    ] {
        assert_eq!(BuiltInNodeKind::from_str(kind.as_str()), kind);
    }
    assert_eq!(BuiltInNodeKind::from_str("nonsense"), BuiltInNodeKind::Default);
}

#[test]
fn node_kind_handle_capabilities() {
    assert!(BuiltInNodeKind::Default.has_source_handle());
    assert!(BuiltInNodeKind::Default.has_target_handle());
    assert!(BuiltInNodeKind::Input.has_source_handle());
    assert!(!BuiltInNodeKind::Input.has_target_handle());
    assert!(!BuiltInNodeKind::Output.has_source_handle());
    assert!(BuiltInNodeKind::Output.has_target_handle());
    assert!(!BuiltInNodeKind::Group.has_source_handle());
    assert!(!BuiltInNodeKind::Group.has_target_handle());
}

#[test]
fn default_node_has_two_handles() {
    let props = NodeProps::<String>::default();
    let output = default_node_handles(&props);
    assert_eq!(output.handles.len(), 2);
    assert_eq!(output.handles[0].handle_type, HandleType::Target);
    assert_eq!(output.handles[1].handle_type, HandleType::Source);
}

#[test]
fn input_node_has_only_source() {
    let props = NodeProps::<String>::default();
    let output = input_node_handles(&props);
    assert_eq!(output.handles.len(), 1);
    assert_eq!(output.handles[0].handle_type, HandleType::Source);
}

#[test]
fn output_node_has_only_target() {
    let props = NodeProps::<String>::default();
    let output = output_node_handles(&props);
    assert_eq!(output.handles.len(), 1);
    assert_eq!(output.handles[0].handle_type, HandleType::Target);
}

#[test]
fn group_node_has_no_handles() {
    let props = NodeProps::<String>::default();
    assert!(group_node_handles(&props).handles.is_empty());
}

#[test]
fn built_in_node_output_dispatches() {
    let props = NodeProps::<String>::default();
    assert_eq!(
        built_in_node_output(BuiltInNodeKind::Group, &props).handles.len(),
        0
    );
    assert_eq!(
        built_in_node_output(BuiltInNodeKind::Default, &props).handles.len(),
        2
    );
}

#[test]
fn node_props_from_node_reads_fields() {
    let mut node = Node::<String>::default();
    node.id = "n1".to_string();
    node.node_type = Some("input".to_string());
    node.source_position = Some(Position::Right);
    node.connectable = Some(false);
    let props = NodeProps::from_node(&node, Some("Hello".to_string()));
    assert_eq!(props.id, "n1");
    assert_eq!(props.label.as_deref(), Some("Hello"));
    assert_eq!(props.source_position, Position::Right);
    assert!(!props.is_connectable);
}

#[test]
fn node_handles_for_uses_node_type() {
    let mut node = Node::<String>::default();
    node.id = "n1".to_string();
    node.node_type = Some("output".to_string());
    let handles = node_handles_for(&node, None);
    assert_eq!(handles.len(), 1);
    assert_eq!(handles[0].handle_type, HandleType::Target);
    assert_eq!(handles[0].node_id, "n1");
}

#[test]
fn node_label_default_skips_default_type() {
    let mut node = Node::<String>::default();
    node.node_type = Some("default".to_string());
    assert!(node_label_default(&node).is_none());
    node.node_type = Some("input".to_string());
    assert_eq!(node_label_default(&node).as_deref(), Some("input"));
}

#[test]
fn group_node_produces_no_handles_via_helper() {
    let mut node = Node::<String>::default();
    node.id = "g".to_string();
    node.node_type = Some("group".to_string());
    assert!(node_handles_for(&node, None).is_empty());
}
