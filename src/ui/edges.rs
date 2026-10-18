use crate::core::{
    edge::{
        BezierPathOptions, EdgeMarkerType, PathOptions, SmoothStepPathOptions, StepPathOptions,
    },
    edge_path::{
        get_bezier_edge_center, get_bezier_path, get_edge_center, get_simple_bezier_path,
        get_smooth_step_path, get_step_path, get_straight_path,
    },
    geometry::{EdgePosition, Position, XYPosition},
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct EdgeLabelStyle {
    pub fill: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<u32>,
    pub font_family: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct EdgeLabelBgStyle {
    pub fill: Option<String>,
    pub fill_opacity: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BaseEdgeProps {
    pub path: String,
    pub label_x: Option<f64>,
    pub label_y: Option<f64>,
    pub label: Option<String>,
    pub label_style: Option<EdgeLabelStyle>,
    pub label_show_bg: Option<bool>,
    pub label_bg_style: Option<EdgeLabelBgStyle>,
    pub label_bg_padding: Option<[f64; 2]>,
    pub label_bg_border_radius: Option<f64>,
    pub interaction_width: Option<f64>,
    pub marker_start: Option<String>,
    pub marker_end: Option<String>,
    pub class_name: Option<String>,
    pub style: Option<String>,
}

impl Default for BaseEdgeProps {
    fn default() -> Self {
        BaseEdgeProps {
            path: String::new(),
            label_x: None,
            label_y: None,
            label: None,
            label_style: None,
            label_show_bg: None,
            label_bg_style: None,
            label_bg_padding: None,
            label_bg_border_radius: None,
            interaction_width: Some(20.0),
            marker_start: None,
            marker_end: None,
            class_name: None,
            style: None,
        }
    }
}

impl BaseEdgeProps {
    pub fn interaction_width(&self) -> f64 {
        self.interaction_width.unwrap_or(20.0)
    }

    pub fn has_interaction_path(&self) -> bool {
        self.interaction_width() > 0.0
    }

    pub fn has_label(&self) -> bool {
        self.label.is_some() && self.label_x.is_some() && self.label_y.is_some()
    }

    pub fn path_class_name(&self) -> String {
        match &self.class_name {
            Some(class) => format!("gpui-flow__edge-path {}", class),
            None => "gpui-flow__edge-path".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BaseEdgeRender {
    pub path: BaseEdgeProps,
    pub interaction_path: Option<String>,
    pub label: Option<EdgeLabelRender>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeLabelRender {
    pub x: f64,
    pub y: f64,
    pub label: String,
    pub style: EdgeLabelStyle,
    pub show_bg: bool,
    pub bg_style: EdgeLabelBgStyle,
    pub bg_padding: [f64; 2],
    pub bg_border_radius: f64,
}

pub fn base_edge(props: BaseEdgeProps) -> BaseEdgeRender {
    let interaction_path = if props.has_interaction_path() {
        Some(props.path.clone())
    } else {
        None
    };

    let label = if props.has_label() {
        Some(EdgeLabelRender {
            x: props.label_x.unwrap_or(0.0),
            y: props.label_y.unwrap_or(0.0),
            label: props.label.clone().unwrap_or_default(),
            style: props.label_style.clone().unwrap_or_default(),
            show_bg: props.label_show_bg.unwrap_or(false),
            bg_style: props.label_bg_style.clone().unwrap_or_default(),
            bg_padding: props.label_bg_padding.unwrap_or([2.0, 4.0]),
            bg_border_radius: props.label_bg_border_radius.unwrap_or(2.0),
        })
    } else {
        None
    };

    BaseEdgeRender {
        path: props,
        interaction_path,
        label,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeComponentKind {
    Bezier,
    SimpleBezier,
    Straight,
    Step,
    SmoothStep,
}

impl EdgeComponentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EdgeComponentKind::Bezier => "default",
            EdgeComponentKind::SimpleBezier => "simplebezier",
            EdgeComponentKind::Straight => "straight",
            EdgeComponentKind::Step => "step",
            EdgeComponentKind::SmoothStep => "smoothstep",
        }
    }

    pub fn from_str(value: &str) -> Option<EdgeComponentKind> {
        match value {
            "default" | "bezier" => Some(EdgeComponentKind::Bezier),
            "simplebezier" => Some(EdgeComponentKind::SimpleBezier),
            "straight" => Some(EdgeComponentKind::Straight),
            "step" => Some(EdgeComponentKind::Step),
            "smoothstep" => Some(EdgeComponentKind::SmoothStep),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct EdgePathParams {
    pub source_x: f64,
    pub source_y: f64,
    pub source_position: Option<Position>,
    pub target_x: f64,
    pub target_y: f64,
    pub target_position: Option<Position>,
    pub path_options: Option<PathOptions>,
}

impl EdgePathParams {
    fn bezier_options(&self) -> BezierPathOptions {
        match &self.path_options {
            Some(PathOptions::Bezier(options)) => *options,
            _ => BezierPathOptions::default(),
        }
    }

    fn step_options(&self) -> StepPathOptions {
        match &self.path_options {
            Some(PathOptions::Step(options)) => *options,
            _ => StepPathOptions::default(),
        }
    }

    fn smooth_step_options(&self) -> SmoothStepPathOptions {
        match &self.path_options {
            Some(PathOptions::SmoothStep(options)) => *options,
            _ => SmoothStepPathOptions::default(),
        }
    }
}

impl EdgePathParams {
    pub fn from_position(position: &EdgePosition) -> Self {
        EdgePathParams {
            source_x: position.source_x,
            source_y: position.source_y,
            source_position: Some(position.source_position),
            target_x: position.target_x,
            target_y: position.target_y,
            target_position: Some(position.target_position),
            path_options: None,
        }
    }

    pub fn with_path_options(mut self, options: Option<PathOptions>) -> Self {
        self.path_options = options;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeComponentOutput {
    pub path: String,
    pub label_x: Option<f64>,
    pub label_y: Option<f64>,
    pub kind: EdgeComponentKind,
}

pub fn bezier_edge_path(params: &EdgePathParams) -> EdgeComponentOutput {
    let result = get_bezier_path(
        params.source_x,
        params.source_y,
        params.source_position.unwrap_or(Position::Bottom),
        params.target_x,
        params.target_y,
        params.target_position.unwrap_or(Position::Top),
        params.bezier_options(),
    );
    EdgeComponentOutput {
        path: result.path,
        label_x: Some(result.label_x),
        label_y: Some(result.label_y),
        kind: EdgeComponentKind::Bezier,
    }
}

pub fn simple_bezier_edge_path(params: &EdgePathParams) -> EdgeComponentOutput {
    let result = get_simple_bezier_path(
        params.source_x,
        params.source_y,
        params.target_x,
        params.target_y,
        params.bezier_options(),
    );
    EdgeComponentOutput {
        path: result.path,
        label_x: Some(result.label_x),
        label_y: Some(result.label_y),
        kind: EdgeComponentKind::SimpleBezier,
    }
}

pub fn straight_edge_path(params: &EdgePathParams) -> EdgeComponentOutput {
    let result = get_straight_path(
        params.source_x,
        params.source_y,
        params.target_x,
        params.target_y,
    );
    EdgeComponentOutput {
        path: result.path,
        label_x: Some(result.label_x),
        label_y: Some(result.label_y),
        kind: EdgeComponentKind::Straight,
    }
}

pub fn step_edge_path(params: &EdgePathParams) -> EdgeComponentOutput {
    let result = get_step_path(
        params.source_x,
        params.source_y,
        params.source_position.unwrap_or(Position::Bottom),
        params.target_x,
        params.target_y,
        params.target_position.unwrap_or(Position::Top),
        params.step_options(),
    );
    EdgeComponentOutput {
        path: result.path,
        label_x: Some(result.label_x),
        label_y: Some(result.label_y),
        kind: EdgeComponentKind::Step,
    }
}

pub fn smooth_step_edge_path(params: &EdgePathParams) -> EdgeComponentOutput {
    let result = get_smooth_step_path(
        params.source_x,
        params.source_y,
        params.source_position.unwrap_or(Position::Bottom),
        params.target_x,
        params.target_y,
        params.target_position.unwrap_or(Position::Top),
        params.smooth_step_options(),
        None,
    );
    EdgeComponentOutput {
        path: result.path,
        label_x: Some(result.label_x),
        label_y: Some(result.label_y),
        kind: EdgeComponentKind::SmoothStep,
    }
}

pub fn edge_component_output(
    kind: EdgeComponentKind,
    params: &EdgePathParams,
) -> EdgeComponentOutput {
    match kind {
        EdgeComponentKind::Bezier => bezier_edge_path(params),
        EdgeComponentKind::SimpleBezier => simple_bezier_edge_path(params),
        EdgeComponentKind::Straight => straight_edge_path(params),
        EdgeComponentKind::Step => step_edge_path(params),
        EdgeComponentKind::SmoothStep => smooth_step_edge_path(params),
    }
}

pub fn edge_center_for(kind: EdgeComponentKind, position: &EdgePosition) -> XYPosition {
    let params = EdgePathParams::from_position(position);
    let output = edge_component_output(kind, &params);
    XYPosition::new(output.label_x.unwrap_or(0.0), output.label_y.unwrap_or(0.0))
}

pub fn straight_center(source_x: f64, source_y: f64, target_x: f64, target_y: f64) -> XYPosition {
    let center = get_edge_center(source_x, source_y, target_x, target_y);
    XYPosition::new(center.0, center.1)
}

pub fn bezier_center(
    source_x: f64,
    source_y: f64,
    source_control_x: f64,
    source_control_y: f64,
    target_x: f64,
    target_y: f64,
    target_control_x: f64,
    target_control_y: f64,
) -> XYPosition {
    let center = get_bezier_edge_center(
        source_x,
        source_y,
        target_x,
        target_y,
        source_control_x,
        source_control_y,
        target_control_x,
        target_control_y,
    );
    XYPosition::new(center.0, center.1)
}

pub fn marker_ref(marker: Option<&EdgeMarkerType>) -> Option<String> {
    marker.map(|marker| marker.marker_id(None))
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeMarkers {
    pub start: Option<String>,
    pub end: Option<String>,
}

pub fn edge_markers(
    marker_start: Option<&EdgeMarkerType>,
    marker_end: Option<&EdgeMarkerType>,
) -> EdgeMarkers {
    EdgeMarkers {
        start: marker_ref(marker_start),
        end: marker_ref(marker_end),
    }
}

pub fn default_edge_interaction_width() -> f64 {
    20.0
}
