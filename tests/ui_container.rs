use gpui_xyflow::{
    core::{
        edge::Edge,
        geometry::{Dimensions, PanOnScrollMode, Rect, Transform, Viewport, XYPosition},
        node::{Node, NodeLookup, ParentLookup},
        panzoom::PanOnDrag,
        store::{AdoptOptions, adopt_user_nodes},
        xypanzoom::PanZoomUpdateOptions,
    },
    ui::{container::*, store::FlowProps},
};

fn props() -> FlowProps<String> {
    FlowProps::default()
}

fn keys() -> RendererKeyState {
    RendererKeyState::default()
}

#[test]
fn default_props_derive_pan_on_drag_enabled() {
    let derived = derive_renderer_state(&props(), keys(), false);
    assert!(derived.pan_on_drag);
    assert!(!derived.pan_on_scroll);
    assert!(!derived.is_selecting);
}

#[test]
fn pan_activation_key_forces_pan_on_drag_and_scroll() {
    let mut props = props();
    props.pan_on_drag = PanOnDrag::Disabled;
    props.pan_on_scroll = false;
    let mut keys = keys();
    keys.pan_activation_key_pressed = true;

    let derived = derive_renderer_state(&props, keys, false);
    assert!(derived.pan_on_drag);
    assert!(derived.pan_on_scroll);
}

#[test]
fn pan_on_scroll_disabled_by_default() {
    let derived = derive_renderer_state(&props(), keys(), false);
    assert!(!derived.zoom_pane_pan_on_scroll);
}

#[test]
fn selection_on_drag_disabled_when_pan_on_drag_always_enabled() {
    let mut props = props();
    props.selection_on_drag = true;
    props.pan_on_drag = PanOnDrag::Enabled;
    let derived = derive_renderer_state(&props, keys(), false);
    assert!(!derived.selection_on_drag);
}

#[test]
fn selection_on_drag_enabled_when_pan_on_drag_is_button_list() {
    let mut props = props();
    props.selection_on_drag = true;
    props.pan_on_drag = PanOnDrag::Buttons(vec![1]);
    let derived = derive_renderer_state(&props, keys(), false);
    assert!(derived.selection_on_drag);
    assert!(derived.is_selecting);
}

#[test]
fn selection_key_pressed_sets_selecting_and_blocks_zoom_pane_drag() {
    let mut keys = keys();
    keys.selection_key_pressed = true;
    let derived = derive_renderer_state(&props(), keys, false);
    assert!(derived.is_selecting);
    assert!(!derived.zoom_pane_pan_on_drag);
    assert!(derived.pan_on_drag);
}

#[test]
fn user_selection_active_counts_as_selecting() {
    let derived = derive_renderer_state(&props(), keys(), true);
    assert!(derived.is_selecting);
}

#[test]
fn pane_options_mirror_derived_state() {
    let mut props = props();
    props.pan_on_drag = PanOnDrag::Buttons(vec![0, 2]);
    let derived = derive_renderer_state(&props, keys(), false);
    let pane = derive_pane_options(&props, derived, keys());
    assert_eq!(pane.pan_on_drag, derived.pan_on_drag);
    assert_eq!(pane.is_selecting, derived.is_selecting);
    assert_eq!(pane.pane_click_distance, props.pane_click_distance);
}

#[test]
fn zoom_pane_update_options_copies_props() {
    let mut props = props();
    props.zoom_on_scroll = false;
    props.zoom_on_pinch = false;
    props.prevent_scrolling = false;
    props.pan_on_scroll_mode = PanOnScrollMode::Vertical;
    props.no_wheel_class_name = String::from("custom-wheel");
    props.no_pan_class_name = String::from("custom-pan");

    let derived = derive_renderer_state(&props, keys(), false);
    let options = zoom_pane_update_options(&props, derived, keys(), false, false);

    assert!(!options.zoom_on_scroll);
    assert!(!options.zoom_on_pinch);
    assert!(!options.prevent_scrolling);
    assert_eq!(options.pan_on_scroll_mode, PanOnScrollMode::Vertical);
    assert_eq!(options.no_wheel_class_name, "custom-wheel");
    assert_eq!(options.no_pan_class_name, "custom-pan");
    assert_eq!(options.lib, props.lib);
}

