use crate::core::{
    edge::{EdgeMarker, EdgeMarkerType, MarkerType},
    geometry::{Alignment, Position, Rect, Viewport},
    node::{InternalNode, Node},
};

pub const ERROR_006: &str = "Can't create edge. An edge needs a source and a target.";

pub fn error_006() -> String {
    ERROR_006.to_string()
}

pub fn error_007(old_edge_id: &str) -> String {
    format!("The old edge with id={old_edge_id} does not exist.")
}

pub fn is_rect_object(value: &Rect) -> bool {
    crate::core::utils::is_numeric(value.width)
        && crate::core::utils::is_numeric(value.height)
        && crate::core::utils::is_numeric(value.x)
        && crate::core::utils::is_numeric(value.y)
}

pub fn is_coordinate_extent(extent: Option<&crate::core::node::NodeExtent>) -> bool {
    matches!(extent, Some(crate::core::node::NodeExtent::Coordinates(_)))
}

pub fn node_has_dimensions<S>(node: &Node<S>) -> bool {
    node.has_dimensions()
}

pub fn get_marker_id(marker: Option<&EdgeMarkerType>, id: Option<&str>) -> String {
    match marker {
        None => String::new(),
        Some(marker) => marker.marker_id(id),
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CreateMarkerIdsOptions {
    pub id: Option<String>,
    pub default_color: Option<String>,
    pub default_marker_start: Option<EdgeMarkerType>,
    pub default_marker_end: Option<EdgeMarkerType>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkerProps {
    pub id: String,
    pub color: Option<String>,
    pub marker_type: MarkerType,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub marker_units: Option<String>,
    pub orient: Option<String>,
    pub stroke_width: Option<f64>,
}

impl MarkerProps {
    pub fn from_marker(id: String, color: Option<String>, marker: &EdgeMarker) -> Self {
        MarkerProps {
            id,
            color: marker.color.clone().or(color),
            marker_type: marker.marker_type,
            width: marker.width,
            height: marker.height,
            marker_units: marker.marker_units.clone(),
            orient: marker.orient.clone(),
            stroke_width: marker.stroke_width,
        }
    }
}

pub fn create_marker_ids<S: Clone>(
    edges: &[crate::core::edge::Edge<S>],
    options: &CreateMarkerIdsOptions,
) -> Vec<MarkerProps> {
    let mut ids: Vec<String> = Vec::new();
    let mut markers: Vec<MarkerProps> = Vec::new();

    for edge in edges {
        let candidates = [
            edge.marker_start
                .as_ref()
                .or(options.default_marker_start.as_ref()),
            edge.marker_end
                .as_ref()
                .or(options.default_marker_end.as_ref()),
        ];

        for candidate in candidates {
            let Some(EdgeMarkerType::Marker(marker)) = candidate else {
                continue;
            };

            let marker_id = get_marker_id(
                Some(&EdgeMarkerType::Marker(marker.clone())),
                options.id.as_deref(),
            );

            if ids.iter().any(|existing| existing == &marker_id) {
                continue;
            }

            markers.push(MarkerProps::from_marker(
                marker_id.clone(),
                options.default_color.clone(),
                marker,
            ));
            ids.push(marker_id);
        }
    }

    markers.sort_by(|a, b| a.id.cmp(&b.id));
    markers
}

pub fn get_node_toolbar_transform(
    node_rect: Rect,
    viewport: Viewport,
    position: Position,
    offset: f64,
    align: Alignment,
) -> String {
    let alignment_offset = align.fraction();
    let zoom = viewport.zoom;

    let mut pos = [
        (node_rect.x + node_rect.width * alignment_offset) * zoom + viewport.x,
        node_rect.y * zoom + viewport.y - offset,
    ];
    let mut shift = [-100.0 * alignment_offset, -100.0];

    match position {
        Position::Right => {
            pos = [
                (node_rect.x + node_rect.width) * zoom + viewport.x + offset,
                (node_rect.y + node_rect.height * alignment_offset) * zoom + viewport.y,
            ];
            shift = [0.0, -100.0 * alignment_offset];
        }
        Position::Bottom => {
            pos[1] = (node_rect.y + node_rect.height) * zoom + viewport.y + offset;
            shift[1] = 0.0;
        }
        Position::Left => {
            pos = [
                node_rect.x * zoom + viewport.x - offset,
                (node_rect.y + node_rect.height * alignment_offset) * zoom + viewport.y,
            ];
            shift = [-100.0, -100.0 * alignment_offset];
        }
        Position::Top => {}
    }

    format!(
        "translate({}px, {}px) translate({}%, {}%)",
        crate::core::utils::format_number(pos[0]),
        crate::core::utils::format_number(pos[1]),
        crate::core::utils::format_number(shift[0]),
        crate::core::utils::format_number(shift[1])
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarAlignX {
    Left,
    Center,
    Right,
}

impl ToolbarAlignX {
    pub fn percent(self) -> f64 {
        match self {
            ToolbarAlignX::Left => 0.0,
            ToolbarAlignX::Center => 50.0,
            ToolbarAlignX::Right => 100.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarAlignY {
    Top,
    Center,
    Bottom,
}

impl ToolbarAlignY {
    pub fn percent(self) -> f64 {
        match self {
            ToolbarAlignY::Top => 0.0,
            ToolbarAlignY::Center => 50.0,
            ToolbarAlignY::Bottom => 100.0,
        }
    }
}

pub fn get_edge_toolbar_transform(
    x: f64,
    y: f64,
    zoom: f64,
    align_x: ToolbarAlignX,
    align_y: ToolbarAlignY,
) -> String {
    let scale = if zoom == 0.0 { 1.0 } else { 1.0 / zoom };
    format!(
        "translate({}px, {}px) scale({}) translate({}%, {}%)",
        crate::core::utils::format_number(x),
        crate::core::utils::format_number(y),
        crate::core::utils::format_number(scale),
        crate::core::utils::format_number(-align_x.percent()),
        crate::core::utils::format_number(-align_y.percent())
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AriaLabelConfig {
    pub node_a11y_description_default: String,
    pub node_a11y_description_keyboard_disabled: String,
    pub edge_a11y_description_default: String,
    pub controls_aria_label: String,
    pub controls_zoom_in_aria_label: String,
    pub controls_zoom_out_aria_label: String,
    pub controls_fit_view_aria_label: String,
    pub controls_interactive_aria_label: String,
    pub minimap_aria_label: String,
    pub handle_aria_label: String,
}

impl Default for AriaLabelConfig {
    fn default() -> Self {
        AriaLabelConfig {
            node_a11y_description_default: String::from(
                "Press enter or space to select a node. Press delete to remove it and escape to cancel.",
            ),
            node_a11y_description_keyboard_disabled: String::from(
                "Press enter or space to select a node. You can then use the arrow keys to move the node around. Press delete to remove it and escape to cancel.",
            ),
            edge_a11y_description_default: String::from(
                "Press enter or space to select an edge. You can then press delete to remove it or escape to cancel.",
            ),
            controls_aria_label: String::from("Control Panel"),
            controls_zoom_in_aria_label: String::from("Zoom In"),
            controls_zoom_out_aria_label: String::from("Zoom Out"),
            controls_fit_view_aria_label: String::from("Fit View"),
            controls_interactive_aria_label: String::from("Toggle Interactivity"),
            minimap_aria_label: String::from("Mini Map"),
            handle_aria_label: String::from("Handle"),
        }
    }
}

impl AriaLabelConfig {
    pub fn a11y_live_message(&self, direction: &str, x: f64, y: f64) -> String {
        format!(
            "Moved selected node {direction}. New position, x: {}, y: {}",
            crate::core::utils::format_number(x),
            crate::core::utils::format_number(y)
        )
    }
}

pub fn merge_aria_label_config(partial: Option<AriaLabelConfig>) -> AriaLabelConfig {
    partial.unwrap_or_default()
}

pub fn shallow_node_data<S: PartialEq>(a: &[Node<S>], b: &[Node<S>]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.iter().zip(b.iter()).all(|(left, right)| {
        left.id == right.id && left.node_type == right.node_type && left.data == right.data
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevWarn {
    pub library: String,
    pub help_url: String,
}

impl DevWarn {
    pub fn new(library: impl Into<String>, help_url: impl Into<String>) -> Self {
        DevWarn {
            library: library.into(),
            help_url: help_url.into(),
        }
    }

    pub fn message(&self, id: &str, message: &str) -> String {
        format!(
            "[{}]: {} Help: {}error#{id}",
            self.library, message, self.help_url
        )
    }
}

pub fn is_attribution_visible(pane: Option<Rect>, attribution: Option<Rect>) -> bool {
    let Some(pane) = pane else {
        return true;
    };

    if pane.width == 0.0 && pane.height == 0.0 {
        return true;
    }

    match attribution {
        None => false,
        Some(attribution) => !(attribution.width == 0.0 && attribution.height == 0.0),
    }
}

pub fn handle_attribution_warning_message(library: &str) -> String {
    let framework = format!(
        "{} Flow",
        library
            .chars()
            .next()
            .map(|first| first.to_uppercase().to_string())
            .unwrap_or_default()
            + &library.chars().skip(1).collect::<String>()
    );

    format!(
        "{framework}: It seems like you are hiding the attribution. Please only do this when you are subscribed to {framework} Pro: https://{library}flow.dev/remove-attr\nYou can ignore this warning if you are subscribed."
    )
}

pub fn get_node_rect_with_position<S: Clone>(
    node: &Node<S>,
    position: crate::core::geometry::XYPosition,
) -> Rect {
    let width = node
        .measured
        .width
        .or(node.width)
        .or(node.initial_width)
        .unwrap_or(0.0);
    let height = node
        .measured
        .height
        .or(node.height)
        .or(node.initial_height)
        .unwrap_or(0.0);

    Rect::new(position.x, position.y, width, height)
}

pub fn internal_node_rect<S: Clone>(node: &InternalNode<S>) -> Rect {
    let dimensions = node.size();
    Rect::new(
        node.internals.position_absolute.x,
        node.internals.position_absolute.y,
        dimensions.width,
        dimensions.height,
    )
}

pub fn get_nodes_intersecting_rect<S: Clone>(
    nodes: &[InternalNode<S>],
    rect: Rect,
    partially: bool,
    exclude: Option<&str>,
) -> Vec<InternalNode<S>> {
    nodes
        .iter()
        .filter(|node| {
            if let Some(exclude) = exclude
                && node.id == exclude
            {
                return false;
            }

            let node_rect = internal_node_rect(node);
            let overlapping = crate::core::utils::get_overlapping_area(node_rect, rect);
            let partially_visible = partially && overlapping > 0.0;

            partially_visible
                || overlapping >= node_rect.width * node_rect.height
                || overlapping >= rect.width * rect.height
        })
        .cloned()
        .collect()
}

pub fn is_node_intersecting<S: Clone>(node: &InternalNode<S>, area: Rect, partially: bool) -> bool {
    let node_rect = internal_node_rect(node);
    let overlapping = crate::core::utils::get_overlapping_area(node_rect, area);
    let partially_visible = partially && overlapping > 0.0;

    partially_visible
        || overlapping >= area.width * area.height
        || overlapping >= node_rect.width * node_rect.height
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Resolvers<T> {
    resolved: Option<T>,
}

impl<T> Resolvers<T> {
    pub fn new() -> Self {
        Resolvers { resolved: None }
    }

    pub fn resolve(&mut self, value: T) {
        self.resolved = Some(value);
    }

    pub fn try_resolve(&mut self) -> Option<T> {
        self.resolved.take()
    }

    pub fn is_resolved(&self) -> bool {
        self.resolved.is_some()
    }
}
