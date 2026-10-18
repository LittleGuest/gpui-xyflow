use crate::core::edge::{BezierPathOptions, EdgeGraphics, SmoothStepPathOptions, StepPathOptions};
use crate::core::geometry::{Axis, Position, XYPosition};

pub fn get_edge_center(
    source_x: f64,
    source_y: f64,
    target_x: f64,
    target_y: f64,
) -> (f64, f64, f64, f64) {
    let x_offset = (target_x - source_x).abs() / 2.0;
    let center_x = if target_x < source_x {
        target_x + x_offset
    } else {
        target_x - x_offset
    };

    let y_offset = (target_y - source_y).abs() / 2.0;
    let center_y = if target_y < source_y {
        target_y + y_offset
    } else {
        target_y - y_offset
    };

    (center_x, center_y, x_offset, y_offset)
}

pub fn get_bezier_edge_center(
    source_x: f64,
    source_y: f64,
    target_x: f64,
    target_y: f64,
    source_control_x: f64,
    source_control_y: f64,
    target_control_x: f64,
    target_control_y: f64,
) -> (f64, f64, f64, f64) {
    let center_x =
        source_x * 0.125 + source_control_x * 0.375 + target_control_x * 0.375 + target_x * 0.125;
    let center_y =
        source_y * 0.125 + source_control_y * 0.375 + target_control_y * 0.375 + target_y * 0.125;
    let offset_x = (center_x - source_x).abs();
    let offset_y = (center_y - source_y).abs();

    (center_x, center_y, offset_x, offset_y)
}

fn calculate_control_offset(distance: f64, curvature: f64) -> f64 {
    if distance >= 0.0 {
        0.5_f64 * distance
    } else {
        curvature * 25.0 * (-distance).sqrt()
    }
}

fn get_control_with_curvature(
    position: Position,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    curvature: f64,
) -> (f64, f64) {
    match position {
        Position::Left => (x1 - calculate_control_offset(x1 - x2, curvature), y1),
        Position::Right => (x1 + calculate_control_offset(x2 - x1, curvature), y1),
        Position::Top => (x1, y1 - calculate_control_offset(y1 - y2, curvature)),
        Position::Bottom => (x1, y1 + calculate_control_offset(y2 - y1, curvature)),
    }
}

pub fn get_bezier_path(
    source_x: f64,
    source_y: f64,
    source_position: Position,
    target_x: f64,
    target_y: f64,
    target_position: Position,
    options: BezierPathOptions,
) -> EdgeGraphics {
    let curvature = options.curvature;
    let (source_control_x, source_control_y) = get_control_with_curvature(
        source_position,
        source_x,
        source_y,
        target_x,
        target_y,
        curvature,
    );
    let (target_control_x, target_control_y) = get_control_with_curvature(
        target_position,
        target_x,
        target_y,
        source_x,
        source_y,
        curvature,
    );
    let (label_x, label_y, offset_x, offset_y) = get_bezier_edge_center(
        source_x,
        source_y,
        target_x,
        target_y,
        source_control_x,
        source_control_y,
        target_control_x,
        target_control_y,
    );

    EdgeGraphics::new(
        format!(
            "M{source_x},{source_y} C{source_control_x},{source_control_y} {target_control_x},{target_control_y} {target_x},{target_y}"
        ),
        label_x,
        label_y,
        offset_x,
        offset_y,
    )
}

pub fn get_simple_bezier_path(
    source_x: f64,
    source_y: f64,
    target_x: f64,
    target_y: f64,
    options: BezierPathOptions,
) -> EdgeGraphics {
    let curvature = options.curvature;
    let offset = 0.25 * (target_x - source_x).abs().max(1.0) * curvature.max(0.0);
    let source_control_x = source_x + if target_x >= source_x { offset } else { -offset };
    let target_control_x = target_x - if target_x >= source_x { offset } else { -offset };

    let (label_x, label_y, offset_x, offset_y) = get_bezier_edge_center(
        source_x,
        source_y,
        target_x,
        target_y,
        source_control_x,
        source_y,
        target_control_x,
        target_y,
    );

    EdgeGraphics::new(
        format!(
            "M{source_x},{source_y} C{source_control_x},{source_y} {target_control_x},{target_y} {target_x},{target_y}"
        ),
        label_x,
        label_y,
        offset_x,
        offset_y,
    )
}

