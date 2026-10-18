use gpui_xyflow::{
    Alignment, Box2, Connection, CoordinateExtent, Edge, EdgeMarker, EdgeMarkerType, Node,
    NodeExtent, Position, Rect, Transform, Viewport, XYPosition,
    core::{edge::MarkerType, utils_extra::*},
};

fn node(id: &str, x: f64, y: f64, w: f64, h: f64) -> Node<String> {
    let mut node = Node::new(id, XYPosition::new(x, y), id.to_string());
    node.measured.width = Some(w);
    node.measured.height = Some(h);
    node
}

#[test]
fn error_messages_match_official_wording() {
    assert_eq!(
        error_006(),
        "Can't create edge. An edge needs a source and a target."
    );
    assert_eq!(error_007("abc"), "The old edge with id=abc does not exist.");
}

#[test]
fn is_rect_object_requires_all_numeric_fields() {
    assert!(is_rect_object(&Rect::new(0.0, 0.0, 10.0, 10.0)));
    assert!(!is_rect_object(&Rect::new(0.0, 0.0, f64::NAN, 10.0)));
    assert!(!is_rect_object(&Rect::new(0.0, 0.0, 10.0, f64::INFINITY)));
}

#[test]
fn is_coordinate_extent_rejects_parent() {
    assert!(!is_coordinate_extent(None));
    assert!(!is_coordinate_extent(Some(&NodeExtent::Parent)));
    assert!(is_coordinate_extent(Some(&NodeExtent::Coordinates([
        [0.0, 0.0],
        [10.0, 10.0]
    ]))));
}

#[test]
fn node_has_dimensions_needs_both_axes() {
    let mut only_width = Node::new("a", XYPosition::ZERO, String::new());
    only_width.width = Some(10.0);
    assert!(!node_has_dimensions(&only_width));

    assert!(node_has_dimensions(&node("b", 0.0, 0.0, 10.0, 5.0)));
}

#[test]
fn get_marker_id_returns_empty_for_none() {
    assert_eq!(get_marker_id(None, None), "");
}

#[test]
fn get_marker_id_uses_name_verbatim() {
    let marker = EdgeMarkerType::Name(String::from("my-marker"));
    assert_eq!(get_marker_id(Some(&marker), Some("prefix")), "my-marker");
}

#[test]
fn get_marker_id_builds_sorted_key_value_pairs() {
    let marker = EdgeMarkerType::Marker(
        EdgeMarker::new(MarkerType::ArrowClosed)
            .with_color("red")
            .with_size(12.0, 12.0),
    );
    let id = get_marker_id(Some(&marker), Some("rf"));
    assert!(id.starts_with("rf__"));
    let parts: Vec<&str> = id.trim_start_matches("rf__").split('&').collect();
    let mut sorted = parts.clone();
    sorted.sort();
    assert_eq!(parts, sorted);
    assert!(parts.contains(&"color=red"));
    assert!(parts.contains(&"width=12"));
}

#[test]
fn create_marker_ids_deduplicates_identical_markers() {
    let marker = EdgeMarkerType::Marker(EdgeMarker::new(MarkerType::ArrowClosed));
    let mut first = Edge::<String>::new("e1", "a", "b");
    first.marker_end = Some(marker.clone());
    let mut second = Edge::<String>::new("e2", "b", "c");
    second.marker_end = Some(marker);

    let markers = create_marker_ids(&[first, second], &CreateMarkerIdsOptions::default());
    assert_eq!(markers.len(), 1);
}

#[test]
fn create_marker_ids_ignores_string_markers() {
    let mut edge = Edge::<String>::new("e1", "a", "b");
    edge.marker_end = Some(EdgeMarkerType::Name(String::from("named")));
    let markers = create_marker_ids(&[edge], &CreateMarkerIdsOptions::default());
    assert!(markers.is_empty());
}

