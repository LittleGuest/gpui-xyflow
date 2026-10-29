use gpui_kit::{Path, PathBuilder, Pixels, Point, point, px};

use crate::ui::paths::{PathCommand, SvgPath, SvgPoint};

pub fn to_point(point_value: SvgPoint) -> Point<Pixels> {
    point(px(point_value.x as f32), px(point_value.y as f32))
}

pub fn build_stroke_path(path: &SvgPath, width: f32) -> Option<Path<Pixels>> {
    if path.is_empty() {
        return None;
    }
    let mut builder = PathBuilder::stroke(px(width));
    for command in &path.commands {
        match command {
            PathCommand::MoveTo(to) => builder.move_to(to_point(*to)),
            PathCommand::LineTo(to) => builder.line_to(to_point(*to)),
            PathCommand::QuadTo(ctrl, to) => builder.curve_to(to_point(*to), to_point(*ctrl)),
            PathCommand::CubicTo(ctrl_a, ctrl_b, to) => {
                builder.cubic_bezier_to(to_point(*to), to_point(*ctrl_a), to_point(*ctrl_b))
            }
            PathCommand::Close => builder.close(),
        }
    }
    builder.build().ok()
}

pub fn build_fill_path(path: &SvgPath) -> Option<Path<Pixels>> {
    if path.is_empty() {
        return None;
    }
    let mut builder = PathBuilder::fill();
    for command in &path.commands {
        match command {
            PathCommand::MoveTo(to) => builder.move_to(to_point(*to)),
            PathCommand::LineTo(to) => builder.line_to(to_point(*to)),
            PathCommand::QuadTo(ctrl, to) => builder.curve_to(to_point(*to), to_point(*ctrl)),
            PathCommand::CubicTo(ctrl_a, ctrl_b, to) => {
                builder.cubic_bezier_to(to_point(*to), to_point(*ctrl_a), to_point(*ctrl_b))
            }
            PathCommand::Close => builder.close(),
        }
    }
    builder.build().ok()
}

pub fn build_stroke_path_from_str(path: &str, width: f32) -> Option<Path<Pixels>> {
    build_stroke_path(&crate::ui::paths::parse_svg_path(path), width)
}

pub fn build_fill_path_from_str(path: &str) -> Option<Path<Pixels>> {
    build_fill_path(&crate::ui::paths::parse_svg_path(path))
}

pub fn transform_points(path: &SvgPath, transform: crate::core::geometry::Transform) -> SvgPath {
    let zoom = transform.zoom();
    let mapped: Vec<PathCommand> = path
        .commands
        .iter()
        .map(|command| match command {
            PathCommand::MoveTo(to) => PathCommand::MoveTo(map_point(*to, transform, zoom)),
            PathCommand::LineTo(to) => PathCommand::LineTo(map_point(*to, transform, zoom)),
            PathCommand::QuadTo(ctrl, to) => PathCommand::QuadTo(
                map_point(*ctrl, transform, zoom),
                map_point(*to, transform, zoom),
            ),
            PathCommand::CubicTo(ctrl_a, ctrl_b, to) => PathCommand::CubicTo(
                map_point(*ctrl_a, transform, zoom),
                map_point(*ctrl_b, transform, zoom),
                map_point(*to, transform, zoom),
            ),
            PathCommand::Close => PathCommand::Close,
        })
        .collect();
    SvgPath::new(mapped)
}

fn map_point(value: SvgPoint, transform: crate::core::geometry::Transform, zoom: f64) -> SvgPoint {
    SvgPoint::new(
        value.x * zoom + transform.x(),
        value.y * zoom + transform.y(),
    )
}

pub fn transformed_stroke_path(
    path: &SvgPath,
    transform: crate::core::geometry::Transform,
    width: f32,
) -> Option<Path<Pixels>> {
    let scaled = transform_points(path, transform);
    let stroke_width = (width as f64 * transform.zoom()).max(0.5) as f32;
    build_stroke_path(&scaled, stroke_width)
}

