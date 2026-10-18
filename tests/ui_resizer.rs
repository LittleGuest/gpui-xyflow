use gpui_xyflow::{
    core::{
        change::{NodeChange, NodeDimensionChange, NodePositionChange},
        geometry::{Dimensions, XYPosition},
        xyresizer::XYResizerChange,
    },
    ui::resizer::resize_change_to_node_changes,
};

#[test]
fn resize_change_produces_position_and_dimensions() {
    let change = XYResizerChange {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 100.0,
        direction: [1.0, 1.0],
    };
    let changes = resize_change_to_node_changes::<String>("n1", &change, true);
    assert_eq!(changes.len(), 2);

    let position = match &changes[0] {
        NodeChange::Position(p) => p,
        other => panic!("expected position change, got {other:?}"),
    };
    assert_eq!(position.id, "n1");
    assert_eq!(position.position, Some(XYPosition::new(10.0, 20.0)));

    let dimensions = match &changes[1] {
        NodeChange::Dimensions(d) => d,
        other => panic!("expected dimensions change, got {other:?}"),
    };
    assert_eq!(dimensions.id, "n1");
    assert_eq!(dimensions.dimensions, Some(Dimensions::new(200.0, 100.0)));
    assert_eq!(dimensions.resizing, Some(true));
}

#[test]
fn resize_change_resizing_flag_is_forwarded() {
    let change = XYResizerChange {
        x: 0.0,
        y: 0.0,
        width: 50.0,
        height: 50.0,
        direction: [-1.0, -1.0],
    };
    let changes = resize_change_to_node_changes::<String>("n1", &change, false);
    let dimensions = match &changes[1] {
        NodeChange::Dimensions(d) => d,
        other => panic!("expected dimensions change, got {other:?}"),
    };
    assert_eq!(dimensions.resizing, Some(false));
}

#[test]
fn resize_change_struct_fields_are_stable() {
    let change = XYResizerChange {
        x: 1.0,
        y: 2.0,
        width: 3.0,
        height: 4.0,
        direction: [0.0, 1.0],
    };
    assert_eq!(change.width, 3.0);
    assert_eq!(change.height, 4.0);
    let _ = NodePositionChange {
        id: "x".to_string(),
        position: None,
        position_absolute: None,
        dragging: None,
    };
    let _ = NodeDimensionChange {
        id: "x".to_string(),
        dimensions: None,
        resizing: None,
        set_attributes: None,
    };
}