#[test]
fn create_marker_ids_applies_defaults_and_sorts() {
    let mut edge = Edge::<String>::new("e1", "a", "b");
    edge.marker_start = Some(EdgeMarkerType::Marker(EdgeMarker::new(MarkerType::Arrow)));
    let mut other = Edge::<String>::new("e2", "b", "c");
    other.marker_end = Some(EdgeMarkerType::Marker(EdgeMarker::new(
        MarkerType::ArrowClosed,
    )));

    let options = CreateMarkerIdsOptions {
        id: None,
        default_color: Some(String::from("#fff")),
        default_marker_start: None,
        default_marker_end: None,
    };
    let markers = create_marker_ids(&[edge, other], &options);
    assert_eq!(markers.len(), 2);
    assert!(markers[0].id < markers[1].id);
    assert!(markers.iter().all(|m| m.color.as_deref() == Some("#fff")));
}

#[test]
fn create_marker_ids_falls_back_to_default_markers() {
    let edge = Edge::<String>::new("e1", "a", "b");
    let options = CreateMarkerIdsOptions {
        id: None,
        default_color: None,
        default_marker_start: Some(EdgeMarkerType::Marker(EdgeMarker::new(
            MarkerType::ArrowClosed,
        ))),
        default_marker_end: None,
    };
    let markers = create_marker_ids(&[edge], &options);
    assert_eq!(markers.len(), 1);
}

#[test]
fn node_toolbar_transform_top_uses_center_alignment() {
    let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
    let viewport = Viewport::new(0.0, 0.0, 2.0);
    let transform =
        get_node_toolbar_transform(rect, viewport, Position::Top, 8.0, Alignment::Center);
    assert_eq!(transform, "translate(120px, 32px) translate(-50%, -100%)");
}

#[test]
fn node_toolbar_transform_right_shifts_horizontally() {
    let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    let viewport = Viewport::new(0.0, 0.0, 1.0);
    let transform =
        get_node_toolbar_transform(rect, viewport, Position::Right, 10.0, Alignment::Start);
    assert_eq!(transform, "translate(110px, 0px) translate(0%, 0%)");
}

#[test]
fn node_toolbar_transform_bottom_keeps_zero_vertical_shift() {
    let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    let viewport = Viewport::new(0.0, 0.0, 1.0);
    let transform =
        get_node_toolbar_transform(rect, viewport, Position::Bottom, 10.0, Alignment::End);
    assert_eq!(transform, "translate(100px, 60px) translate(-100%, 0%)");
}

#[test]
fn node_toolbar_transform_left_uses_full_left_shift() {
    let rect = Rect::new(50.0, 50.0, 100.0, 50.0);
    let viewport = Viewport::new(0.0, 0.0, 1.0);
    let transform =
        get_node_toolbar_transform(rect, viewport, Position::Left, 5.0, Alignment::Center);
    assert_eq!(transform, "translate(45px, 75px) translate(-100%, -50%)");
}

#[test]
fn edge_toolbar_transform_scales_by_inverse_zoom() {
    let transform = get_edge_toolbar_transform(
        10.0,
        20.0,
        2.0,
        ToolbarAlignX::Center,
        ToolbarAlignY::Center,
    );
    assert_eq!(
        transform,
        "translate(10px, 20px) scale(0.5) translate(-50%, -50%)"
    );
}

#[test]
fn edge_toolbar_transform_supports_corner_alignment() {
    let transform =
        get_edge_toolbar_transform(0.0, 0.0, 1.0, ToolbarAlignX::Left, ToolbarAlignY::Top);
    assert_eq!(transform, "translate(0px, 0px) scale(1) translate(0%, 0%)");
}

#[test]
fn edge_toolbar_transform_guards_zero_zoom() {
    let transform =
        get_edge_toolbar_transform(0.0, 0.0, 0.0, ToolbarAlignX::Right, ToolbarAlignY::Bottom);
    assert!(transform.contains("scale(1)"));
}

#[test]
fn aria_label_config_defaults_are_official() {
    let config = AriaLabelConfig::default();
    assert_eq!(config.controls_aria_label, "Control Panel");
    assert_eq!(config.controls_zoom_in_aria_label, "Zoom In");
    assert_eq!(config.controls_zoom_out_aria_label, "Zoom Out");
    assert_eq!(config.controls_fit_view_aria_label, "Fit View");
    assert_eq!(
        config.controls_interactive_aria_label,
        "Toggle Interactivity"
    );
    assert_eq!(config.minimap_aria_label, "Mini Map");
    assert_eq!(config.handle_aria_label, "Handle");
}

