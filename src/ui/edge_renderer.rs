use crate::{
    core::{
        edge::{
            ConnectionLineType, DEFAULT_EDGE_STROKE_WIDTH, DEFAULT_INTERACTION_WIDTH, Edge,
            EdgeGraphics,
        },
        edge_path::get_edge_path,
        geometry::{EdgePosition, XYPosition},
    },
    ui::{marker::MarkerSet, paths::parse_svg_path, store::FlowStore, theme::FlowTheme},
};

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeShape {
    pub id: String,
    pub path: String,
    pub label_x: f64,
    pub label_y: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub z: f64,
    pub selected: bool,
    pub animated: bool,
    pub label: Option<String>,
    pub stroke_width: f32,
    pub interaction_width: f32,
    pub markers: MarkerSet,
}

impl EdgeShape {
    pub fn from_edge<S: Clone>(edge: &Edge<S>, graphics: &EdgeGraphics, z: f64) -> Self {
        EdgeShape {
            id: edge.id.clone(),
            path: graphics.path.clone(),
            label_x: graphics.label_x,
            label_y: graphics.label_y,
            offset_x: graphics.offset_x,
            offset_y: graphics.offset_y,
            z,
            selected: edge.selected,
            animated: edge.animated,
            label: edge.label.clone(),
            stroke_width: DEFAULT_EDGE_STROKE_WIDTH as f32,
            interaction_width: edge.interaction_width.unwrap_or(DEFAULT_INTERACTION_WIDTH) as f32,
            markers: MarkerSet::from_markers(edge.marker_start.as_ref(), edge.marker_end.as_ref()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    pub fn center(&self) -> XYPosition {
        XYPosition::new(self.label_x, self.label_y)
    }

    pub fn label_position(&self) -> XYPosition {
        XYPosition::new(self.label_x + self.offset_x, self.label_y + self.offset_y)
    }
}

pub fn edge_shape_for<S: Clone>(edge: &Edge<S>, position: &EdgePosition, z: f64) -> EdgeShape {
    let edge_type = edge
        .edge_type
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let graphics = get_edge_path(
        &edge_type,
        position.source_x,
        position.source_y,
        position.source_position,
        position.target_x,
        position.target_y,
        position.target_position,
        edge.path_options.as_ref(),
    );
    EdgeShape::from_edge(edge, &graphics, z)
}

pub fn connection_line_path(
    line: &crate::core::connect::ConnectionLine,
    line_type: ConnectionLineType,
    path_options: Option<&crate::core::edge::PathOptions>,
) -> EdgeGraphics {
    let edge_type = match line_type {
        ConnectionLineType::Bezier => "default",
        ConnectionLineType::Straight => "straight",
        ConnectionLineType::Step => "step",
        ConnectionLineType::SmoothStep => "smoothstep",
        ConnectionLineType::SimpleBezier => "simplebezier",
    };
    get_edge_path(
        edge_type,
        line.source_x,
        line.source_y,
        line.source_position,
        line.target_x,
        line.target_y,
        line.target_position,
        path_options,
    )
}

pub fn edge_stroke(theme: &FlowTheme, selected: bool) -> gpui_kit::Rgba {
    theme.edge_stroke_for(selected)
}

pub fn edge_stroke_width(edge: &EdgeShape, zoom: f64) -> f32 {
    let scaled = edge.stroke_width as f64 * zoom.max(0.1);
    let minimum = if edge.selected { 1.5 } else { 1.0 };
    scaled.max(minimum) as f32
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EdgeRenderList {
    pub edges: Vec<EdgeShape>,
}

impl EdgeRenderList {
    pub fn new(mut edges: Vec<EdgeShape>) -> Self {
        edges.sort_by(|a, b| {
            a.z.partial_cmp(&b.z)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        EdgeRenderList { edges }
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    pub fn len(&self) -> usize {
        self.edges.len()
    }

    pub fn find(&self, id: &str) -> Option<&EdgeShape> {
        self.edges.iter().find(|edge| edge.id == id)
    }
}

pub fn point_near_segment(
    point: XYPosition,
    start: XYPosition,
    end: XYPosition,
    tolerance: f64,
) -> bool {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared == 0.0 {
        let distance = ((point.x - start.x).powi(2) + (point.y - start.y).powi(2)).sqrt();
        return distance <= tolerance;
    }
    let mut t = ((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared;
    t = t.clamp(0.0, 1.0);
    let closest_x = start.x + t * dx;
    let closest_y = start.y + t * dy;
    let distance = ((point.x - closest_x).powi(2) + (point.y - closest_y).powi(2)).sqrt();
    distance <= tolerance
}

/// Returns the id of the topmost edge whose interaction stroke contains `point`.
///
/// The point is given in flow coordinates, `zoom` is used to keep the interaction
/// stroke at a constant screen width, mirroring React Flow's interaction path.
pub fn edge_at_point<S: Clone + PartialEq>(
    store: &FlowStore<S>,
    point: XYPosition,
    zoom: f64,
) -> Option<String> {
    let zoom = if zoom <= 0.0 { 1.0 } else { zoom };
    let mut best: Option<(String, f64)> = None;
    for edge in store.edges() {
        if edge.hidden {
            continue;
        }
        let Some(graphics) = store.edge_path_for(edge) else {
            continue;
        };
        let interaction_width = edge.interaction_width.unwrap_or(DEFAULT_INTERACTION_WIDTH);
        let tolerance = interaction_width / 2.0 / zoom;
        if tolerance <= 0.0 {
            continue;
        }
        let path = parse_svg_path(&graphics.path);
        if !edge_hit_test(&path, point, tolerance) {
            continue;
        }
        let z = store.edge_z_index(edge);
        match &best {
            Some((_, best_z)) if *best_z > z => {}
            _ => best = Some((edge.id.clone(), z)),
        }
    }
    best.map(|(id, _)| id)
}

pub fn edge_hit_test(path: &crate::ui::paths::SvgPath, point: XYPosition, tolerance: f64) -> bool {
    let samples = path.sample(48);
    if samples.len() < 2 {
        return false;
    }
    for window in samples.windows(2) {
        let start = XYPosition::new(window[0].x, window[0].y);
        let end = XYPosition::new(window[1].x, window[1].y);
        if point_near_segment(point, start, end, tolerance) {
            return true;
        }
    }
    false
}
