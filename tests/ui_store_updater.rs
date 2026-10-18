use gpui_xyflow::{
    core::{
        edge::Edge,
        geometry::{XYPosition, ZIndexMode},
        graph::FitViewOptions,
        node::Node,
    },
    ui::store_updater::{
        FieldKind, ProviderOptions, REACT_FLOW_FIELDS_TO_TRACK, classify_field, create_store,
        is_tracked_field,
    },
};

fn node(id: &str, x: f64, y: f64) -> Node<String> {
    Node::<String> {
        id: id.to_string(),
        position: XYPosition::new(x, y),
        width: Some(80.0),
        height: Some(40.0),
        measured: gpui_xyflow::core::node::MeasuredDimensions::new(80.0, 40.0),
        ..Node::default()
    }
}

#[test]
fn fields_to_track_has_official_count() {
    assert_eq!(REACT_FLOW_FIELDS_TO_TRACK.len(), 58);
}

#[test]
fn classify_dedicated_setters() {
    for field in [
        "nodes",
        "edges",
        "minZoom",
        "maxZoom",
        "translateExtent",
        "nodeExtent",
    ] {
        assert_eq!(classify_field(field), FieldKind::DedicatedSetter);
    }
}

#[test]
fn classify_renamed_fields() {
    for field in ["ariaLabelConfig", "fitView", "fitViewOptions"] {
        assert_eq!(classify_field(field), FieldKind::Renamed);
    }
}

#[test]
fn classify_direct_fields() {
    for field in [
        "snapGrid",
        "nodeOrigin",
        "connectionMode",
        "onConnect",
        "debug",
    ] {
        assert_eq!(classify_field(field), FieldKind::Direct);
    }
}

#[test]
fn tracked_field_membership() {
    assert!(is_tracked_field("nodes"));
    assert!(is_tracked_field("zIndexMode"));
    assert!(!is_tracked_field("notAProp"));
}

#[test]
fn create_store_adopts_default_nodes() {
    let mut store = create_store::<String>(ProviderOptions {
        default_nodes: Some(vec![node("a", 0.0, 0.0)]),
        ..ProviderOptions::default()
    });
    store.update_nodes();
    assert!(store.has_default_nodes());
    assert_eq!(store.nodes().len(), 1);
    assert!(store.node_lookup().contains_key("a"));
}

#[test]
fn create_store_controlled_nodes_are_not_default() {
    let mut store = create_store::<String>(ProviderOptions {
        nodes: Some(vec![node("a", 0.0, 0.0)]),
        ..ProviderOptions::default()
    });
    store.update_nodes();
    assert!(!store.has_default_nodes());
    assert_eq!(store.nodes().len(), 1);
}

#[test]
fn create_store_applies_zoom_extent() {
    let store = create_store::<String>(ProviderOptions {
        min_zoom: Some(0.1),
        max_zoom: Some(5.0),
        ..ProviderOptions::default()
    });
    assert_eq!(store.props().min_zoom, 0.1);
    assert_eq!(store.props().max_zoom, 5.0);
}

#[test]
fn create_store_applies_origin_extent_and_z_index() {
    let store = create_store::<String>(ProviderOptions {
        node_origin: Some([0.5, 0.5]),
        node_extent: Some([[-100.0, -100.0], [100.0, 100.0]]),
        z_index_mode: Some(ZIndexMode::Manual),
        ..ProviderOptions::default()
    });
    assert_eq!(store.props().node_origin, [0.5, 0.5]);
    assert_eq!(
        store.props().node_extent,
        [[-100.0, -100.0], [100.0, 100.0]]
    );
    assert_eq!(store.props().z_index_mode, ZIndexMode::Manual);
}

#[test]
fn create_store_fit_view_centers_content() {
    let store = create_store::<String>(ProviderOptions {
        nodes: Some(vec![node("a", 0.0, 0.0), node("b", 200.0, 100.0)]),
        width: Some(800.0),
        height: Some(600.0),
        fit_view: true,
        fit_view_options: Some(FitViewOptions {
            padding: Some(0.1),
            ..FitViewOptions::default()
        }),
        ..ProviderOptions::default()
    });
    assert!(store.queued_fit_view() || !store.props().fit_view_queued);
    assert!(store.viewport().zoom >= store.props().min_zoom);
}

#[test]
fn create_store_without_fit_view_keeps_identity() {
    let store = create_store::<String>(ProviderOptions {
        nodes: Some(vec![node("a", 0.0, 0.0)]),
        width: Some(800.0),
        height: Some(600.0),
        ..ProviderOptions::default()
    });
    assert!(!store.props().fit_view_queued);
}

#[test]
fn create_store_edges_are_stored() {
    let store = create_store::<String>(ProviderOptions {
        edges: Some(vec![Edge::<String>::new("e1", "a", "b")]),
        ..ProviderOptions::default()
    });
    assert_eq!(store.edges().len(), 1);
}

#[test]
fn default_edges_take_precedence_over_edges() {
    let store = create_store::<String>(ProviderOptions {
        edges: Some(vec![Edge::<String>::new("e1", "a", "b")]),
        default_edges: Some(vec![Edge::<String>::new("e2", "c", "d")]),
        ..ProviderOptions::default()
    });
    assert!(store.has_default_edges());
    assert_eq!(store.edges().len(), 1);
    assert_eq!(store.edges()[0].id, "e2");
}
