use crate::core::geometry::Position;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerType {
    Arrow,
    ArrowClosed,
}

impl MarkerType {
    pub const ALL: [MarkerType; 2] = [MarkerType::Arrow, MarkerType::ArrowClosed];

    pub fn as_str(self) -> &'static str {
        match self {
            MarkerType::Arrow => "arrow",
            MarkerType::ArrowClosed => "arrowclosed",
        }
    }

    pub fn from_str(value: &str) -> Option<MarkerType> {
        match value {
            "arrow" => Some(MarkerType::Arrow),
            "arrowclosed" => Some(MarkerType::ArrowClosed),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeMarker {
    pub marker_type: MarkerType,
    pub color: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub marker_units: Option<String>,
    pub orient: Option<String>,
    pub stroke_width: Option<f64>,
}

impl EdgeMarker {
    pub fn new(marker_type: MarkerType) -> Self {
        EdgeMarker {
            marker_type,
            color: None,
            width: None,
            height: None,
            marker_units: None,
            orient: None,
            stroke_width: None,
        }
    }

    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn with_size(mut self, width: f64, height: f64) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    pub fn with_stroke_width(mut self, stroke_width: f64) -> Self {
        self.stroke_width = Some(stroke_width);
        self
    }

    pub fn marker_id(&self, id: Option<&str>) -> String {
        let mut parts = Vec::new();
        parts.push(format!("type={}", self.marker_type.as_str()));
        if let Some(color) = &self.color {
            parts.push(format!("color={color}"));
        }
        if let Some(width) = self.width {
            parts.push(format!("width={width}"));
        }
        if let Some(height) = self.height {
            parts.push(format!("height={height}"));
        }
        if let Some(marker_units) = &self.marker_units {
            parts.push(format!("markerUnits={marker_units}"));
        }
        if let Some(orient) = &self.orient {
            parts.push(format!("orient={orient}"));
        }
        if let Some(stroke_width) = self.stroke_width {
            parts.push(format!("strokeWidth={stroke_width}"));
        }
        parts.sort();
        let prefix = id.map(|id| format!("{id}__")).unwrap_or_default();
        format!("{prefix}{}", parts.join("&"))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EdgeMarkerType {
    Name(String),
    Marker(EdgeMarker),
}

impl EdgeMarkerType {
    pub fn marker_id(&self, id: Option<&str>) -> String {
        match self {
            EdgeMarkerType::Name(name) => name.clone(),
            EdgeMarkerType::Marker(marker) => marker.marker_id(id),
        }
    }

    pub fn from_str(value: &str) -> EdgeMarkerType {
        match MarkerType::from_str(value) {
            Some(marker_type) => EdgeMarkerType::Marker(EdgeMarker::new(marker_type)),
            None => EdgeMarkerType::Name(value.to_string()),
        }
    }

    pub fn as_marker(&self) -> Option<&EdgeMarker> {
        match self {
            EdgeMarkerType::Name(_) => None,
            EdgeMarkerType::Marker(marker) => Some(marker),
        }
    }
}

impl From<MarkerType> for EdgeMarkerType {
    fn from(value: MarkerType) -> Self {
        EdgeMarkerType::Marker(EdgeMarker::new(value))
    }
}

impl From<EdgeMarker> for EdgeMarkerType {
    fn from(value: EdgeMarker) -> Self {
        EdgeMarkerType::Marker(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectionLineType {
    #[default]
    Bezier,
    Straight,
    Step,
    SmoothStep,
    SimpleBezier,
}

impl ConnectionLineType {
    pub fn as_str(self) -> &'static str {
        match self {
            ConnectionLineType::Bezier => "default",
            ConnectionLineType::Straight => "straight",
            ConnectionLineType::Step => "step",
            ConnectionLineType::SmoothStep => "smoothstep",
            ConnectionLineType::SimpleBezier => "simplebezier",
        }
    }

    pub fn from_str(value: &str) -> Option<ConnectionLineType> {
        match value {
            "default" | "bezier" => Some(ConnectionLineType::Bezier),
            "straight" => Some(ConnectionLineType::Straight),
            "step" => Some(ConnectionLineType::Step),
            "smoothstep" => Some(ConnectionLineType::SmoothStep),
            "simplebezier" => Some(ConnectionLineType::SimpleBezier),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BezierPathOptions {
    pub curvature: f64,
}

impl Default for BezierPathOptions {
    fn default() -> Self {
        BezierPathOptions { curvature: 0.25 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmoothStepPathOptions {
    pub offset: f64,
    pub border_radius: f64,
    pub step_position: f64,
}

impl Default for SmoothStepPathOptions {
    fn default() -> Self {
        SmoothStepPathOptions {
            offset: 20.0,
            border_radius: 5.0,
            step_position: 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepPathOptions {
    pub offset: f64,
}

impl Default for StepPathOptions {
    fn default() -> Self {
        StepPathOptions { offset: 20.0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PathOptions {
    Bezier(BezierPathOptions),
    SmoothStep(SmoothStepPathOptions),
    Step(StepPathOptions),
}

impl Default for PathOptions {
    fn default() -> Self {
        PathOptions::Bezier(BezierPathOptions::default())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge<S = String> {
    pub id: String,
    pub edge_type: Option<String>,
    pub source: String,
    pub target: String,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
    pub animated: bool,
    pub hidden: bool,
    pub deletable: Option<bool>,
    pub selectable: Option<bool>,
    pub data: Option<S>,
    pub selected: bool,
    pub marker_start: Option<EdgeMarkerType>,
    pub marker_end: Option<EdgeMarkerType>,
    pub z_index: Option<f64>,
    pub aria_label: Option<String>,
    pub interaction_width: Option<f64>,
    pub label: Option<String>,
    pub label_show_bg: Option<bool>,
    pub path_options: Option<PathOptions>,
    pub focusable: Option<bool>,
    pub reconnectable: Option<bool>,
}

impl<S> Default for Edge<S> {
    fn default() -> Self {
        Edge {
            id: String::new(),
            edge_type: None,
            source: String::new(),
            target: String::new(),
            source_handle: None,
            target_handle: None,
            animated: false,
            hidden: false,
            deletable: None,
            selectable: None,
            data: None,
            selected: false,
            marker_start: None,
            marker_end: None,
            z_index: None,
            aria_label: None,
            interaction_width: None,
            label: None,
            label_show_bg: None,
            path_options: None,
            focusable: None,
            reconnectable: None,
        }
    }
}

impl<S> Edge<S> {
    pub fn new(
        id: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Edge {
            id: id.into(),
            source: source.into(),
            target: target.into(),
            ..Edge::default()
        }
    }

    pub fn with_type(mut self, edge_type: impl Into<String>) -> Self {
        self.edge_type = Some(edge_type.into());
        self
    }

    pub fn with_source_handle(mut self, handle: impl Into<String>) -> Self {
        self.source_handle = Some(handle.into());
        self
    }

    pub fn with_target_handle(mut self, handle: impl Into<String>) -> Self {
        self.target_handle = Some(handle.into());
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn with_marker_end(mut self, marker: impl Into<EdgeMarkerType>) -> Self {
        self.marker_end = Some(marker.into());
        self
    }

    pub fn with_marker_start(mut self, marker: impl Into<EdgeMarkerType>) -> Self {
        self.marker_start = Some(marker.into());
        self
    }

    pub fn with_z_index(mut self, z_index: f64) -> Self {
        self.z_index = Some(z_index);
        self
    }

    pub fn with_path_options(mut self, options: PathOptions) -> Self {
        self.path_options = Some(options);
        self
    }

    pub fn type_name(&self) -> &str {
        self.edge_type.as_deref().unwrap_or("default")
    }

    pub fn is_deletable(&self) -> bool {
        self.deletable.unwrap_or(true)
    }

    pub fn is_selectable(&self, elements_selectable: bool) -> bool {
        self.selectable.unwrap_or(elements_selectable)
    }

    pub fn is_focusable(&self, edges_focusable: bool) -> bool {
        self.focusable.unwrap_or(edges_focusable)
    }
}

pub type EdgeLookup<S = String> = std::collections::BTreeMap<String, Edge<S>>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Connection {
    pub source: String,
    pub target: String,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
}

impl Connection {
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Connection {
            source: source.into(),
            target: target.into(),
            source_handle: None,
            target_handle: None,
        }
    }

    pub fn with_source_handle(mut self, handle: impl Into<String>) -> Self {
        self.source_handle = Some(handle.into());
        self
    }

    pub fn with_target_handle(mut self, handle: impl Into<String>) -> Self {
        self.target_handle = Some(handle.into());
        self
    }

    pub fn reversed(&self) -> Connection {
        Connection {
            source: self.target.clone(),
            target: self.source.clone(),
            source_handle: self.target_handle.clone(),
            target_handle: self.source_handle.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HandleConnection {
    pub connection: Connection,
    pub edge_id: String,
}

impl HandleConnection {
    pub fn new(connection: Connection, edge_id: impl Into<String>) -> Self {
        HandleConnection {
            connection,
            edge_id: edge_id.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeProps<S = String> {
    pub id: String,
    pub source: String,
    pub target: String,
    pub edge_type: Option<String>,
    pub selected: bool,
    pub animated: bool,
    pub selectable: bool,
    pub deletable: bool,
    pub label: Option<String>,
    pub source_x: f64,
    pub source_y: f64,
    pub target_x: f64,
    pub target_y: f64,
    pub source_position: Position,
    pub target_position: Position,
    pub data: Option<S>,
    pub source_handle_id: Option<String>,
    pub target_handle_id: Option<String>,
    pub marker_start: Option<String>,
    pub marker_end: Option<String>,
    pub interaction_width: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EdgeGraphics {
    pub path: String,
    pub label_x: f64,
    pub label_y: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl EdgeGraphics {
    pub fn new(
        path: impl Into<String>,
        label_x: f64,
        label_y: f64,
        offset_x: f64,
        offset_y: f64,
    ) -> Self {
        EdgeGraphics {
            path: path.into(),
            label_x,
            label_y,
            offset_x,
            offset_y,
        }
    }
}

pub const DEFAULT_EDGE_STROKE: &str = "#b1b1b7";
pub const DEFAULT_EDGE_STROKE_SELECTED: &str = "#555";
pub const DEFAULT_EDGE_STROKE_WIDTH: f64 = 1.0;
pub const DEFAULT_INTERACTION_WIDTH: f64 = 20.0;