#[test]
fn aria_live_message_formats_position() {
    let config = AriaLabelConfig::default();
    assert_eq!(
        config.a11y_live_message("up", 10.0, 20.5),
        "Moved selected node up. New position, x: 10, y: 20.5"
    );
}

#[test]
fn merge_aria_label_config_returns_default_without_partial() {
    let merged = merge_aria_label_config(None);
    assert_eq!(merged, AriaLabelConfig::default());
}

#[test]
fn merge_aria_label_config_keeps_partial_values() {
    let partial = AriaLabelConfig {
        handle_aria_label: String::from("Knob"),
        ..AriaLabelConfig::default()
    };
    let merged = merge_aria_label_config(Some(partial));
    assert_eq!(merged.handle_aria_label, "Knob");
    assert_eq!(merged.controls_aria_label, "Control Panel");
}

#[test]
fn shallow_node_data_compares_id_type_and_data() {
    let a = vec![node("a", 0.0, 0.0, 1.0, 1.0)];
    let b = vec![node("a", 99.0, 99.0, 5.0, 5.0)];
    assert!(shallow_node_data(&a, &b));

    let mut with_type = b.clone();
    with_type[0].node_type = Some(String::from("custom"));
    assert!(!shallow_node_data(&a, &with_type));

    let mut with_data = b.clone();
    with_data[0].data = String::from("other");
    assert!(!shallow_node_data(&a, &with_data));
}

#[test]
fn shallow_node_data_requires_equal_length() {
    let a = vec![node("a", 0.0, 0.0, 1.0, 1.0)];
    assert!(!shallow_node_data(&a, &[]));
}

#[test]
fn dev_warn_formats_official_message() {
    let warn = DevWarn::new("React Flow", "https://reactflow.dev/");
    assert_eq!(
        warn.message("002", "it broke"),
        "[React Flow]: it broke Help: https://reactflow.dev/error#002"
    );
}

#[test]
fn is_attribution_visible_true_without_pane() {
    assert!(is_attribution_visible(None, None));
}

#[test]
fn is_attribution_visible_false_without_attribution_rect() {
    assert!(!is_attribution_visible(
        Some(Rect::new(0.0, 0.0, 10.0, 10.0)),
        None
    ));
}

#[test]
fn is_attribution_visible_false_for_zero_sized_attribution() {
    assert!(!is_attribution_visible(
        Some(Rect::new(0.0, 0.0, 10.0, 10.0)),
        Some(Rect::ZERO)
    ));
}

#[test]
fn is_attribution_visible_true_for_visible_attribution() {
    assert!(is_attribution_visible(
        Some(Rect::new(0.0, 0.0, 100.0, 100.0)),
        Some(Rect::new(0.0, 90.0, 40.0, 10.0))
    ));
}

#[test]
fn attribution_warning_message_capitalizes_library() {
    let message = handle_attribution_warning_message("react");
    assert!(message.starts_with("React Flow:"));
    assert!(message.contains("https://reactflow.dev/remove-attr"));
}

#[test]
fn get_nodes_intersecting_rect_excludes_self() {
    let nodes = vec![
        node("a", 0.0, 0.0, 100.0, 100.0),
        node("b", 50.0, 50.0, 100.0, 100.0),
        node("c", 500.0, 500.0, 10.0, 10.0),
    ];
    let lookup = build_lookup(nodes);
    let internals: Vec<_> = lookup.values().cloned().collect();
    let matches = get_nodes_intersecting_rect(
        &internals,
        Rect::new(0.0, 0.0, 100.0, 100.0),
        true,
        Some("a"),
    );
    let ids: Vec<&str> = matches.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, vec!["b"]);
}

#[test]
fn get_nodes_intersecting_rect_requires_full_overlap_when_not_partial() {
    let nodes = vec![
        node("a", 0.0, 0.0, 100.0, 100.0),
        node("b", 50.0, 50.0, 100.0, 100.0),
    ];
    let lookup = build_lookup(nodes);
    let internals: Vec<_> = lookup.values().cloned().collect();
    let matches = get_nodes_intersecting_rect(
        &internals,
        Rect::new(0.0, 0.0, 100.0, 100.0),
        false,
        Some("a"),
    );
    assert!(matches.is_empty());
}