/// Splits a path into dashes, offsetting the dash pattern by `phase`.
pub fn dashed_path(path: &SvgPath, dash: f32, gap: f32, phase: f32) -> SvgPath {
    let period = dash + gap;
    if dash <= 0.0 || period <= 0.0 {
        return path.clone();
    }
    let phase = phase.rem_euclid(period);
    let mut polylines: Vec<Vec<SvgPoint>> = Vec::new();
    let mut current: Vec<SvgPoint> = Vec::new();
    let mut cursor = SvgPoint::new(0.0, 0.0);
    for command in &path.commands {
        match command {
            PathCommand::MoveTo(to) => {
                if current.len() > 1 {
                    polylines.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
                current.push(*to);
                cursor = *to;
            }
            PathCommand::LineTo(to) => {
                if current.is_empty() {
                    current.push(cursor);
                }
                current.push(*to);
                cursor = *to;
            }
            PathCommand::QuadTo(ctrl, to) => {
                if current.is_empty() {
                    current.push(cursor);
                }
                flatten_quadratic(cursor, *ctrl, *to, &mut current);
                cursor = *to;
            }
            PathCommand::CubicTo(ctrl_a, ctrl_b, to) => {
                if current.is_empty() {
                    current.push(cursor);
                }
                flatten_cubic(cursor, *ctrl_a, *ctrl_b, *to, &mut current);
                cursor = *to;
            }
            PathCommand::Close => {
                if current.len() > 1 {
                    let first = current[0];
                    current.push(first);
                    polylines.push(std::mem::take(&mut current));
                    cursor = first;
                } else {
                    current.clear();
                }
            }
        }
    }
    if current.len() > 1 {
        polylines.push(current);
    }

    let mut commands = Vec::new();
    let mut open = false;
    for points in polylines {
        let mut traveled = 0.0;
        for pair in points.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            let length = distance(from, to);
            if length <= f32::EPSILON as f64 {
                continue;
            }
            let mut position = 0.0;
            while position < length - 1e-6 {
                let shifted = (traveled + position) as f32 + phase;
                let local = shifted.rem_euclid(period);
                let remaining = if local < dash {
                    dash - local
                } else {
                    period - local
                };
                let next = (position + remaining.max(1e-4) as f64).min(length);
                if local < dash {
                    let start = lerp(from, to, (position / length) as f32);
                    let end = lerp(from, to, (next / length) as f32);
                    if !open {
                        commands.push(PathCommand::MoveTo(start));
                        open = true;
                    }
                    commands.push(PathCommand::LineTo(end));
                } else {
                    open = false;
                }
                position = next;
            }
            traveled += length;
        }
        open = false;
    }
    SvgPath::new(commands)
}

fn distance(from: SvgPoint, to: SvgPoint) -> f64 {
    ((to.x - from.x).powi(2) + (to.y - from.y).powi(2)).sqrt()
}

fn lerp(from: SvgPoint, to: SvgPoint, t: f32) -> SvgPoint {
    let t = t.clamp(0.0, 1.0) as f64;
    SvgPoint::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t)
}

fn flatten_quadratic(from: SvgPoint, ctrl: SvgPoint, to: SvgPoint, out: &mut Vec<SvgPoint>) {
    let steps = 12;
    for step in 1..=steps {
        let t = step as f32 / steps as f32;
        let one = 1.0 - t;
        let x = one * one * from.x as f32 + 2.0 * one * t * ctrl.x as f32 + t * t * to.x as f32;
        let y = one * one * from.y as f32 + 2.0 * one * t * ctrl.y as f32 + t * t * to.y as f32;
        out.push(SvgPoint::new(x as f64, y as f64));
    }
}

fn flatten_cubic(
    from: SvgPoint,
    ctrl_a: SvgPoint,
    ctrl_b: SvgPoint,
    to: SvgPoint,
    out: &mut Vec<SvgPoint>,
) {
    let steps = 20;
    for step in 1..=steps {
        let t = step as f32 / steps as f32;
        let one = 1.0 - t;
        let x = one * one * one * from.x as f32
            + 3.0 * one * one * t * ctrl_a.x as f32
            + 3.0 * one * t * t * ctrl_b.x as f32
            + t * t * t * to.x as f32;
        let y = one * one * one * from.y as f32
            + 3.0 * one * one * t * ctrl_a.y as f32
            + 3.0 * one * t * t * ctrl_b.y as f32
            + t * t * t * to.y as f32;
        out.push(SvgPoint::new(x as f64, y as f64));
    }
}
