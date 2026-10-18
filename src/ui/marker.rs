use crate::core::edge::{EdgeMarkerType, MarkerType};
use crate::core::geometry::Position;
use crate::ui::paths::{SvgPath, polyline};
use gpui_kit::px;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerVariant {
    Arrow,
    ArrowClosed,
}

impl MarkerVariant {
    pub fn from_str(value: &str) -> Option<MarkerVariant> {
        match value {
            "arrow" => Some(MarkerVariant::Arrow),
            "arrowclosed" => Some(MarkerVariant::ArrowClosed),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            MarkerVariant::Arrow => "arrow",
            MarkerVariant::ArrowClosed => "arrowclosed",
        }
    }

    pub fn is_closed(self) -> bool {
        matches!(self, MarkerVariant::ArrowClosed)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkerSpec {
    pub marker_type: MarkerType,
    pub width: f64,
    pub height: f64,
    pub color: Option<String>,
    pub variant: MarkerVariant,
}

impl MarkerSpec {
    pub fn new(marker_type: MarkerType) -> Self {
        MarkerSpec {
            marker_type,
            width: 12.5,
            height: 12.5,
            color: None,
            variant: MarkerVariant::ArrowClosed,
        }
    }

    pub fn from_marker_type(marker: &EdgeMarkerType) -> Option<Self> {
        match marker {
            EdgeMarkerType::Marker(endpoint) => {
                let mut spec = MarkerSpec::new(endpoint.marker_type);
                if let Some(width) = endpoint.width {
                    spec.width = width;
                }
                if let Some(height) = endpoint.height {
                    spec.height = height;
                }
                spec.color = endpoint.color.clone();
                Some(spec)
            }
            EdgeMarkerType::Name(_) => None,
        }
    }

    pub fn with_size(mut self, width: f64, height: f64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_variant(mut self, variant: MarkerVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn is_start(&self) -> bool {
        matches!(self.marker_type, MarkerType::Arrow)
    }

    pub fn is_end(&self) -> bool {
        matches!(self.marker_type, MarkerType::ArrowClosed)
    }

    pub fn view_box(&self) -> String {
        format!("0 0 {} {}", self.width, self.height)
    }

    pub fn ref_x(&self) -> f64 {
        if self.marker_type == MarkerType::ArrowClosed {
            self.width
        } else {
            0.0
        }
    }

    pub fn ref_y(&self) -> f64 {
        self.height / 2.0
    }

    pub fn orient(&self) -> &'static str {
        match self.marker_type {
            MarkerType::Arrow => "auto-start-reverse",
            MarkerType::ArrowClosed => "auto-start-reverse",
        }
    }

    pub fn marker_units(&self) -> &'static str {
        "strokeWidth"
    }

    pub fn marker_path(&self) -> SvgPath {
        if self.variant.is_closed() {
            polyline(&[
                crate::ui::paths::SvgPoint::new(0.0, 0.0),
                crate::ui::paths::SvgPoint::new(self.width, self.height / 2.0),
                crate::ui::paths::SvgPoint::new(0.0, self.height),
            ])
            .closed(true)
        } else {
            polyline(&[
                crate::ui::paths::SvgPoint::new(0.0, 0.0),
                crate::ui::paths::SvgPoint::new(self.width * 0.5, self.height * 0.5),
                crate::ui::paths::SvgPoint::new(0.0, self.height),
            ])
        }
    }

    pub fn stroke_width(&self) -> Option<f32> {
        if self.variant.is_closed() {
            None
        } else {
            Some(1.0)
        }
    }

    pub fn id_for_edge(&self, edge_id: &str, is_start: bool) -> String {
        let suffix = if is_start { "start" } else { "end" };
        format!("{}-{}-{}", edge_id, self.marker_type.as_str(), suffix)
    }

    pub fn url_for_edge(&self, edge_id: &str, is_start: bool) -> String {
        format!("url(#{})", self.id_for_edge(edge_id, is_start))
    }

    pub fn pixel_width(&self) -> gpui_kit::Pixels {
        px(self.width as f32)
    }

    pub fn pixel_height(&self) -> gpui_kit::Pixels {
        px(self.height as f32)
    }

    pub fn angle_for(&self, source: (f64, f64), target: (f64, f64)) -> f64 {
        let dx = target.0 - source.0;
        let dy = target.1 - source.1;
        if dx == 0.0 && dy == 0.0 {
            return 0.0;
        }
        let angle = dy.atan2(dx).to_degrees();
        match self.marker_type {
            MarkerType::Arrow => angle + 180.0,
            MarkerType::ArrowClosed => angle,
        }
    }

    pub fn tip_offset(&self) -> (f64, f64) {
        match self.marker_type {
            MarkerType::Arrow => (-self.width, 0.0),
            MarkerType::ArrowClosed => (0.0, 0.0),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarkerSet {
    pub start: Option<MarkerSpec>,
    pub end: Option<MarkerSpec>,
}

impl MarkerSet {
    pub fn new() -> Self {
        MarkerSet {
            start: None,
            end: None,
        }
    }

    pub fn from_markers(
        marker_start: Option<&EdgeMarkerType>,
        marker_end: Option<&EdgeMarkerType>,
    ) -> Self {
        MarkerSet {
            start: marker_start.and_then(MarkerSpec::from_marker_type),
            end: marker_end.and_then(MarkerSpec::from_marker_type),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start.is_none() && self.end.is_none()
    }

    pub fn has_start(&self) -> bool {
        self.start.is_some()
    }

    pub fn has_end(&self) -> bool {
        self.end.is_some()
    }

    pub fn defs_for_edge(&self, edge_id: &str) -> Vec<(String, MarkerSpec, bool)> {
        let mut defs = Vec::new();
        if let Some(ref spec) = self.start {
            defs.push((spec.id_for_edge(edge_id, true), spec.clone(), true));
        }
        if let Some(ref spec) = self.end {
            defs.push((spec.id_for_edge(edge_id, false), spec.clone(), false));
        }
        defs
    }
}

pub fn marker_orientation(position: Position) -> f64 {
    match position {
        Position::Top => -90.0,
        Position::Right => 0.0,
        Position::Bottom => 90.0,
        Position::Left => 180.0,
    }
}

pub fn marker_anchor(position: Position) -> (f64, f64) {
    match position {
        Position::Top => (0.5, 1.0),
        Position::Right => (0.0, 0.5),
        Position::Bottom => (0.5, 0.0),
        Position::Left => (1.0, 0.5),
    }
}

pub fn arrow_head_points(
    tip: (f64, f64),
    angle_degrees: f64,
    length: f64,
    spread_degrees: f64,
) -> [(f64, f64); 3] {
    let angle = angle_degrees.to_radians();
    let left = angle + spread_degrees.to_radians();
    let right = angle - spread_degrees.to_radians();
    [
        tip,
        (
            tip.0 - length * left.cos(),
            tip.1 - length * left.sin(),
        ),
        (
            tip.0 - length * right.cos(),
            tip.1 - length * right.sin(),
        ),
    ]
}

pub fn marker_path_string(spec: &MarkerSpec) -> String {
    spec.marker_path().to_string()
}