#[test]
fn is_node_intersecting_matches_overlap_rules() {
    let nodes = vec![node("a", 0.0, 0.0, 100.0, 100.0)];
    let lookup = build_lookup(nodes);
    let internal = lookup.get("a").expect("node exists");

    assert!(is_node_intersecting(
        internal,
        Rect::new(0.0, 0.0, 50.0, 50.0),
        true
    ));
    assert!(!is_node_intersecting(
        internal,
        Rect::new(200.0, 200.0, 50.0, 50.0),
        false
    ));
    assert!(is_node_intersecting(
        internal,
        Rect::new(0.0, 0.0, 100.0, 100.0),
        false
    ));
}

#[test]
fn resolvers_store_and_take_value_once() {
    let mut resolvers = Resolvers::new();
    assert!(!resolvers.is_resolved());
    resolvers.resolve(42);
    assert!(resolvers.is_resolved());
    assert_eq!(resolvers.try_resolve(), Some(42));
    assert_eq!(resolvers.try_resolve(), None);
}

#[test]
fn reconnect_edge_replaces_connection_and_keeps_style() {
    let mut old = Edge::<String>::new("e1", "a", "b");
    old.label = Some(String::from("hello"));
    let edges = vec![old.clone()];
    let connection = Connection::new("a", "c");

    let result = gpui_xyflow::core::connections::reconnect_edge(
        &old,
        &connection,
        &edges,
        gpui_xyflow::core::connections::ReconnectEdgeOptions::default(),
    )
    .expect("reconnect succeeds");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].target, "c");
    assert_eq!(result[0].label.as_deref(), Some("hello"));
    assert_eq!(result[0].id, "xy-edge__a-c");
}

#[test]
fn reconnect_edge_can_keep_old_id() {
    let old = Edge::<String>::new("e1", "a", "b");
    let connection = Connection::new("a", "c");
    let result = gpui_xyflow::core::connections::reconnect_edge(
        &old,
        &connection,
        std::slice::from_ref(&old),
        gpui_xyflow::core::connections::ReconnectEdgeOptions {
            should_replace_id: false,
        },
    )
    .expect("reconnect succeeds");
    assert_eq!(result[0].id, "e1");
}

#[test]
fn reconnect_edge_rejects_missing_endpoint() {
    let old = Edge::<String>::new("e1", "a", "b");
    let connection = Connection::new("a", "");
    let error = gpui_xyflow::core::connections::reconnect_edge(
        &old,
        &connection,
        std::slice::from_ref(&old),
        Default::default(),
    )
    .expect_err("must fail");
    assert_eq!(error.code(), "006");
}

#[test]
fn reconnect_edge_rejects_unknown_old_edge() {
    let old = Edge::<String>::new("e1", "a", "b");
    let connection = Connection::new("a", "c");
    let error =
        gpui_xyflow::core::connections::reconnect_edge(&old, &connection, &[], Default::default())
            .expect_err("must fail");
    assert_eq!(error.code(), "007");
    assert!(error.message().contains("e1"));
}

#[test]
fn transform_and_viewport_helpers_are_consistent() {
    let viewport = Viewport::new(10.0, 20.0, 2.0);
    let transform = viewport.transform();
    assert_eq!(transform, Transform::new(10.0, 20.0, 2.0));
    assert_eq!(transform.viewport(), viewport);
    let _ = Box2::new(0.0, 0.0, 1.0, 1.0);
    let _ = CoordinateExtent::default();
    let _ = XYPosition::ZERO;
}

fn build_lookup(nodes: Vec<Node<String>>) -> gpui_xyflow::core::node::NodeLookup<String> {
    let mut lookup = gpui_xyflow::core::node::NodeLookup::new();
    let mut parent_lookup = gpui_xyflow::core::node::ParentLookup::new();
    let options = gpui_xyflow::core::store::AdoptOptions::default();
    gpui_xyflow::core::store::adopt_user_nodes(&nodes, &mut lookup, &mut parent_lookup, &options);
    lookup
}
