use std::{collections::BTreeMap, sync::Arc};

use gpui_kit::{
    App, Bounds, Font, Pixels, Point, SharedString, TextAlign, TextRun, Window, fill, point, px,
};

use crate::{
    core::{
        edge::Edge,
        geometry::{Rect, Transform, XYPosition},
        node::Node,
    },
    ui::{
        background::{BackgroundPattern, BackgroundVariant},
        edge_renderer::{EdgeRenderList, EdgeShape, edge_stroke, edge_stroke_width},
        handle::handle_fill,
        marker::MarkerSpec,
        node_renderer::{NodeBox, NodeRenderList, node_border, node_fill, node_text_color},
        paint::{dashed_path, transformed_stroke_path},
        paths::parse_svg_path,
        store::FlowStore,
        theme::FlowTheme,
    },
};

/// Paints custom content on top of a node's default rendering.
///
/// The bounds are in window coordinates and describe the node's screen rect, so callers can
/// place child graphics relative to it.
pub type NodeContentRenderer =
    Arc<dyn Fn(&Node<String>, Bounds<Pixels>, &mut Window, &mut App) + Send + Sync>;

/// Paints custom content for an edge after the default path, markers and label.
pub type EdgeContentRenderer =
    Arc<dyn Fn(&Edge<String>, Bounds<Pixels>, Transform, &mut Window, &mut App) + Send + Sync>;

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
    pub node_content_renderer: Option<NodeContentRenderer>,
    pub edge_content_renderer: Option<EdgeContentRenderer>,
    pub node_sources: BTreeMap<String, Node<String>>,
    pub edge_sources: BTreeMap<String, Edge<String>>,
    pub reconnect_handles: Vec<(f64, f64)>,
    pub resize_controls: Vec<ResizeControlPaint>,
    pub focus_rect: Option<Rect>,
    pub hovered_edge: Option<String>,
    pub dash_offset: f32,
    /// 为 true 时跳过画布上的默认节点底、边框与标签，交给元素层组件绘制。
    pub skip_node_bodies: bool,
}

/// A resizer control to paint in the canvas, expressed in flow coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResizeControlPaint {
    pub rect: Rect,
    pub kind: crate::ui::resizer::ResizeHandleKind,
    pub background: Option<gpui_kit::Rgba>,
    pub border: Option<gpui_kit::Rgba>,
}

impl PaneRenderData {
    /// Installs custom paint hooks. Source elements are only cloned when a hook is present.
    pub fn attach_custom_renderers(
        &mut self,
        store: &FlowStore<String>,
        node_renderer: Option<NodeContentRenderer>,
        edge_renderer: Option<EdgeContentRenderer>,
    ) {
        self.node_content_renderer = node_renderer;
        self.edge_content_renderer = edge_renderer;
        self.node_sources.clear();
        self.edge_sources.clear();
        if self.node_content_renderer.is_some() {
            self.node_sources = store
                .node_lookup()
                .values()
                .filter(|node| !node.hidden)
                .map(|node| (node.id.clone(), node.internals.user_node.clone()))
                .collect();
        }
        if self.edge_content_renderer.is_some() {
            self.edge_sources = store
                .edges()
                .iter()
                .map(|edge| (edge.id.clone(), edge.clone()))
                .collect();
        }
    }
}

pub fn background_pattern(theme: &FlowTheme, variant: BackgroundVariant) -> BackgroundPattern {
    let mut pattern = BackgroundPattern::new(variant);
    if theme.is_dark {
        pattern = pattern.with_size(1.4);
    }
    pattern
}

fn rects_overlap(x: f64, y: f64, width: f64, height: f64, view: &Rect) -> bool {
    x < view.x + view.width && x + width > view.x && y < view.y + view.height && y + height > view.y
}

