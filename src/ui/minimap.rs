use crate::core::geometry::{Rect, Transform, Viewport, XYPosition};
use crate::core::node::NodeLookup;
use crate::core::xyminimap::{
    minimap_bounding_rect, minimap_layout, minimap_mask_path, minimap_nodes, minimap_pan,
    minimap_viewbox, minimap_zoom, view_bounding_box, MinimapLayout, MinimapNode,
};
use crate::ui::paths::parse_svg_path;
use crate::ui::theme::FlowTheme;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinimapProps {
    pub width: f64,
    pub height: f64,
    pub padding: f64,
    pub mask_stroke_width: f64,
    pub node_stroke_width: f64,
}

impl Default for MinimapProps {
    fn default() -> Self {
        MinimapProps {
            width: 200.0,
            height: 150.0,
            padding: 5.0,
            mask_stroke_width: 1.0,
            node_stroke_width: 2.0,
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
    props: MinimapProps,
) -> MinimapRender {
    let vb = view_bounding_box(transform, viewport_width, viewport_height);
    let bounds = minimap_bounding_rect(node_lookup, vb, false);
    let layout = minimap_layout(bounds, props.width, props.height, props.padding);
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
