use crate::core::{
    geometry::{
        Box2, CoordinateExtent, Dimensions, NodeOrigin, Padding, PaddingWithUnit, Rect, SnapGrid,
        Transform, Viewport, XYPosition,
    },
    node::{InternalNode, Node, NodeDragItem, NodeLookup},
};

pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.min(max).max(min)
}

pub fn is_numeric(value: f64) -> bool {
    value.is_finite() && !value.is_nan()
}

pub fn clamp_position(
    position: XYPosition,
    extent: CoordinateExtent,
    dimensions: Dimensions,
) -> XYPosition {
    XYPosition::new(
        clamp(position.x, extent[0][0], extent[1][0] - dimensions.width),
        clamp(position.y, extent[0][1], extent[1][1] - dimensions.height),
    )
}

pub fn clamp_position_to_parent<S: Clone>(
    child_position: XYPosition,
    child_dimensions: Dimensions,
    parent: &InternalNode<S>,
) -> XYPosition {
    let parent_dimensions = parent.size();
    let parent_origin = parent.internals.position_absolute;
    clamp_position(
        child_position,
        [
            [parent_origin.x, parent_origin.y],
            [
                parent_origin.x + parent_dimensions.width,
                parent_origin.y + parent_dimensions.height,
            ],
        ],
        child_dimensions,
    )
}

fn calc_auto_pan_velocity(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        clamp((value - min).abs(), 1.0, min) / min
    } else if value > max {
        -clamp((value - max).abs(), 1.0, min) / min
    } else {
        0.0
    }
}

pub fn calc_auto_pan(
    position: XYPosition,
    bounds: Dimensions,
    speed: f64,
    distance: f64,
) -> XYPosition {
    let x = calc_auto_pan_velocity(position.x, distance, bounds.width - distance) * speed;
    let y = calc_auto_pan_velocity(position.y, distance, bounds.height - distance) * speed;
    XYPosition::new(x, y)
}

pub fn get_bounds_of_boxes(box1: Box2, box2: Box2) -> Box2 {
    Box2::new(
        box1.x.min(box2.x),
        box1.y.min(box2.y),
        box1.x2.max(box2.x2),
        box1.y2.max(box2.y2),
    )
}

pub fn get_bounds_of_box_list(boxes: &[Box2]) -> Box2 {
    boxes
        .iter()
        .copied()
        .reduce(get_bounds_of_boxes)
        .unwrap_or(Box2::EMPTY)
}

pub fn get_bounds_of_rect_slice(rects: &[Rect]) -> Rect {
    let boxes: Vec<Box2> = rects.iter().map(|rect| rect_to_box(*rect)).collect();
    box_to_rect(get_bounds_of_box_list(&boxes))
}

pub fn rect_to_box(rect: Rect) -> Box2 {
    Box2::new(rect.x, rect.y, rect.x + rect.width, rect.y + rect.height)
}

pub fn box_to_rect(b: Box2) -> Rect {
    Rect::new(b.x, b.y, b.x2 - b.x, b.y2 - b.y)
}

pub fn node_to_rect<S: Clone>(node: &Node<S>, origin: NodeOrigin) -> Rect {
    let dimensions = node.size();
    let position = node.position_with_origin(origin);
    Rect::new(position.x, position.y, dimensions.width, dimensions.height)
}

pub fn node_to_box<S: Clone>(node: &Node<S>, origin: NodeOrigin) -> Box2 {
    rect_to_box(node_to_rect(node, origin))
}

pub fn internal_node_to_rect<S: Clone>(node: &InternalNode<S>) -> Rect {
    let dimensions = node.size();
    Rect::new(
        node.internals.position_absolute.x,
        node.internals.position_absolute.y,
        dimensions.width,
        dimensions.height,
    )
}

pub fn internal_node_to_box<S: Clone>(node: &InternalNode<S>) -> Box2 {
    rect_to_box(internal_node_to_rect(node))
}