fn edge_touches_view<S: Clone + PartialEq + ToString>(
    store: &FlowStore<S>,
    edge: &Edge<S>,
    view: &Rect,
) -> bool {
    let node_visible = |id: &String| {
        store
            .node_lookup()
            .get(id)
            .map(|node| {
                let dimensions = node.size();
                rects_overlap(
                    node.internals.position_absolute.x,
                    node.internals.position_absolute.y,
                    dimensions.width,
                    dimensions.height,
                    view,
                )
            })
            .unwrap_or(false)
    };
    node_visible(&edge.source) || node_visible(&edge.target)
}

pub fn build_render_data<S: Clone + PartialEq + ToString>(
    store: &FlowStore<S>,
    bounds: Bounds<Pixels>,
) -> PaneRenderData {
    let transform = store.transform();
    let theme = store.props().theme.clone();
    let origin = store.props().node_origin;
    let zoom = transform.zoom();
    let cull = store.props().only_render_visible_elements;
    let view = Rect::new(
        -transform.x() / zoom,
        -transform.y() / zoom,
        f32::from(bounds.size.width) as f64 / zoom,
        f32::from(bounds.size.height) as f64 / zoom,
    );

    let mut nodes: Vec<NodeBox> = store
        .node_lookup()
        .values()
        .filter(|node| !node.hidden)
        .map(|node| NodeBox::from_internal(node, origin))
        .filter(|node| {
            !cull
                || rects_overlap(
                    node.rect.x,
                    node.rect.y,
                    node.rect.width,
                    node.rect.height,
                    &view,
                )
        })
        .collect();
    let elevate_on_select =
        store.props().elevate_nodes_on_select && !store.props().z_index_mode.is_manual();
    nodes.sort_by(|a, b| {
        let by_z = a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal);
        let ordering = if elevate_on_select {
            a.selected.cmp(&b.selected).then(by_z)
        } else {
            by_z
        };
        ordering.then_with(|| a.id.cmp(&b.id))
    });

    let mut edges: Vec<EdgeShape> = Vec::new();
    for edge in store.edges() {
        if edge.hidden {
            continue;
        }
        if cull && !edge_touches_view(store, edge, &view) {
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

    let mut reconnect_handles = Vec::new();
    if store.props().edges_reconnectable {
        for edge in store.edges() {
            if !edge.selected || edge.hidden {
                continue;
            }
            let Some(graphics) = store.edge_path_for(edge) else {
                continue;
            };
            let samples = parse_svg_path(&graphics.path).sample(2);
            if samples.len() >= 2 {
                let start = samples[0];
                let end = samples[samples.len() - 1];
                reconnect_handles.push((start.x, start.y));
                reconnect_handles.push((end.x, end.y));
            }
        }
    }

    PaneRenderData {
        bounds,
        nodes: NodeRenderList { nodes },
        edges: EdgeRenderList { edges },
        transform,
        theme: theme.clone(),
        pattern: background_pattern(&theme, BackgroundVariant::Dots),
        selection,
        connection_path,
        connection_valid,
        reconnect_handles,
        resize_controls: Vec::new(),
        focus_rect: None,
        hovered_edge: None,
        dash_offset: 0.0,
        skip_node_bodies: false,
        node_content_renderer: None,
        edge_content_renderer: None,
        node_sources: BTreeMap::new(),
        edge_sources: BTreeMap::new(),
    }
}

pub fn paint_background(window: &mut Window, data: &PaneRenderData) {
    let width = f32::from(data.bounds.size.width) as f64;
    let height = f32::from(data.bounds.size.height) as f64;
    let origin = data.bounds.origin;
    let color = data.pattern.background_color(&data.theme);
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

pub fn paint_edges(window: &mut Window, data: &PaneRenderData, cx: &mut App) {
    let origin = data.bounds.origin;
    let zoom = data.transform.zoom();
    for edge in &data.edges.edges {
        let svg = parse_svg_path(&edge.path);
        if svg.is_empty() {
            continue;
        }
        let animated = if edge.animated {
            let dash = (6.0 / zoom) as f32;
            let gap = (4.0 / zoom) as f32;
            let phase = data.dash_offset / zoom as f32;
            dashed_path(&svg, dash, gap, phase)
        } else {
            svg.clone()
        };
        let hovered = data.hovered_edge.as_deref() == Some(edge.id.as_str());
        let width = if hovered {
            edge_stroke_width(edge, zoom) + 1.0
        } else {
            edge_stroke_width(edge, zoom)
        };
        let color = edge_stroke(&data.theme, edge.selected);
        if let Some(path) = transformed_stroke_path(&animated, data.transform, width) {
            let translated = translate_path(path, origin);
            window.paint_path(translated, color);
        }
        paint_edge_markers(window, data, edge, color.into());
        if let Some(label) = &edge.label {
            paint_edge_label(window, data, edge, label, cx);
        }
        if let Some(renderer) = &data.edge_content_renderer
            && let Some(source) = data.edge_sources.get(&edge.id)
        {
            renderer(source, data.bounds, data.transform, window, cx);
        }
    }
}

fn paint_edge_label(
    window: &mut Window,
    data: &PaneRenderData,
    edge: &EdgeShape,
    label: &str,
    cx: &mut App,
) {
    if label.is_empty() {
        return;
    }
    let screen = data.transform.apply(edge.label_position());
    let font_size = data.theme.edge_label_font_size.max(1.0);
    let line_height = font_size * 1.2;
    let run = TextRun {
        len: label.len(),
        font: Font::default(),
        color: data.theme.edge_label_color.into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped =
        window
            .text_system()
            .shape_line(SharedString::from(label), px(font_size), &[run], None);
    let width = f32::from(shaped.width());
    let origin = data.bounds.origin;
    let text_origin = point(
        origin.x + px(screen.x as f32 - width / 2.0),
        origin.y + px(screen.y as f32 - line_height / 2.0),
    );
    let _ = shaped.paint(
        text_origin,
        px(line_height),
        TextAlign::Left,
        None,
        window,
        cx,
    );
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
    let scale = 0.625 * data.theme.edge_stroke_width as f64;

    if let Some(spec) = &edge.markers.end {
        let last = samples[samples.len() - 1];
        let prev = samples[samples.len() - 2];
        let direction = (last.x - prev.x, last.y - prev.y);
        let points = marker_polygon_points(last, direction, scale);
        paint_marker_polygon(window, data, &points, spec, color);
    }

    if let Some(spec) = &edge.markers.start {
        let first = samples[0];
        let second = samples[1];
        let direction = (first.x - second.x, first.y - second.y);
        let points = marker_polygon_points(first, direction, scale);
        paint_marker_polygon(window, data, &points, spec, color);
    }
}

fn marker_polygon_points(
    tip: crate::ui::paths::SvgPoint,
    direction: (f64, f64),
    scale: f64,
) -> Vec<(f64, f64)> {
    use crate::core::geometry::XYPosition;

    let len = (direction.0 * direction.0 + direction.1 * direction.1).sqrt();
    if len == 0.0 {
        return Vec::new();
    }
    let d = (direction.0 / len, direction.1 / len);
    let perp = (-d.1, d.0);
    [(-5.0, -4.0), (0.0, 0.0), (-5.0, 4.0)]
        .iter()
        .map(|(lx, ly)| {
            let u = lx * scale;
            let v = ly * scale;
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
        if !data.skip_node_bodies {
            window.paint_quad(
                fill(bounds, fill_color)
                    .corner_radii(px(data.theme.node_border_radius))
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
        }

        if !data.skip_node_bodies && !node.variant.is_group() {
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
        if !data.skip_node_bodies
            && let Some(renderer) = &data.node_content_renderer
            && let Some(source) = data.node_sources.get(&node.id)
        {
            renderer(source, bounds, window, cx);
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
    window.paint_quad(
        fill(bounds, data.theme.selection_background)
            .border_widths(px(1.0))
            .border_color(data.theme.selection_border),
    );
}

pub fn paint_reconnect_handles(window: &mut Window, data: &PaneRenderData) {
    if data.reconnect_handles.is_empty() {
        return;
    }
    let origin = data.bounds.origin;
    let radius = 4.0;
    for (x, y) in &data.reconnect_handles {
        let screen = data.transform.apply(XYPosition::new(*x, *y));
        let center = point(
            origin.x + px(screen.x as f32),
            origin.y + px(screen.y as f32),
        );

        let mut fill_builder = gpui_kit::PathBuilder::fill();
        circle_path(&mut fill_builder, center, radius);
        if let Ok(path) = fill_builder.build() {
            window.paint_path(path, data.theme.controls_background);
        }

        let mut stroke_builder = gpui_kit::PathBuilder::stroke(px(1.5));
        circle_path(&mut stroke_builder, center, radius);
        if let Ok(path) = stroke_builder.build() {
            window.paint_path(path, data.theme.controls_border);
        }
    }
}

pub fn paint_resize_controls(window: &mut Window, data: &PaneRenderData) {
    if data.resize_controls.is_empty() {
        return;
    }
    let origin = data.bounds.origin;
    for control in &data.resize_controls {
        if control.background.is_none() && control.border.is_none() {
            continue;
        }
        let top_left = data
            .transform
            .apply(XYPosition::new(control.rect.x, control.rect.y));
        let bottom_right = data.transform.apply(XYPosition::new(
            control.rect.x + control.rect.width,
            control.rect.y + control.rect.height,
        ));
        let bounds = Bounds {
            origin: point(
                origin.x + px(top_left.x as f32),
                origin.y + px(top_left.y as f32),
            ),
            size: gpui_kit::size(
                px((bottom_right.x - top_left.x).max(0.0) as f32),
                px((bottom_right.y - top_left.y).max(0.0) as f32),
            ),
        };
        let background = control.background.unwrap_or(gpui_kit::Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        });
        let mut quad = fill(bounds, background);
        if let Some(border) = control.border {
            quad = quad.border_widths(px(1.0)).border_color(border);
        }
        window.paint_quad(quad);
    }
}

pub fn paint_focus_ring(window: &mut Window, data: &PaneRenderData) {
    let Some(rect) = data.focus_rect else {
        return;
    };
    let origin = data.bounds.origin;
    let top_left = data.transform.apply(XYPosition::new(rect.x, rect.y));
    let bottom_right = data
        .transform
        .apply(XYPosition::new(rect.x + rect.width, rect.y + rect.height));
    let bounds = Bounds {
        origin: point(
            origin.x + px(top_left.x as f32),
            origin.y + px(top_left.y as f32),
        ),
        size: gpui_kit::size(
            px((bottom_right.x - top_left.x).max(0.0) as f32),
            px((bottom_right.y - top_left.y).max(0.0) as f32),
        ),
    };
    window.paint_quad(
        fill(
            bounds,
            gpui_kit::Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        )
        .border_widths(px(2.0))
        .border_color(data.theme.selection_border),
    );
}

fn circle_path(builder: &mut gpui_kit::PathBuilder, center: Point<Pixels>, radius: f32) {
    let left = point(center.x - px(radius), center.y);
    let right = point(center.x + px(radius), center.y);
    let top = point(center.x, center.y - px(radius));
    let bottom = point(center.x, center.y + px(radius));
    let control = radius * 0.552_284_8;

    builder.move_to(left);
    builder.curve_to(top, point(left.x, center.y - px(control)));
    builder.curve_to(right, point(center.x + px(control), top.y));
    builder.curve_to(bottom, point(right.x, center.y + px(control)));
    builder.curve_to(left, point(center.x - px(control), bottom.y));
    builder.close();
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