pub fn get_straight_path(
    source_x: f64,
    source_y: f64,
    target_x: f64,
    target_y: f64,
) -> EdgeGraphics {
    let (label_x, label_y, offset_x, offset_y) =
        get_edge_center(source_x, source_y, target_x, target_y);

    EdgeGraphics::new(
        format!("M {source_x},{source_y}L {target_x},{target_y}"),
        label_x,
        label_y,
        offset_x,
        offset_y,
    )
}

fn get_direction(source: XYPosition, source_position: Position, target: XYPosition) -> XYPosition {
    match source_position.axis() {
        Axis::Horizontal => {
            if source.x < target.x {
                XYPosition::new(1.0, 0.0)
            } else {
                XYPosition::new(-1.0, 0.0)
            }
        }
        Axis::Vertical => {
            if source.y < target.y {
                XYPosition::new(0.0, 1.0)
            } else {
                XYPosition::new(0.0, -1.0)
            }
        }
    }
}

fn distance(a: XYPosition, b: XYPosition) -> f64 {
    ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
}

struct PointsResult {
    points: Vec<XYPosition>,
    center_x: f64,
    center_y: f64,
    default_offset_x: f64,
    default_offset_y: f64,
}

fn accessor(p: &XYPosition, axis: Axis) -> f64 {
    match axis {
        Axis::Horizontal => p.x,
        Axis::Vertical => p.y,
    }
}

fn set_accessor(p: &mut XYPosition, axis: Axis, value: f64) {
    match axis {
        Axis::Horizontal => p.x = value,
        Axis::Vertical => p.y = value,
    }
}

fn component(p: &XYPosition, axis: Axis) -> f64 {
    match axis {
        Axis::Horizontal => p.x,
        Axis::Vertical => p.y,
    }
}