pub fn drag_item_to_box(item: &NodeDragItem) -> Box2 {
    let dimensions = item.measured;
    Box2::new(
        item.position_absolute.x,
        item.position_absolute.y,
        item.position_absolute.x + dimensions.width.unwrap_or(0.0),
        item.position_absolute.y + dimensions.height.unwrap_or(0.0),
    )
}

pub fn get_bounds_of_rects(rect1: Rect, rect2: Rect) -> Rect {
    box_to_rect(get_bounds_of_boxes(rect_to_box(rect1), rect_to_box(rect2)))
}

pub fn get_rects_overlapping_area(a: Rect, b: Rect) -> f64 {
    let x_overlap = 0.0_f64.max(a.max_x().min(b.max_x()) - a.x.max(b.x));
    let y_overlap = 0.0_f64.max(a.max_y().min(b.max_y()) - a.y.max(b.y));
    (x_overlap * y_overlap).ceil()
}

pub fn get_overlapping_area(a: Rect, b: Rect) -> f64 {
    get_rects_overlapping_area(a, b)
}

pub fn snap_position(position: XYPosition, snap_grid: SnapGrid) -> XYPosition {
    let grid_x = if snap_grid[0] == 0.0 {
        1.0
    } else {
        snap_grid[0]
    };
    let grid_y = if snap_grid[1] == 0.0 {
        1.0
    } else {
        snap_grid[1]
    };
    XYPosition::new(
        grid_x * (position.x / grid_x).round(),
        grid_y * (position.y / grid_y).round(),
    )
}

pub fn point_to_renderer_point(
    point: XYPosition,
    transform: Transform,
    snap_to_grid: bool,
    snap_grid: SnapGrid,
) -> XYPosition {
    let position = transform.invert(point);
    if snap_to_grid {
        snap_position(position, snap_grid)
    } else {
        position
    }
}

pub fn renderer_point_to_point(point: XYPosition, transform: Transform) -> XYPosition {
    transform.apply(point)
}

