use std::sync::Arc;

use crate::{
    core::{
        geometry::{PanelPosition, Rect, Transform, Viewport, XYPosition},
        node::{Node, NodeLookup},
        xyminimap::{
            MinimapLayout, MinimapNode, minimap_bounding_rect, minimap_layout, minimap_mask_path,
            minimap_nodes, minimap_pan, minimap_viewbox, minimap_zoom, view_bounding_box,
        },
    },
    ui::{paths::parse_svg_path, theme::FlowTheme},
};

pub type MinimapClickHandler = Arc<dyn Fn(XYPosition) + Send + Sync>;
pub type MinimapNodeClickHandler = Arc<dyn Fn(&Node<String>) + Send + Sync>;

/// Callbacks fired by minimap interactions.
#[derive(Clone, Default)]
pub struct MinimapCallbacks {
    pub on_click: Option<MinimapClickHandler>,
    pub on_node_click: Option<MinimapNodeClickHandler>,
}

impl std::fmt::Debug for MinimapCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinimapCallbacks").finish_non_exhaustive()
    }
}

impl PartialEq for MinimapCallbacks {
    fn eq(&self, other: &Self) -> bool {
        option_arc_eq(&self.on_click, &other.on_click)
            && option_arc_eq(&self.on_node_click, &other.on_node_click)
    }
}