fn get_points(
    source: XYPosition,
    source_position: Position,
    target: XYPosition,
    target_position: Position,
    center: Option<XYPosition>,
    offset: f64,
    step_position: f64,
) -> PointsResult {
    let source_dir = source_position.direction();
    let target_dir = target_position.direction();
    let source_gapped = XYPosition::new(
        source.x + source_dir.x * offset,
        source.y + source_dir.y * offset,
    );
    let target_gapped = XYPosition::new(
        target.x + target_dir.x * offset,
        target.y + target_dir.y * offset,
    );
    let dir = get_direction(source_gapped, source_position, target_gapped);
    let dir_accessor = if dir.x != 0.0 {
        Axis::Horizontal
    } else {
        Axis::Vertical
    };
    let curr_dir = accessor(&dir, dir_accessor);
    let source_dir_component = accessor(&source_dir, dir_accessor);
    let target_dir_component = accessor(&target_dir, dir_accessor);

    let mut points: Vec<XYPosition>;
    let center_x;
    let center_y;
    let mut source_gap_offset = XYPosition::ZERO;
    let mut target_gap_offset = XYPosition::ZERO;

    let (_, _, default_offset_x, default_offset_y) =
        get_edge_center(source.x, source.y, target.x, target.y);

    if source_dir_component * target_dir_component == -1.0 {
        let (cx, cy) = match center {
            Some(center) => (center.x, center.y),
            None => match dir_accessor {
                Axis::Horizontal => (
                    source_gapped.x + (target_gapped.x - source_gapped.x) * step_position,
                    (source_gapped.y + target_gapped.y) / 2.0,
                ),
                Axis::Vertical => (
                    (source_gapped.x + target_gapped.x) / 2.0,
                    source_gapped.y + (target_gapped.y - source_gapped.y) * step_position,
                ),
            },
        };
        center_x = cx;
        center_y = cy;

        let vertical_split = [
            XYPosition::new(center_x, source_gapped.y),
            XYPosition::new(center_x, target_gapped.y),
        ];
        let horizontal_split = [
            XYPosition::new(source_gapped.x, center_y),
            XYPosition::new(target_gapped.x, center_y),
        ];

        points = if source_dir_component == curr_dir {
            match dir_accessor {
                Axis::Horizontal => vertical_split.to_vec(),
                Axis::Vertical => horizontal_split.to_vec(),
            }
        } else {
            match dir_accessor {
                Axis::Horizontal => horizontal_split.to_vec(),
                Axis::Vertical => vertical_split.to_vec(),
            }
        };
    } else {
        let source_target = [XYPosition::new(source_gapped.x, target_gapped.y)];
        let target_source = [XYPosition::new(target_gapped.x, source_gapped.y)];

        points = match dir_accessor {
            Axis::Horizontal => {
                if source_dir.x == curr_dir {
                    target_source.to_vec()
                } else {
                    source_target.to_vec()
                }
            }
            Axis::Vertical => {
                if source_dir.y == curr_dir {
                    source_target.to_vec()
                } else {
                    target_source.to_vec()
                }
            }
        };

        if source_position == target_position {
            let diff = (accessor(&source, dir_accessor) - accessor(&target, dir_accessor)).abs();
            if diff <= offset {
                let gap_offset = (offset - 1.0).min(offset - diff);
                if source_dir_component == curr_dir {
                    let sign = if accessor(&source_gapped, dir_accessor)
                        > accessor(&source, dir_accessor)
                    {
                        -1.0
                    } else {
                        1.0
                    };
                    set_accessor(&mut source_gap_offset, dir_accessor, sign * gap_offset);
                } else {
                    let sign = if accessor(&target_gapped, dir_accessor)
                        > accessor(&target, dir_accessor)
                    {
                        -1.0
                    } else {
                        1.0
                    };
                    set_accessor(&mut target_gap_offset, dir_accessor, sign * gap_offset);
                }
            }
        }

        if source_position != target_position {
            let dir_accessor_opposite = dir_accessor.other();
            let is_same_dir = source_dir_component == component(&target_dir, dir_accessor_opposite);
            let source_gt_target_oppo =
                component(&source_gapped, dir_accessor_opposite)
                    > component(&target_gapped, dir_accessor_opposite);
            let source_lt_target_oppo =
                component(&source_gapped, dir_accessor_opposite)
                    < component(&target_gapped, dir_accessor_opposite);
            let flip_source_target = (source_dir_component == 1.0
                && ((!is_same_dir && source_gt_target_oppo)
                    || (is_same_dir && source_lt_target_oppo)))
                || (source_dir_component != 1.0
                    && ((!is_same_dir && source_lt_target_oppo)
                        || (is_same_dir && source_gt_target_oppo)));

            if flip_source_target {
                points = match dir_accessor {
                    Axis::Horizontal => source_target.to_vec(),
                    Axis::Vertical => target_source.to_vec(),
                };
            }
        }

        let source_gap_point = source_gapped.add(source_gap_offset);
        let target_gap_point = target_gapped.add(target_gap_offset);
        let max_x_distance = (source_gap_point.x - points[0].x)
            .abs()
            .max((target_gap_point.x - points[0].x).abs());
        let max_y_distance = (source_gap_point.y - points[0].y)
            .abs()
            .max((target_gap_point.y - points[0].y).abs());

        if max_x_distance >= max_y_distance {
            center_x = (source_gap_point.x + target_gap_point.x) / 2.0;
            center_y = points[0].y;
        } else {
            center_x = points[0].x;
            center_y = (source_gap_point.y + target_gap_point.y) / 2.0;
        }
    }

    let gapped_source = source_gapped.add(source_gap_offset);
    let gapped_target = target_gapped.add(target_gap_offset);
    let first = points[0];
    let last = *points.last().expect("points is never empty");

    let mut path_points = vec![source];
    if gapped_source.x != first.x || gapped_source.y != first.y {
        path_points.push(gapped_source);
    }
    path_points.extend(points);
    if gapped_target.x != last.x || gapped_target.y != last.y {
        path_points.push(gapped_target);
    }
    path_points.push(target);

    PointsResult {
        points: path_points,
        center_x,
        center_y,
        default_offset_x,
        default_offset_y,
    }
}

