use crate::core::geometry::{
    CoordinateExtent, Rect, Transform, Viewport, XYPosition,
};
use crate::core::node::{InternalNode, NodeLookup};
use crate::core::utils::get_bounds_of_rect_slice;

pub const DEFAULT_MINIMAP_WIDTH: f64 = 200.0;
pub const DEFAULT_MINIMAP_HEIGHT: f64 = 150.0;
pub const DEFAULT_OFFSET_SCALE: f64 = 5.0;
pub const DEFAULT_NODE_BORDER_RADIUS: f64 = 5.0;

#[derive(Clone, Debug, PartialEq)]
pub struct MinimapUpdate {
    pub translate_extent: CoordinateExtent,
    pub width: f64,
    pub height: f64,
    pub inverse_pan: bool,
    pub zoom_step: f64,
    pub pannable: bool,
    pub zoomable: bool,
}

impl Default for MinimapUpdate {
    fn default() -> Self {
        MinimapUpdate {
            translate_extent: crate::core::constants::INFINITE_EXTENT,
            width: 0.0,
            height: 0.0,
            inverse_pan: false,
            zoom_step: 1.0,
            pannable: true,
            zoomable: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinimapLayout {
    pub view_x: f64,
    pub view_y: f64,
    pub view_width: f64,
    pub view_height: f64,
    pub width: f64,
    pub height: f64,
    pub offset: f64,
    pub view_scale: f64,
}

impl Default for MinimapLayout {
    fn default() -> Self {
        MinimapLayout {
            view_x: 0.0,
            view_y: 0.0,
            view_width: 0.0,
            view_height: 0.0,
            width: 0.0,
            height: 0.0,
            offset: 0.0,
            view_scale: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinimapNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub selected: bool,
    pub color: Option<String>,
}

pub fn view_bounding_box(transform: Transform, width: f64, height: f64) -> Rect {
    Rect::new(
        -transform.x() / transform.zoom(),
        -transform.y() / transform.zoom(),
        width / transform.zoom(),
        height / transform.zoom(),
    )
}

pub fn minimap_bounding_rect<S: Clone>(
    node_lookup: &NodeLookup<S>,
    view_bb: Rect,
    include_hidden: bool,
) -> Rect {
    let has_visible = node_lookup
        .values()
        .any(|node| include_hidden || !node.hidden);

    if !has_visible {
        return view_bb;
    }

    let mut rects = vec![view_bb];

    for node in node_lookup.values() {
        if !include_hidden && node.hidden {
            continue;
        }

        let dimensions = node.size();
        rects.push(Rect::new(
            node.internals.position_absolute.x,
            node.internals.position_absolute.y,
            dimensions.width,
            dimensions.height,
        ));
    }

    get_bounds_of_rect_slice(&rects)
}

pub fn minimap_layout(
    bounding_rect: Rect,
    element_width: f64,
    element_height: f64,
    offset_scale: f64,
) -> MinimapLayout {
    let width = if element_width > 0.0 {
        element_width
    } else {
        DEFAULT_MINIMAP_WIDTH
    };
    let height = if element_height > 0.0 {
        element_height
    } else {
        DEFAULT_MINIMAP_HEIGHT
    };

    let scaled_width = bounding_rect.width / width;
    let scaled_height = bounding_rect.height / height;
    let view_scale = scaled_width.max(scaled_height);
    let view_width = view_scale * width;
    let view_height = view_scale * height;
    let offset = offset_scale * view_scale;

    let x = bounding_rect.x - (view_width - bounding_rect.width) / 2.0 - offset;
    let y = bounding_rect.y - (view_height - bounding_rect.height) / 2.0 - offset;

    MinimapLayout {
        view_x: bounding_rect.x,
        view_y: bounding_rect.y,
        view_width: bounding_rect.width,
        view_height: bounding_rect.height,
        width: view_width + offset * 2.0,
        height: view_height + offset * 2.0,
        offset,
        view_scale,
    }
}

pub fn minimap_viewbox(layout: &MinimapLayout) -> String {
    format!(
        "{} {} {} {}",
        layout.view_x - (layout.width - layout.view_width) / 2.0,
        layout.view_y - (layout.height - layout.view_height) / 2.0,
        layout.width,
        layout.height
    )
}

pub fn minimap_mask_path(layout: &MinimapLayout) -> String {
    let x = layout.view_x - (layout.width - layout.view_width) / 2.0 - layout.offset;
    let y = layout.view_y - (layout.height - layout.view_height) / 2.0 - layout.offset;
    let width = layout.width;
    let height = layout.height;
    let offset = layout.offset;
    let view_x = layout.view_x;
    let view_y = layout.view_y;
    let view_width = layout.view_width;
    let view_height = layout.view_height;

    format!(
        "M{},{}h{}v{}h{}z M{},{}h{}v{}h{}z",
        x - offset,
        y - offset,
        width + offset * 2.0,
        height + offset * 2.0,
        -width - offset * 2.0,
        view_x,
        view_y,
        view_width,
        view_height,
        -view_width
    )
}

pub fn minimap_nodes<S: Clone>(
    node_lookup: &NodeLookup<S>,
    color: Option<&str>,
) -> Vec<MinimapNode> {
    let mut result = Vec::new();

    for (id, node) in node_lookup {
        if node.hidden {
            continue;
        }

        let dimensions = node.size();

        if dimensions.width <= 0.0 || dimensions.height <= 0.0 {
            continue;
        }

        result.push(MinimapNode {
            id: id.clone(),
            x: node.internals.position_absolute.x,
            y: node.internals.position_absolute.y,
            width: dimensions.width,
            height: dimensions.height,
            selected: node.selected,
            color: color.map(|value| value.to_string()),
        });
    }

    result
}

pub fn minimap_pan(
    viewport: Viewport,
    pointer_delta: XYPosition,
    view_scale: f64,
    inverse_pan: bool,
) -> Viewport {
    let move_scale = view_scale * viewport.zoom.max(viewport.zoom.ln()) * if inverse_pan { -1.0 } else { 1.0 };

    Viewport::new(
        viewport.x - pointer_delta.x * move_scale,
        viewport.y - pointer_delta.y * move_scale,
        viewport.zoom,
    )
}

pub fn minimap_zoom(
    viewport: Viewport,
    delta: f64,
    zoom_step: f64,
    scale_extent: (f64, f64),
) -> Viewport {
    Viewport::new(
        viewport.x,
        viewport.y,
        (viewport.zoom * 2f64.powf(delta * zoom_step)).clamp(scale_extent.0, scale_extent.1),
    )
}

pub fn minimap_node_rects<S: Clone>(node_lookup: &NodeLookup<S>) -> Vec<Rect> {
    node_lookup
        .values()
        .filter(|node| !node.hidden)
        .map(|node| {
            let dimensions = node.size();
            Rect::new(
                node.internals.position_absolute.x,
                node.internals.position_absolute.y,
                dimensions.width,
                dimensions.height,
            )
        })
        .collect()
}

pub fn minimap_bounds<S: Clone>(node_lookup: &NodeLookup<S>) -> Option<Rect> {
    let rects = minimap_node_rects(node_lookup);

    if rects.is_empty() {
        return None;
    }

    Some(get_bounds_of_rect_slice(&rects))
}

pub fn minimap_color_for<S: Clone>(node: &InternalNode<S>, default_color: &str) -> String {
    if node.selected {
        "#ff0072".to_string()
    } else {
        default_color.to_string()
    }
}
