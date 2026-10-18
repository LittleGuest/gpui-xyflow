use crate::{
    core::{
        edge::Edge,
        geometry::{PanOnScrollMode, Transform, Viewport},
        node::{InternalNode, Node},
        panzoom::{PanOnDrag, PanZoomOptions},
        xypanzoom::{PanZoomUpdateOptions, XYPanZoom},
    },
    ui::store::FlowProps,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RendererKeyState {
    pub selection_key_pressed: bool,
    pub pan_activation_key_pressed: bool,
    pub zoom_activation_key_pressed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RendererDerived {
    pub pan_on_drag: bool,
    pub pan_on_scroll: bool,
    pub selection_on_drag: bool,
    pub is_selecting: bool,
    pub zoom_pane_pan_on_drag: bool,
    pub zoom_pane_pan_on_scroll: bool,
}

pub fn derive_renderer_state<S: Clone>(
    props: &FlowProps<S>,
    keys: RendererKeyState,
    user_selection_active: bool,
) -> RendererDerived {
    let pan_on_drag = keys.pan_activation_key_pressed || props.pan_on_drag.is_enabled();
    let pan_on_scroll = keys.pan_activation_key_pressed || props.pan_on_scroll;
    let selection_on_drag =
        props.selection_on_drag && !matches!(props.pan_on_drag, PanOnDrag::Enabled);
    let is_selecting = keys.selection_key_pressed || user_selection_active || selection_on_drag;

    RendererDerived {
        pan_on_drag,
        pan_on_scroll,
        selection_on_drag,
        is_selecting,
        zoom_pane_pan_on_drag: !keys.selection_key_pressed && pan_on_drag,
        zoom_pane_pan_on_scroll: pan_on_scroll,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RendererPaneOptions {
    pub pan_on_drag: bool,
    pub is_selecting: bool,
    pub selection_mode: crate::core::geometry::SelectionMode,
    pub selection_key_pressed: bool,
    pub pane_click_distance: f64,
    pub selection_on_drag: bool,
}

pub fn derive_pane_options<S: Clone>(
    props: &FlowProps<S>,
    derived: RendererDerived,
    keys: RendererKeyState,
) -> RendererPaneOptions {
    RendererPaneOptions {
        pan_on_drag: derived.pan_on_drag,
        is_selecting: derived.is_selecting,
        selection_mode: props.selection_mode,
        selection_key_pressed: keys.selection_key_pressed,
        pane_click_distance: props.pane_click_distance,
        selection_on_drag: derived.selection_on_drag,
    }
}

pub fn zoom_pane_update_options<S: Clone>(
    props: &FlowProps<S>,
    derived: RendererDerived,
    keys: RendererKeyState,
    user_selection_active: bool,
    connection_in_progress: bool,
) -> PanZoomUpdateOptions {
    PanZoomUpdateOptions {
        no_wheel_class_name: props.no_wheel_class_name.clone(),
        no_pan_class_name: props.no_pan_class_name.clone(),
        on_pane_context_menu: props.callbacks.on_pane_context_menu.is_some(),
        user_selection_active,
        pan_on_scroll: derived.zoom_pane_pan_on_scroll,
        pan_on_drag: if derived.zoom_pane_pan_on_drag {
            props.pan_on_drag.clone()
        } else {
            PanOnDrag::Disabled
        },
        pan_on_scroll_mode: props.pan_on_scroll_mode,
        pan_on_scroll_speed: props.pan_on_scroll_speed,
        prevent_scrolling: props.prevent_scrolling,
        zoom_on_pinch: props.zoom_on_pinch,
        zoom_on_scroll: props.zoom_on_scroll,
        zoom_on_double_click: props.zoom_on_double_click,
        pan_activation_key_pressed: keys.pan_activation_key_pressed,
        zoom_activation_key_pressed: keys.zoom_activation_key_pressed,
        lib: props.lib.clone(),
        connection_in_progress,
        pane_click_distance: props.pane_click_distance,
        selection_on_drag: derived.selection_on_drag,
    }
}

pub fn zoom_pane_options<S: Clone>(props: &FlowProps<S>) -> PanZoomOptions {
    let mut options = props.pan_zoom_options();
    options.pan_on_drag = props.pan_on_drag.clone();
    options.pan_on_scroll_mode = props.pan_on_scroll_mode;
    options
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomPanePhase {
    pub mounted: bool,
    pub destroyed: bool,
    pub viewport: Viewport,
}

impl Default for ZoomPanePhase {
    fn default() -> Self {
        ZoomPanePhase {
            mounted: false,
            destroyed: false,
            viewport: Viewport::IDENTITY,
        }
    }
}

pub struct ZoomPane<S = String> {
    pan_zoom: Option<XYPanZoom>,
    phase: ZoomPanePhase,
    is_controlled_viewport: bool,
    _marker: std::marker::PhantomData<S>,
}

impl<S> Default for ZoomPane<S> {
    fn default() -> Self {
        ZoomPane {
            pan_zoom: None,
            phase: ZoomPanePhase::default(),
            is_controlled_viewport: false,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S: Clone> ZoomPane<S> {
    pub fn mount(
        props: &FlowProps<S>,
        viewport: Viewport,
        extent: crate::core::geometry::CoordinateExtent,
        is_controlled_viewport: bool,
    ) -> Self {
        let options = zoom_pane_options(props);
        let pan_zoom = XYPanZoom::new(viewport, options, extent);
        ZoomPane {
            phase: ZoomPanePhase {
                mounted: true,
                destroyed: false,
                viewport,
            },
            pan_zoom: Some(pan_zoom),
            is_controlled_viewport,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn mounted(&self) -> bool {
        self.phase.mounted
    }

    pub fn destroyed(&self) -> bool {
        self.phase.destroyed
    }

    pub fn is_controlled_viewport(&self) -> bool {
        self.is_controlled_viewport
    }

    pub fn phase(&self) -> ZoomPanePhase {
        self.phase
    }

    pub fn pan_zoom(&self) -> Option<&XYPanZoom> {
        self.pan_zoom.as_ref()
    }

    pub fn pan_zoom_mut(&mut self) -> Option<&mut XYPanZoom> {
        self.pan_zoom.as_mut()
    }

    pub fn update(&mut self, options: &PanZoomUpdateOptions) -> bool {
        match self.pan_zoom.as_mut() {
            Some(pan_zoom) => {
                pan_zoom.update(options);
                true
            }
            None => false,
        }
    }

    pub fn update_from_props(
        &mut self,
        props: &FlowProps<S>,
        keys: RendererKeyState,
        user_selection_active: bool,
        connection_in_progress: bool,
    ) -> bool {
        let derived = derive_renderer_state(props, keys, user_selection_active);
        let options = zoom_pane_update_options(
            props,
            derived,
            keys,
            user_selection_active,
            connection_in_progress,
        );
        self.update(&options)
    }

    pub fn viewport(&self) -> Viewport {
        self.phase.viewport
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.phase.viewport = viewport;
        if let Some(pan_zoom) = self.pan_zoom.as_mut() {
            pan_zoom.sync_viewport(viewport);
        }
    }

    pub fn transform(&self) -> Transform {
        self.phase.viewport.transform()
    }

    pub fn on_transform_change(&mut self, viewport: Viewport) -> Viewport {
        if !self.is_controlled_viewport {
            self.set_viewport(viewport);
        }
        viewport
    }

    pub fn destroy(&mut self) {
        if let Some(pan_zoom) = self.pan_zoom.as_mut() {
            pan_zoom.destroy();
        }
        self.phase.destroyed = true;
        self.phase.mounted = false;
    }

    pub fn remount(&mut self) {
        if let Some(pan_zoom) = self.pan_zoom.as_mut() {
            pan_zoom.restore();
        }
        self.phase.destroyed = false;
        self.phase.mounted = true;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportPaneState {
    pub transform: Transform,
    pub zoom: f64,
}

pub fn viewport_transform(viewport: Viewport) -> Transform {
    viewport.transform()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderLayer {
    Edges,
    ConnectionLine,
    EdgeLabels,
    Nodes,
    ViewportPortal,
}

impl RenderLayer {
    pub const ORDER: [RenderLayer; 5] = [
        RenderLayer::Edges,
        RenderLayer::ConnectionLine,
        RenderLayer::EdgeLabels,
        RenderLayer::Nodes,
        RenderLayer::ViewportPortal,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            RenderLayer::Edges => "edges",
            RenderLayer::ConnectionLine => "connectionline",
            RenderLayer::EdgeLabels => "edgelabels",
            RenderLayer::Nodes => "nodes",
            RenderLayer::ViewportPortal => "viewportportal",
        }
    }

    pub fn class_name(self, rf_id: &str) -> String {
        format!("{rf_id}__{}", self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderSnapshot<S = String> {
    pub transform: Transform,
    pub visible_nodes: Vec<InternalNode<S>>,
    pub visible_edges: Vec<Edge<S>>,
    pub layers: Vec<RenderLayer>,
}

pub fn render_snapshot<S: Clone>(
    transform: Transform,
    size: crate::core::geometry::Dimensions,
    node_lookup: &crate::core::node::NodeLookup<S>,
    edges: &[Edge<S>],
    only_render_visible_elements: bool,
) -> RenderSnapshot<S> {
    let visible_nodes: Vec<InternalNode<S>> = if only_render_visible_elements {
        crate::core::graph::get_nodes_inside(
            node_lookup,
            crate::core::geometry::Rect::new(0.0, 0.0, size.width, size.height),
            transform,
            true,
            false,
        )
    } else {
        node_lookup.values().cloned().collect()
    };

    let visible_edges: Vec<Edge<S>> = if only_render_visible_elements {
        edges
            .iter()
            .filter(|edge| {
                let source = node_lookup.get(&edge.source);
                let target = node_lookup.get(&edge.target);
                match (source, target) {
                    (Some(source), Some(target)) => crate::core::graph::is_edge_visible(
                        source,
                        target,
                        size.width,
                        size.height,
                        transform,
                    ),
                    _ => false,
                }
            })
            .cloned()
            .collect()
    } else {
        edges.to_vec()
    };

    RenderSnapshot {
        transform,
        visible_nodes,
        visible_edges,
        layers: RenderLayer::ORDER.to_vec(),
    }
}

pub type ConnectionLineMode = PanOnScrollMode;

pub fn node_is_visible<S: Clone>(
    node: &Node<S>,
    node_lookup: &crate::core::node::NodeLookup<S>,
) -> bool {
    node_lookup.contains_key(&node.id)
}