fn get_bend(a: XYPosition, b: XYPosition, c: XYPosition, size: f64) -> String {
    let bend_size = (distance(a, b) / 2.0)
        .min(distance(b, c) / 2.0)
        .min(size);
    let (x, y) = (b.x, b.y);

    if (a.x == x && x == c.x) || (a.y == y && y == c.y) {
        return format!("L{x} {y}");
    }

    if a.y == y {
        let x_dir = if a.x < c.x { -1.0 } else { 1.0 };
        let y_dir = if a.y < c.y { 1.0 } else { -1.0 };
        return format!(
            "L {},{}Q {},{} {},{}",
            x + bend_size * x_dir,
            y,
            x,
            y,
            x,
            y + bend_size * y_dir
        );
    }

    let x_dir = if a.x < c.x { 1.0 } else { -1.0 };
    let y_dir = if a.y < c.y { -1.0 } else { 1.0 };
    format!(
        "L {},{}Q {},{} {},{}",
        x,
        y + bend_size * y_dir,
        x,
        y,
        x + bend_size * x_dir,
        y
    )
}


pub fn get_smooth_step_path(
    source_x: f64,
    source_y: f64,
    source_position: Position,
    target_x: f64,
    target_y: f64,
    target_position: Position,
    options: SmoothStepPathOptions,
    center: Option<XYPosition>,
) -> EdgeGraphics {
    let result = get_points(
        XYPosition::new(source_x, source_y),
        source_position,
        XYPosition::new(target_x, target_y),
        target_position,
        center,
        options.offset,
        options.step_position,
    );

    let points = &result.points;
    let mut path = format!("M{} {}", points[0].x, points[0].y);

    for index in 1..points.len().saturating_sub(1) {
        path.push_str(&get_bend(
            points[index - 1],
            points[index],
            points[index + 1],
            options.border_radius,
        ));
    }

    let last = points[points.len() - 1];
    path.push_str(&format!("L{} {}", last.x, last.y));

    EdgeGraphics::new(
        path,
        result.center_x,
        result.center_y,
        result.default_offset_x,
        result.default_offset_y,
    )
}

pub fn get_step_path(
    source_x: f64,
    source_y: f64,
    source_position: Position,
    target_x: f64,
    target_y: f64,
    target_position: Position,
    options: StepPathOptions,
) -> EdgeGraphics {
    get_smooth_step_path(
        source_x,
        source_y,
        source_position,
        target_x,
        target_y,
        target_position,
        SmoothStepPathOptions {
            offset: options.offset,
            border_radius: 0.0,
            step_position: 0.5,
        },
        None,
    )
}

pub fn get_edge_path(
    edge_type: &str,
    source_x: f64,
    source_y: f64,
    source_position: Position,
    target_x: f64,
    target_y: f64,
    target_position: Position,
    path_options: Option<&crate::core::edge::PathOptions>,
) -> EdgeGraphics {
    match edge_type {
        "straight" => get_straight_path(source_x, source_y, target_x, target_y),
        "step" => {
            let options = match path_options {
                Some(crate::core::edge::PathOptions::Step(options)) => *options,
                _ => StepPathOptions::default(),
            };
            get_step_path(
                source_x,
                source_y,
                source_position,
                target_x,
                target_y,
                target_position,
                options,
            )
        }
        "smoothstep" => {
            let options = match path_options {
                Some(crate::core::edge::PathOptions::SmoothStep(options)) => *options,
                _ => SmoothStepPathOptions::default(),
            };
            get_smooth_step_path(
                source_x,
                source_y,
                source_position,
                target_x,
                target_y,
                target_position,
                options,
                None,
            )
        }
        "simplebezier" => {
            let options = match path_options {
                Some(crate::core::edge::PathOptions::Bezier(options)) => *options,
                _ => BezierPathOptions::default(),
            };
            get_simple_bezier_path(source_x, source_y, target_x, target_y, options)
        }
        _ => {
            let options = match path_options {
                Some(crate::core::edge::PathOptions::Bezier(options)) => *options,
                _ => BezierPathOptions::default(),
            };
            get_bezier_path(
                source_x,
                source_y,
                source_position,
                target_x,
                target_y,
                target_position,
                options,
            )
        }
    }
}