fn parse_padding(padding: PaddingWithUnit, viewport: f64) -> f64 {
    match padding {
        PaddingWithUnit::Number(number) => ((viewport - viewport / (1.0 + number)) * 0.5).floor(),
        PaddingWithUnit::Px(pixels) => pixels.floor(),
        PaddingWithUnit::Percent(percent) => (viewport * percent * 0.01).floor(),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResolvedPadding {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
    pub x: f64,
    pub y: f64,
}

pub fn parse_paddings(padding: Padding, width: f64, height: f64) -> ResolvedPadding {
    let uniform = padding.top == padding.right
        && padding.right == padding.bottom
        && padding.bottom == padding.left
        && padding.x.is_none()
        && padding.y.is_none();

    if uniform {
        if let Some(value) = padding.top {
            let padding_y = parse_padding(value, height);
            let padding_x = parse_padding(value, width);
            return ResolvedPadding {
                top: padding_y,
                right: padding_x,
                bottom: padding_y,
                left: padding_x,
                x: padding_x * 2.0,
                y: padding_y * 2.0,
            };
        }
        return ResolvedPadding::default();
    }

    if padding.top.is_none()
        && padding.bottom.is_none()
        && let Some(value) = padding.x
    {
        let padding_x = parse_padding(value, width);
        return ResolvedPadding {
            top: 0.0,
            right: padding_x,
            bottom: 0.0,
            left: padding_x,
            x: padding_x * 2.0,
            y: 0.0,
        };
    }

    if padding.left.is_none()
        && padding.right.is_none()
        && let Some(value) = padding.y
    {
        let padding_y = parse_padding(value, height);
        return ResolvedPadding {
            top: padding_y,
            right: 0.0,
            bottom: padding_y,
            left: 0.0,
            x: 0.0,
            y: padding_y * 2.0,
        };
    }

    let zero = PaddingWithUnit::Number(0.0);
    let top = parse_padding(padding.top.or(padding.y).unwrap_or(zero), height);
    let bottom = parse_padding(padding.bottom.or(padding.y).unwrap_or(zero), height);
    let left = parse_padding(padding.left.or(padding.x).unwrap_or(zero), width);
    let right = parse_padding(padding.right.or(padding.x).unwrap_or(zero), width);

    ResolvedPadding {
        top,
        right,
        bottom,
        left,
        x: left + right,
        y: top + bottom,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct AppliedPaddings {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

fn calculate_applied_paddings(
    bounds: Rect,
    x: f64,
    y: f64,
    zoom: f64,
    width: f64,
    height: f64,
) -> AppliedPaddings {
    let transform = Transform(x, y, zoom);
    let top_left = transform.apply(bounds.origin());
    let bottom_right = transform.apply(XYPosition::new(bounds.max_x(), bounds.max_y()));

    AppliedPaddings {
        left: top_left.x.floor(),
        top: top_left.y.floor(),
        right: (width - bottom_right.x).floor(),
        bottom: (height - bottom_right.y).floor(),
    }
}

pub fn get_viewport_for_bounds(
    bounds: Rect,
    width: f64,
    height: f64,
    min_zoom: f64,
    max_zoom: f64,
    padding: Padding,
) -> Viewport {
    let p = parse_paddings(padding, width, height);
    let bounds_width = if bounds.width == 0.0 {
        1.0
    } else {
        bounds.width
    };
    let bounds_height = if bounds.height == 0.0 {
        1.0
    } else {
        bounds.height
    };

    let x_zoom = (width - p.x) / bounds_width;
    let y_zoom = (height - p.y) / bounds_height;
    let clamped_zoom = clamp(x_zoom.min(y_zoom), min_zoom, max_zoom);

    let bounds_center_x = bounds.x + bounds.width / 2.0;
    let bounds_center_y = bounds.y + bounds.height / 2.0;
    let x = width / 2.0 - bounds_center_x * clamped_zoom;
    let y = height / 2.0 - bounds_center_y * clamped_zoom;

    let applied = calculate_applied_paddings(bounds, x, y, clamped_zoom, width, height);

    let offset_left = (applied.left - p.left).min(0.0);
    let offset_top = (applied.top - p.top).min(0.0);
    let offset_right = (applied.right - p.right).min(0.0);
    let offset_bottom = (applied.bottom - p.bottom).min(0.0);

    Viewport {
        x: x - offset_left + offset_right,
        y: y - offset_top + offset_bottom,
        zoom: clamped_zoom,
    }
}

pub fn get_node_dimensions<S: Clone>(node: &Node<S>) -> Dimensions {
    node.size()
}

pub fn get_pointer_position(
    event_position: XYPosition,
    container_bounds: Rect,
    transform: Transform,
    snap_grid: SnapGrid,
    snap_to_grid: bool,
) -> XYPosition {
    let local = XYPosition::new(
        event_position.x - container_bounds.x,
        event_position.y - container_bounds.y,
    );
    let pointer = point_to_renderer_point(local, transform, false, snap_grid);
    if snap_to_grid {
        snap_position(pointer, snap_grid)
    } else {
        pointer
    }
}

pub fn are_sets_equal<T: Ord + Clone>(
    a: &std::collections::BTreeSet<T>,
    b: &std::collections::BTreeSet<T>,
) -> bool {
    a.len() == b.len() && a.iter().all(|item| b.contains(item))
}

pub fn evaluate_absolute_position<S: Clone>(
    position: XYPosition,
    dimensions: Dimensions,
    parent_id: &str,
    node_lookup: &NodeLookup<S>,
    node_origin: NodeOrigin,
) -> XYPosition {
    let mut absolute = position;
    if let Some(parent) = node_lookup.get(parent_id) {
        let origin = parent.origin.unwrap_or(node_origin);
        absolute.x += parent.internals.position_absolute.x - dimensions.width * origin[0];
        absolute.y += parent.internals.position_absolute.y - dimensions.height * origin[1];
    }
    absolute
}

pub fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}