#[test]
fn zoom_pane_update_options_disables_drag_when_selection_key_pressed() {
    let mut keys = keys();
    keys.selection_key_pressed = true;
    let props = props();
    let derived = derive_renderer_state(&props, keys, false);
    let options = zoom_pane_update_options(&props, derived, keys, false, false);
    assert_eq!(options.pan_on_drag, PanOnDrag::Disabled);
}

#[test]
fn zoom_pane_update_options_tracks_connection_in_progress() {
    let props = props();
    let derived = derive_renderer_state(&props, keys(), false);
    let options = zoom_pane_update_options(&props, derived, keys(), false, true);
    assert!(options.connection_in_progress);
}

#[test]
fn zoom_pane_mounts_and_reports_viewport() {
    let props = props();
    let viewport = Viewport::new(10.0, 20.0, 1.5);
    let pane = ZoomPane::mount(&props, viewport, [[0.0, 0.0], [1000.0, 1000.0]], false);

    assert!(pane.mounted());
    assert!(!pane.destroyed());
    assert_eq!(pane.viewport(), viewport);
    assert_eq!(pane.transform(), Transform::new(10.0, 20.0, 1.5));
}

#[test]
fn zoom_pane_destroy_and_remount() {
    let props = props();
    let mut pane = ZoomPane::mount(
        &props,
        Viewport::IDENTITY,
        [[0.0, 0.0], [100.0, 100.0]],
        false,
    );
    pane.destroy();
    assert!(pane.destroyed());
    assert!(!pane.mounted());
    pane.remount();
    assert!(!pane.destroyed());
    assert!(pane.mounted());
}

#[test]
fn zoom_pane_update_requires_mount() {
    let mut pane: ZoomPane<String> = ZoomPane::default();
    assert!(!pane.update(&PanZoomUpdateOptions::default()));
}

#[test]
fn zoom_pane_update_succeeds_when_mounted() {
    let props = props();
    let mut pane = ZoomPane::mount(
        &props,
        Viewport::IDENTITY,
        [[0.0, 0.0], [100.0, 100.0]],
        false,
    );
    assert!(pane.update(&PanZoomUpdateOptions::default()));
}

#[test]
fn controlled_viewport_ignores_transform_change() {
    let props = props();
    let mut pane = ZoomPane::mount(
        &props,
        Viewport::IDENTITY,
        [[0.0, 0.0], [100.0, 100.0]],
        true,
    );
    let next = Viewport::new(5.0, 5.0, 2.0);
    pane.on_transform_change(next);
    assert_eq!(pane.viewport(), Viewport::IDENTITY);
}

#[test]
fn uncontrolled_viewport_applies_transform_change() {
    let props = props();
    let mut pane = ZoomPane::mount(
        &props,
        Viewport::IDENTITY,
        [[0.0, 0.0], [100.0, 100.0]],
        false,
    );
    let next = Viewport::new(5.0, 5.0, 2.0);
    pane.on_transform_change(next);
    assert_eq!(pane.viewport(), next);
}

#[test]
fn zoom_pane_update_from_props_uses_current_state() {
    let mut props = props();
    props.pan_on_scroll = true;
    let mut pane = ZoomPane::mount(
        &props,
        Viewport::IDENTITY,
        [[0.0, 0.0], [100.0, 100.0]],
        false,
    );
    assert!(pane.update_from_props(&props, keys(), false, false));
}

#[test]
fn render_layer_order_matches_dom_order() {
    let order: Vec<&str> = RenderLayer::ORDER
        .iter()
        .map(|layer| layer.as_str())
        .collect();
    assert_eq!(
        order,
        vec![
            "edges",
            "connectionline",
            "edgelabels",
            "nodes",
            "viewportportal"
        ]
    );
}

