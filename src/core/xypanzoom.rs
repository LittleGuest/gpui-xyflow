use crate::core::geometry::{Rect, Transform};

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