fn option_arc_eq<T: ?Sized>(left: &Option<Arc<T>>, right: &Option<Arc<T>>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => Arc::ptr_eq(left, right),
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinimapProps {
    pub width: f64,
    pub height: f64,
    pub offset_scale: f64,
    pub mask_stroke_width: f64,
    pub node_stroke_width: f64,
    pub position: PanelPosition,
    pub margin: f64,
    pub pannable: bool,
    pub zoomable: bool,
    pub zoom_step: f64,
    pub inverse_pan: bool,
    pub aria_label: Option<String>,
    pub bg_color: Option<gpui_kit::Rgba>,
    pub mask_color: Option<gpui_kit::Rgba>,
    pub mask_stroke_color: Option<gpui_kit::Rgba>,
    pub node_color: Option<gpui_kit::Rgba>,
    pub node_stroke_color: Option<gpui_kit::Rgba>,
    pub node_border_radius: f64,
}

impl Default for MinimapProps {
    fn default() -> Self {
        MinimapProps {
            width: 200.0,
            height: 150.0,
            offset_scale: 5.0,
            mask_stroke_width: 1.0,
            node_stroke_width: 2.0,
            position: PanelPosition::BottomRight,
            margin: 15.0,
            pannable: false,
            zoomable: false,
            zoom_step: 0.1,
            inverse_pan: false,
            aria_label: None,
            bg_color: None,
            mask_color: None,
            mask_stroke_color: None,
            node_color: None,
            node_stroke_color: None,
            node_border_radius: 5.0,
        }
    }
}

impl MinimapProps {
    pub fn with_position(mut self, position: PanelPosition) -> Self {
        self.position = position;
        self
    }

    pub fn with_margin(mut self, margin: f64) -> Self {
        self.margin = margin;
        self
    }

    pub fn with_pannable(mut self, pannable: bool) -> Self {
        self.pannable = pannable;
        self
    }

    pub fn with_zoomable(mut self, zoomable: bool) -> Self {
        self.zoomable = zoomable;
        self
    }

    pub fn with_inverse_pan(mut self, inverse_pan: bool) -> Self {
        self.inverse_pan = inverse_pan;
        self
    }

    pub fn with_offset_scale(mut self, offset_scale: f64) -> Self {
        self.offset_scale = offset_scale;
        self
    }

    pub fn with_aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    pub fn with_colors(
        mut self,
        bg_color: gpui_kit::Rgba,
        mask_color: gpui_kit::Rgba,
        node_color: gpui_kit::Rgba,
    ) -> Self {
        self.bg_color = Some(bg_color);
        self.mask_color = Some(mask_color);
        self.node_color = Some(node_color);
        self
    }

    pub fn with_node_style(
        mut self,
        color: gpui_kit::Rgba,
        stroke: gpui_kit::Rgba,
        border_radius: f64,
    ) -> Self {
        self.node_color = Some(color);
        self.node_stroke_color = Some(stroke);
        self.node_border_radius = border_radius;
        self
    }
}

/// Returns the top-left corner of the minimap panel in window coordinates.
pub fn minimap_origin(
    bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
    props: &MinimapProps,
) -> XYPosition {
    let x = f32::from(bounds.origin.x) as f64;
    let y = f32::from(bounds.origin.y) as f64;
    let width = f32::from(bounds.size.width) as f64;
    let height = f32::from(bounds.size.height) as f64;
    let mw = props.width;
    let mh = props.height;
    let margin = props.margin;
    match props.position {
        PanelPosition::TopLeft => XYPosition::new(x + margin, y + margin),
        PanelPosition::TopRight => XYPosition::new(x + width - margin - mw, y + margin),
        PanelPosition::BottomLeft => XYPosition::new(x + margin, y + height - margin - mh),
        PanelPosition::BottomRight => {
            XYPosition::new(x + width - margin - mw, y + height - margin - mh)
        }
        PanelPosition::TopCenter => XYPosition::new(x + (width - mw) / 2.0, y + margin),
        PanelPosition::BottomCenter => {
            XYPosition::new(x + (width - mw) / 2.0, y + height - margin - mh)
        }
        PanelPosition::CenterLeft => XYPosition::new(x + margin, y + (height - mh) / 2.0),
        PanelPosition::CenterRight => {
            XYPosition::new(x + width - margin - mw, y + (height - mh) / 2.0)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinimapRender {
    pub bounds: Rect,
    pub layout: MinimapLayout,
    pub nodes: Vec<MinimapNode>,
    pub view_box: String,
    pub mask_path: String,
}

impl MinimapRender {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn mask_svg(&self) -> crate::ui::paths::SvgPath {
        parse_svg_path(&self.mask_path)
    }

    pub fn node_rect(&self, node: &MinimapNode) -> Rect {
        Rect::new(node.x, node.y, node.width, node.height)
    }
}

pub fn minimap_render<S: Clone>(
    node_lookup: &NodeLookup<S>,
    transform: Transform,
    viewport_width: f64,
    viewport_height: f64,
    props: &MinimapProps,
) -> MinimapRender {
    let vb = view_bounding_box(transform, viewport_width, viewport_height);
    let bounds = minimap_bounding_rect(node_lookup, vb, false);
    let layout = minimap_layout(bounds, props.width, props.height, props.offset_scale);
    MinimapRender {
        bounds,
        view_box: minimap_viewbox(&layout),
        mask_path: minimap_mask_path(&layout),
        layout,
        nodes: minimap_nodes(node_lookup, None),
    }
}

pub fn minimap_click_to_flow(point: XYPosition, layout: &MinimapLayout) -> XYPosition {
    XYPosition::new(
        layout.view_x + point.x * layout.view_scale,
        layout.view_y + point.y * layout.view_scale,
    )
}

pub fn minimap_drag_viewport(
    viewport: Viewport,
    pointer_delta: XYPosition,
    layout: &MinimapLayout,
) -> Viewport {
    minimap_pan(viewport, pointer_delta, layout.view_scale, false)
}

pub fn minimap_zoom_viewport(
    viewport: Viewport,
    delta: f64,
    zoom_step: f64,
    min_zoom: f64,
    max_zoom: f64,
) -> Viewport {
    minimap_zoom(viewport, delta, zoom_step, (min_zoom, max_zoom))
}

pub fn minimap_background(theme: &FlowTheme) -> gpui_kit::Rgba {
    theme.minimap_background
}

pub fn minimap_mask_fill(theme: &FlowTheme) -> gpui_kit::Rgba {
    theme.minimap_mask_background
}

pub fn minimap_node_fill(theme: &FlowTheme) -> gpui_kit::Rgba {
    theme.minimap_node_background
}

pub fn minimap_node_stroke(theme: &FlowTheme) -> gpui_kit::Rgba {
    theme.minimap_node_stroke
}
