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