#[test]
fn render_layer_class_name_uses_rf_id() {
    assert_eq!(
        RenderLayer::Edges.class_name("react-flow"),
        "react-flow__edges"
    );
}

#[test]
fn render_snapshot_without_visibility_filter_keeps_all() {
    let nodes = vec![node("a", 0.0, 0.0), node("b", 5000.0, 5000.0)];
    let lookup = build_lookup_for_test(nodes);
    let edges = vec![Edge::<String>::new("e1", "a", "b")];
    let snapshot = render_snapshot(
        Transform::IDENTITY,
        Dimensions::new(800.0, 600.0),
        &lookup,
        &edges,
        false,
    );
    assert_eq!(snapshot.visible_nodes.len(), 2);
    assert_eq!(snapshot.visible_edges.len(), 1);
    assert_eq!(snapshot.layers.len(), 5);
}

#[test]
fn render_snapshot_with_visibility_filter_drops_offscreen() {
    let nodes = vec![
        node("a", 0.0, 0.0),
        node("b", 5000.0, 5000.0),
        node("c", 6000.0, 6000.0),
    ];
    let lookup = build_lookup_for_test(nodes);
    let edges = vec![Edge::<String>::new("e1", "b", "c")];
    let snapshot = render_snapshot(
        Transform::IDENTITY,
        Dimensions::new(800.0, 600.0),
        &lookup,
        &edges,
        true,
    );
    assert_eq!(snapshot.visible_nodes[0].id, "a");
    assert!(snapshot.visible_edges.is_empty());
}

#[test]
fn render_snapshot_keeps_edge_when_both_ends_visible() {
    let nodes = vec![node("a", 0.0, 0.0), node("b", 100.0, 100.0)];
    let lookup = build_lookup_for_test(nodes);
    let edges = vec![Edge::<String>::new("e1", "a", "b")];
    let snapshot = render_snapshot(
        Transform::IDENTITY,
        Dimensions::new(800.0, 600.0),
        &lookup,
        &edges,
        true,
    );
    assert_eq!(snapshot.visible_edges.len(), 1);
}

#[test]
fn viewport_transform_matches_viewport_fields() {
    let viewport = Viewport::new(3.0, 4.0, 0.5);
    assert_eq!(viewport_transform(viewport), Transform::new(3.0, 4.0, 0.5));
}

#[test]
fn node_is_visible_reports_lookup_membership() {
    let lookup = build_lookup_for_test(vec![node("a", 0.0, 0.0)]);
    let present = node("a", 0.0, 0.0);
    let absent = node("zzz", 0.0, 0.0);
    assert!(node_is_visible(&present, &lookup));
    assert!(!node_is_visible(&absent, &lookup));
}

#[test]
fn flow_props_defaults_for_container_fields() {
    let props = props();
    assert_eq!(props.pan_on_scroll_speed, 0.5);
    assert_eq!(props.pane_click_distance, 1.0);
    assert_eq!(props.node_click_distance, 0.0);
    assert_eq!(props.no_wheel_class_name, "nowheel");
    assert_eq!(props.no_pan_class_name, "nopan");
}

#[test]
fn adopt_options_helper_stays_constructible() {
    let options = AdoptOptions::default();
    assert_eq!(options.node_origin, [0.0, 0.0]);
}

fn build_lookup_for_test(nodes: Vec<Node<String>>) -> NodeLookup<String> {
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

fn node(id: &str, x: f64, y: f64) -> Node<String> {
    Node::<String> {
        id: id.to_string(),
        position: XYPosition::new(x, y),
        width: Some(80.0),
        height: Some(40.0),
        ..Node::default()
    }
}

#[test]
fn empty_rect_is_comparable() {
    assert_eq!(Rect::ZERO.width, 0.0);
}
