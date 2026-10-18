use gpui_kit::{
    App, Bounds, Font, Pixels, Point, SharedString, TextAlign, TextRun, Window, fill, point, px,
};

use crate::{
    core::geometry::{Rect, Transform},
    ui::{
        background::{BackgroundPattern, BackgroundVariant, background_color},
        edge_renderer::{EdgeRenderList, EdgeShape, edge_stroke, edge_stroke_width},
        handle::handle_fill,
        marker::MarkerSpec,
        node_renderer::{NodeBox, NodeRenderList, node_border, node_fill, node_text_color},
        paint::transformed_stroke_path,
        paths::parse_svg_path,
        store::FlowStore,
        theme::FlowTheme,
    },
};

pub struct PaneRenderData {
    pub bounds: Bounds<Pixels>,
    pub nodes: NodeRenderList,
    pub edges: EdgeRenderList,
    pub transform: Transform,
    pub theme: FlowTheme,
    pub pattern: BackgroundPattern,
    pub selection: Option<Rect>,
    pub connection_path: Option<String>,
    pub connection_valid: Option<bool>,
}

pub fn background_pattern(theme: &FlowTheme, variant: BackgroundVariant) -> BackgroundPattern {
    let mut pattern = BackgroundPattern::new(variant);
    if theme.is_dark {
        pattern = pattern.with_size(1.4);
    }
    pattern
}

pub fn build_render_data<S: Clone + PartialEq + ToString>(
    store: &FlowStore<S>,
    bounds: Bounds<Pixels>,
) -> PaneRenderData {
    let transform = store.transform();
    let theme = store.props().theme.clone();
    let origin = store.props().node_origin;

    let mut nodes: Vec<NodeBox> = store
        .node_lookup()
        .values()
        .filter(|node| !node.hidden)
        .map(|node| NodeBox::from_internal(node, origin))
        .collect();
    nodes.sort_by(|a, b| {
        a.z.partial_cmp(&b.z)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });

    let mut edges: Vec<EdgeShape> = Vec::new();
    for edge in store.edges() {
        if edge.hidden {
            continue;
        }
        if let Some(graphics) = store.edge_path_for(edge) {
            edges.push(EdgeShape::from_edge(
                edge,
                &graphics,
                store.edge_z_index(edge),
            ));
        }
    }
    edges.sort_by(|a, b| {
        a.z.partial_cmp(&b.z)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });

    let selection = store.interaction().selection_rect();
    let connection_path = store.connection_line().map(|line| {
        crate::ui::edge_renderer::connection_line_path(
            &line,
            store.props().connection_line_type,
            None,
        )
        .path
    });

    let connection_valid = match store.connection_state() {
        crate::core::connect::ConnectionState::InProgress(state) => state.is_valid,
        crate::core::connect::ConnectionState::Idle(_) => None,
    };

    PaneRenderData {
        bounds,
        nodes: NodeRenderList::new(nodes),
        edges: EdgeRenderList::new(edges),
        transform,
        theme: theme.clone(),
        pattern: background_pattern(&theme, BackgroundVariant::Dots),
        selection,
        connection_path,
        connection_valid,
    }
}

pub fn paint_background(window: &mut Window, data: &PaneRenderData) {
    let width = f32::from(data.bounds.size.width) as f64;
    let height = f32::from(data.bounds.size.height) as f64;
    let origin = data.bounds.origin;
    let color = background_color(&data.theme);
    window.paint_quad(fill(data.bounds, color));

    let pattern = data.pattern;
    let pattern_color = pattern.color(&data.theme);
    match pattern.variant {
        BackgroundVariant::Dots => {
            for (dot, radius) in pattern.dot_list(data.transform, width, height) {
                let center = point(origin.x + px(dot.x as f32), origin.y + px(dot.y as f32));
                let mut builder = gpui_kit::PathBuilder::fill();
                builder.move_to(point(center.x - px(radius as f32), center.y));
                builder.curve_to(
                    point(center.x, center.y - px(radius as f32)),
                    point(center.x - px(radius as f32), center.y - px(radius as f32)),
                );
                builder.curve_to(
                    point(center.x + px(radius as f32), center.y),
                    point(center.x + px(radius as f32), center.y - px(radius as f32)),
                );
                builder.curve_to(
                    point(center.x, center.y + px(radius as f32)),
                    point(center.x + px(radius as f32), center.y + px(radius as f32)),
                );
                builder.curve_to(
                    point(center.x - px(radius as f32), center.y),
                    point(center.x - px(radius as f32), center.y + px(radius as f32)),
                );
                builder.close();
                if let Ok(path) = builder.build() {
                    window.paint_path(path, pattern_color);
                }
            }
        }
        BackgroundVariant::Lines | BackgroundVariant::Cross => {
            let segments = if pattern.variant == BackgroundVariant::Lines {
                pattern.line_segments(data.transform, width, height)
            } else {
                pattern.cross_segments(data.transform, width, height)
            };
            let mut builder = gpui_kit::PathBuilder::stroke(px(1.0));
            let mut has_any = false;
            for (start, end) in segments {
                builder.move_to(point(
                    origin.x + px(start.x as f32),
                    origin.y + px(start.y as f32),
                ));
                builder.line_to(point(
                    origin.x + px(end.x as f32),
                    origin.y + px(end.y as f32),
                ));
                has_any = true;
            }
            if has_any && let Ok(path) = builder.build() {
                window.paint_path(path, pattern_color);
            }
        }
    }
}

