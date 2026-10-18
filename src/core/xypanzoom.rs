use crate::core::geometry::{Rect, Transform, XYPosition};

pub fn constrain_transform(
    transform: Transform,
    extent: Rect,
    translate_extent: [[f64; 2]; 2],
) -> Transform {
    let dx0 = transform.invert(XYPosition::new(extent.x, extent.y)).x - translate_extent[0][0];
    let dx1 = transform
        .invert(XYPosition::new(extent.max_x(), extent.y))
        .x
        - translate_extent[1][0];
    let dy0 = transform.invert(XYPosition::new(extent.x, extent.y)).y - translate_extent[0][1];
    let dy1 = transform
        .invert(XYPosition::new(extent.x, extent.max_y()))
        .y
        - translate_extent[1][0];

    let tx = if dx1 > dx0 {
        (dx0 + dx1) / 2.0
    } else {
        (0.0f64).min(dx0).max((0.0f64).max(dx1))
    };
    let ty = if dy1 > dy0 {
        (dy0 + dy1) / 2.0
    } else {
        (0.0f64).min(dy0).max((0.0f64).max(dy1))
    };

    Transform::new(
        transform.x() + transform.zoom() * tx,
        transform.y() + transform.zoom() * ty,
        transform.zoom(),
    )
}

pub fn minimize_transform(transform: Transform, extent: Rect, translate_extent: Rect) -> Transform {
    if translate_extent.width == 0.0 || translate_extent.height == 0.0 {
        return transform;
    }
    let k = transform.zoom();
    let x = transform.x().clamp(
        extent.x - translate_extent.max_x() * k,
        extent.max_x() - translate_extent.x * k,
    );
    let y = transform.y().clamp(
        extent.y - translate_extent.max_y() * k,
        extent.max_y() - translate_extent.y * k,
    );
    Transform::new(x, y, k)
}