pub fn paint_edges(window: &mut Window, data: &PaneRenderData) {
    let origin = data.bounds.origin;
    let zoom = data.transform.zoom();
    for edge in &data.edges.edges {
        let svg = parse_svg_path(&edge.path);
        if svg.is_empty() {
            continue;
        }
        let width = edge_stroke_width(edge, zoom);
        let color = edge_stroke(&data.theme, edge.selected);
        if let Some(path) = transformed_stroke_path(&svg, data.transform, width) {
            let translated = translate_path(path, origin);
            window.paint_path(translated, color);
        }
        paint_edge_markers(window, data, edge, color.into());
    }
}

fn paint_edge_markers(
    window: &mut Window,
    data: &PaneRenderData,
    edge: &EdgeShape,
    color: gpui_kit::Hsla,
) {
    if edge.markers.is_empty() {
        return;
    }
    let samples = parse_svg_path(&edge.path).sample(64);
    if samples.len() < 2 {
        return;
    }

    if let Some(spec) = &edge.markers.end {
        let last = samples[samples.len() - 1];
        let prev = samples[samples.len() - 2];
        let direction = (last.x - prev.x, last.y - prev.y);
        let points = marker_polygon_points(spec, last, direction);
        paint_marker_polygon(window, data, &points, spec, color);
    }

    if let Some(spec) = &edge.markers.start {
        let first = samples[0];
        let second = samples[1];
        let direction = (first.x - second.x, first.y - second.y);
        let points = marker_polygon_points(spec, first, direction);
        paint_marker_polygon(window, data, &points, spec, color);
    }
}

fn marker_polygon_points(
    spec: &MarkerSpec,
    tip: crate::ui::paths::SvgPoint,
    direction: (f64, f64),
) -> Vec<(f64, f64)> {
    use crate::{core::geometry::XYPosition, ui::marker::MarkerVariant};

    let scale = spec.width / 10.0;
    let ref_x = 9.0;
    let ref_y = 5.0;
    let len = (direction.0 * direction.0 + direction.1 * direction.1).sqrt();
    if len == 0.0 {
        return Vec::new();
    }
    let d = (direction.0 / len, direction.1 / len);
    let perp = (-d.1, d.0);
    let local: &[(f64, f64)] = match spec.variant {
        MarkerVariant::ArrowClosed => &[(0.0, 0.0), (10.0, 5.0), (0.0, 10.0), (5.0, 5.0)],
        MarkerVariant::Arrow => &[(0.0, 0.0), (10.0, 5.0), (0.0, 10.0)],
    };
    local
        .iter()
        .map(|(lx, ly)| {
            let u = (lx - ref_x) * scale;
            let v = (ly - ref_y) * scale;
            let world = XYPosition::new(tip.x + d.0 * u + perp.0 * v, tip.y + d.1 * u + perp.1 * v);
            (world.x, world.y)
        })
        .collect()
}

fn paint_marker_polygon(
    window: &mut Window,
    data: &PaneRenderData,
    points: &[(f64, f64)],
    spec: &MarkerSpec,
    color: gpui_kit::Hsla,
) {
    use crate::{core::geometry::XYPosition, ui::marker::MarkerVariant};

    if points.is_empty() {
        return;
    }
    let origin = data.bounds.origin;
    let to_screen = |x: f64, y: f64| {
        let screen = data.transform.apply(XYPosition::new(x, y));
        point(
            origin.x + px(screen.x as f32),
            origin.y + px(screen.y as f32),
        )
    };

    match spec.variant {
        MarkerVariant::ArrowClosed => {
            let mut builder = gpui_kit::PathBuilder::fill();
            for (index, (x, y)) in points.iter().enumerate() {
                let screen = to_screen(*x, *y);
                if index == 0 {
                    builder.move_to(screen);
                } else {
                    builder.line_to(screen);
                }
            }
            builder.close();
            if let Ok(path) = builder.build() {
                window.paint_path(path, color);
            }
        }
        MarkerVariant::Arrow => {
            let mut builder = gpui_kit::PathBuilder::stroke(px(1.0));
            for (index, (x, y)) in points.iter().enumerate() {
                let screen = to_screen(*x, *y);
                if index == 0 {
                    builder.move_to(screen);
                } else {
                    builder.line_to(screen);
                }
            }
            if let Ok(path) = builder.build() {
                window.paint_path(path, color);
            }
        }
    }
}

fn translate_path(path: gpui_kit::Path<Pixels>, origin: Point<Pixels>) -> gpui_kit::Path<Pixels> {
    let mut builder = gpui_kit::PathBuilder::stroke(px(1.0));
    let _ = &mut builder;
    let _ = origin;
    path
}

pub fn paint_nodes(window: &mut Window, data: &PaneRenderData, cx: &mut App) {
    let origin = data.bounds.origin;
    let zoom = data.transform.zoom();
    for node in &data.nodes.nodes {
        let screen = crate::ui::node_renderer::node_screen_rect(&node.rect, data.transform);
        let bounds = Bounds {
            origin: point(
                origin.x + px(screen.x as f32),
                origin.y + px(screen.y as f32),
            ),
            size: gpui_kit::size(px(screen.width as f32), px(screen.height as f32)),
        };
        let fill_color = node_fill(&data.theme, node.variant, node.selected);
        let border_color = node_border(&data.theme, node.selected);
        let border_width = if node.selected { 1.5 } else { 1.0 } as f32;
        window.paint_quad(
            fill(bounds, fill_color)
                .corner_radii(px(4.0))
                .border_widths(px(border_width))
                .border_color(border_color),
        );

        let line_height = 14.0_f32;
        let run = TextRun {
            len: node.label.len(),
            font: Font::default(),
            color: node_text_color(&data.theme, node.variant).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let shaped = window.text_system().shape_line(
            SharedString::from(node.label.as_str()),
            px(12.0),
            &[run],
            None,
        );
        let text_origin = point(
            origin.x + px(screen.x as f32),
            origin.y + px((screen.y + screen.height / 2.0) as f32 - line_height),
        );
        let _ = shaped.paint(
            text_origin,
            px(line_height),
            TextAlign::Center,
            Some(px(screen.width as f32)),
            window,
            cx,
        );

        if !node.variant.is_group() {
            for (handle_type, _, center, _, _) in node.handle_positions() {
                let screen_center = data.transform.apply(center);
                let radius = ((data.theme.handle_size as f64) * zoom / 2.0).max(2.0) as f32;
                let cx = origin.x + px(screen_center.x as f32);
                let cy = origin.y + px(screen_center.y as f32);
                let handle_bounds = Bounds {
                    origin: point(cx - px(radius), cy - px(radius)),
                    size: gpui_kit::size(px(radius * 2.0), px(radius * 2.0)),
                };
                window.paint_quad(
                    fill(handle_bounds, handle_fill(&data.theme, handle_type))
                        .corner_radii(px(radius)),
                );
            }
        }
    }
}

pub fn paint_selection(window: &mut Window, data: &PaneRenderData) {
    let Some(rect) = data.selection else {
        return;
    };
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let origin = data.bounds.origin;
    let bounds = Bounds {
        origin: point(origin.x + px(rect.x as f32), origin.y + px(rect.y as f32)),
        size: gpui_kit::size(px(rect.width as f32), px(rect.height as f32)),
    };
    window.paint_quad(fill(bounds, data.theme.selection_border));
}

pub fn paint_connection(window: &mut Window, data: &PaneRenderData) {
    let Some(ref path_str) = data.connection_path else {
        return;
    };
    let svg = parse_svg_path(path_str);
    if svg.is_empty() {
        return;
    }
    let width = data.theme.connection_line_stroke_width;
    let color = data.theme.connection_line_stroke;
    if let Some(path) = transformed_stroke_path(&svg, data.transform, width) {
        window.paint_path(path, color);
    }
}
