use std::collections::BTreeMap;

use crate::core::{
    edge::Edge,
    geometry::{CoordinateExtent, Dimensions, NodeOrigin, SnapGrid, Transform, XYPosition},
    node::{InternalNode, Node, NodeDragItem, NodeExtent, NodeLookup},
    store::is_parent_selected,
    utils::{
        box_to_rect, calc_auto_pan, clamp_position, drag_item_to_box, get_bounds_of_boxes,
        get_bounds_of_rects, get_node_dimensions, rect_to_box, snap_position,
    },
};

pub struct DragStore {
    pub nodes: Vec<Node<String>>,
    pub edges: Vec<Edge<String>>,
    pub node_lookup: NodeLookup,
    pub node_extent: CoordinateExtent,
    pub snap_grid: SnapGrid,
    pub snap_to_grid: bool,
    pub node_origin: NodeOrigin,
    pub multi_selection_active: bool,
    pub nodes_draggable: bool,
    pub auto_pan_on_node_drag: bool,
    pub auto_pan_speed: f64,
    pub node_extent_override: Option<CoordinateExtent>,
}

impl Default for DragStore {
    fn default() -> Self {
        DragStore {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_lookup: NodeLookup::new(),
            node_extent: crate::core::constants::INFINITE_EXTENT,
            snap_grid: crate::core::constants::DEFAULT_SNAP_GRID,
            snap_to_grid: false,
            node_origin: crate::core::constants::DEFAULT_NODE_ORIGIN,
            multi_selection_active: false,
            nodes_draggable: true,
            auto_pan_on_node_drag: true,
            auto_pan_speed: crate::core::constants::DEFAULT_AUTO_PAN_SPEED,
            node_extent_override: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodePositionUpdate {
    pub position: XYPosition,
    pub position_absolute: XYPosition,
}

pub struct CalculateNodePositionParams<'a, S> {
    pub node_id: &'a str,
    pub next_position: XYPosition,
    pub node_lookup: &'a NodeLookup<S>,
    pub node_origin: NodeOrigin,
    pub node_extent: CoordinateExtent,
}

pub fn calculate_node_position<S: Clone>(
    params: CalculateNodePositionParams<'_, S>,
) -> Option<NodePositionUpdate> {
    let node = params.node_lookup.get(params.node_id)?;
    let parent_node = node
        .parent_id
        .as_ref()
        .and_then(|parent_id| params.node_lookup.get(parent_id));
    let parent_position = parent_node
        .map(|parent| parent.internals.position_absolute)
        .unwrap_or(XYPosition::ZERO);

    let origin = node.origin.unwrap_or(params.node_origin);
    let mut extent = node
        .extent
        .and_then(|extent| extent.coordinates())
        .unwrap_or(params.node_extent);

    if matches!(node.extent, Some(NodeExtent::Parent)) && !node.expand_parent {
        if let Some(parent) = parent_node {
            let parent_dimensions = parent.size();
            if parent_dimensions.width > 0.0 && parent_dimensions.height > 0.0 {
                extent = [
                    [parent_position.x, parent_position.y],
                    [
                        parent_position.x + parent_dimensions.width,
                        parent_position.y + parent_dimensions.height,
                    ],
                ];
            }
        }
    } else if parent_node.is_some()
        && let Some(NodeExtent::Coordinates(node_extent)) = node.extent
    {
        extent = [
            [
                node_extent[0][0] + parent_position.x,
                node_extent[0][1] + parent_position.y,
            ],
            [
                node_extent[1][0] + parent_position.x,
                node_extent[1][1] + parent_position.y,
            ],
        ];
    }

    let dimensions = node.size();
    let position_absolute = clamp_position(params.next_position, extent, dimensions);

    Some(NodePositionUpdate {
        position: XYPosition::new(
            position_absolute.x - parent_position.x + dimensions.width * origin[0],
            position_absolute.y - parent_position.y + dimensions.height * origin[1],
        ),
        position_absolute,
    })
}

pub fn calculate_snap_offset(
    drag_items: &BTreeMap<String, NodeDragItem>,
    snap_grid: SnapGrid,
    x: f64,
    y: f64,
) -> Option<XYPosition> {
    let reference = drag_items.values().next()?;
    let reference_position = XYPosition::new(x - reference.distance.x, y - reference.distance.y);
    let snapped = snap_position(reference_position, snap_grid);
    Some(XYPosition::new(
        snapped.x - reference_position.x,
        snapped.y - reference_position.y,
    ))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DragUpdate {
    pub changed: bool,
    pub items: BTreeMap<String, NodeDragItem>,
    pub positions: BTreeMap<String, NodePositionUpdate>,
    pub parent_expansions: Vec<(String, String, crate::core::geometry::Rect)>,
    pub single_node_position: Option<XYPosition>,
}

pub struct UpdateNodesParams<'a> {
    pub pointer: XYPosition,
    pub store: &'a DragStore,
}

pub fn update_nodes(
    params: UpdateNodesParams<'_>,
    drag_items: &mut BTreeMap<String, NodeDragItem>,
) -> DragUpdate {
    let store = params.store;
    let is_multi_drag = drag_items.len() > 1;

    let nodes_box = if is_multi_drag {
        let mut bounds = crate::core::geometry::Box2::EMPTY;
        let mut has_item = false;
        for item in drag_items.values() {
            has_item = true;
            bounds = get_bounds_of_boxes(bounds, drag_item_to_box(item));
        }
        if has_item {
            Some(rect_to_box(box_to_rect(bounds)))
        } else {
            None
        }
    } else {
        None
    };

    let multi_drag_snap_offset = if is_multi_drag && store.snap_to_grid {
        calculate_snap_offset(
            drag_items,
            store.snap_grid,
            params.pointer.x,
            params.pointer.y,
        )
    } else {
        None
    };

    let mut has_change = false;
    let mut positions: BTreeMap<String, NodePositionUpdate> = BTreeMap::new();
    let mut parent_expansions = Vec::new();

    let ids: Vec<String> = drag_items.keys().cloned().collect();
    for id in ids {
        if !store.node_lookup.contains_key(&id) {
            continue;
        }
        let Some(item) = drag_items.get_mut(&id) else {
            continue;
        };

        let mut next_position = XYPosition::new(
            params.pointer.x - item.distance.x,
            params.pointer.y - item.distance.y,
        );

        if store.snap_to_grid {
            next_position = match multi_drag_snap_offset {
                Some(offset) => XYPosition::new(
                    (next_position.x + offset.x).round(),
                    (next_position.y + offset.y).round(),
                ),
                None => snap_position(next_position, store.snap_grid),
            };
        }

        let mut adjusted_extent: Option<CoordinateExtent> = None;

        if is_multi_drag
            && item.extent.is_none()
            && let Some(nodes_box) = nodes_box
        {
            let node_extent = store.node_extent;
            let position_absolute = item.position_absolute;
            let dimensions = item.measured;
            let width = dimensions.width.unwrap_or(0.0);
            let height = dimensions.height.unwrap_or(0.0);

            adjusted_extent = Some([
                [
                    position_absolute.x - nodes_box.x + node_extent[0][0],
                    position_absolute.y - nodes_box.y + node_extent[0][1],
                ],
                [
                    position_absolute.x + width - nodes_box.x2 + node_extent[1][0],
                    position_absolute.y + height - nodes_box.y2 + node_extent[1][1],
                ],
            ]);
        }

        let Some(update) = calculate_node_position(CalculateNodePositionParams {
            node_id: &id,
            next_position,
            node_lookup: &store.node_lookup,
            node_origin: store.node_origin,
            node_extent: adjusted_extent
                .unwrap_or(store.node_extent_override.unwrap_or(store.node_extent)),
        }) else {
            continue;
        };

        has_change = has_change
            || item.position.x != update.position.x
            || item.position.y != update.position.y;

        item.position = update.position;
        item.position_absolute = update.position_absolute;

        let node = store.node_lookup.get(&id);
        if let Some(node) = node
            && node.expand_parent
            && let Some(parent_id) = &node.parent_id
        {
            parent_expansions.push((
                id.clone(),
                parent_id.clone(),
                crate::core::geometry::Rect::new(
                    update.position_absolute.x,
                    update.position_absolute.y,
                    item.measured.width.unwrap_or(0.0),
                    item.measured.height.unwrap_or(0.0),
                ),
            ));
        }

        positions.insert(id, update);
    }

    DragUpdate {
        changed: has_change,
        items: drag_items.clone(),
        positions,
        parent_expansions,
        single_node_position: None,
    }
}

pub fn drag_snap_position(position: XYPosition, snap_grid: SnapGrid) -> XYPosition {
    snap_position(position, snap_grid)
}

pub fn auto_pan_for_drag(
    pointer: XYPosition,
    container: Dimensions,
    speed: f64,
    distance: f64,
) -> XYPosition {
    calc_auto_pan(pointer, container, speed, distance)
}

pub fn parent_expansion_rect(item: &NodeDragItem) -> crate::core::geometry::Rect {
    crate::core::geometry::Rect::new(
        item.position_absolute.x,
        item.position_absolute.y,
        item.measured.width.unwrap_or(0.0),
        item.measured.height.unwrap_or(0.0),
    )
}

pub fn snap_offset_for_selection(
    drag_items: &BTreeMap<String, NodeDragItem>,
    snap_grid: SnapGrid,
    pointer: XYPosition,
) -> Option<XYPosition> {
    calculate_snap_offset(drag_items, snap_grid, pointer.x, pointer.y)
}

pub fn is_node_within_parent<S: Clone>(node: &InternalNode<S>, parent: &InternalNode<S>) -> bool {
    let parent_pos = parent.internals.position_absolute;
    let parent_size = parent.size();
    let node_pos = node.internals.position_absolute;
    let node_size = node.size();
    node_pos.x >= parent_pos.x
        && node_pos.y >= parent_pos.y
        && node_pos.x + node_size.width <= parent_pos.x + parent_size.width
        && node_pos.y + node_size.height <= parent_pos.y + parent_size.height
}

pub fn expand_bounds_to_include(
    parent: crate::core::geometry::Rect,
    child: crate::core::geometry::Rect,
) -> crate::core::geometry::Rect {
    get_bounds_of_rects(parent, child)
}

pub fn drag_items_require_transform(transform: Transform) -> bool {
    transform.zoom() != 1.0
}

pub fn node_dimensions_of<S: Clone>(node: &Node<S>) -> Dimensions {
    get_node_dimensions(node)
}

pub fn should_skip_child(node: &InternalNode<String>, node_lookup: &NodeLookup) -> bool {
    node.parent_id.is_some() && is_parent_selected(node, node_lookup)
}

pub fn has_selector(target_classes: &[String], selector: &str, ancestors: &[Vec<String>]) -> bool {
    let selector = selector.trim_start_matches('.');
    if selector.is_empty() {
        return false;
    }
    if target_classes.iter().any(|class| class == selector) {
        return true;
    }
    ancestors
        .iter()
        .any(|classes| classes.iter().any(|class| class == selector))
}

pub fn get_event_handler_params<S: Clone>(
    node_id: Option<&str>,
    drag_items: &BTreeMap<String, NodeDragItem>,
    node_lookup: &NodeLookup<S>,
    dragging: bool,
) -> (Option<Node<S>>, Vec<Node<S>>) {
    let mut nodes_from_drag_items: Vec<Node<S>> = Vec::new();

    for (id, drag_item) in drag_items {
        if let Some(node) = node_lookup.get(id) {
            let mut user_node = node.internals.user_node.clone();
            user_node.position = drag_item.position;
            user_node.dragging = dragging;
            nodes_from_drag_items.push(user_node);
        }
    }

    let Some(node_id) = node_id else {
        return (
            nodes_from_drag_items.first().cloned(),
            nodes_from_drag_items,
        );
    };

    let first = match node_lookup.get(node_id) {
        Some(node) => {
            let mut user_node = node.internals.user_node.clone();
            user_node.position = drag_items
                .get(node_id)
                .map(|item| item.position)
                .unwrap_or(node.position);
            user_node.dragging = dragging;
            Some(user_node)
        }
        None => nodes_from_drag_items.first().cloned(),
    };

    (first, nodes_from_drag_items)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragPhase {
    Idle,
    Pressed,
    Dragging,
}

#[derive(Clone, Debug, PartialEq)]
pub struct XYDragOptions {
    pub node_drag_threshold: f64,
    pub no_drag_class_name: Option<String>,
    pub handle_selector: Option<String>,
    pub auto_pan_on_node_drag: bool,
    pub auto_pan_speed: f64,
    pub snap_to_grid: bool,
    pub snap_grid: crate::core::geometry::SnapGrid,
    pub nodes_draggable: bool,
}

impl Default for XYDragOptions {
    fn default() -> Self {
        XYDragOptions {
            node_drag_threshold: 1.0,
            no_drag_class_name: None,
            handle_selector: None,
            auto_pan_on_node_drag: true,
            auto_pan_speed: 15.0,
            snap_to_grid: false,
            snap_grid: [1.0, 1.0],
            nodes_draggable: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct XYDrag {
    options: XYDragOptions,
    phase: DragPhase,
    press_pointer: Option<XYPosition>,
    drag_items: BTreeMap<String, NodeDragItem>,
    active_node_id: Option<String>,
    started: bool,
}

impl Default for XYDrag {
    fn default() -> Self {
        Self::new(XYDragOptions::default())
    }
}

impl XYDrag {
    pub fn new(options: XYDragOptions) -> Self {
        XYDrag {
            options,
            phase: DragPhase::Idle,
            press_pointer: None,
            drag_items: BTreeMap::new(),
            active_node_id: None,
            started: false,
        }
    }

    pub fn options(&self) -> &XYDragOptions {
        &self.options
    }

    pub fn set_options(&mut self, options: XYDragOptions) {
        self.options = options;
    }

    pub fn phase(&self) -> DragPhase {
        self.phase
    }

    pub fn drag_items(&self) -> &BTreeMap<String, NodeDragItem> {
        &self.drag_items
    }

    pub fn has_started(&self) -> bool {
        self.started
    }

    pub fn active_node_id(&self) -> Option<&str> {
        self.active_node_id.as_deref()
    }

    pub fn can_start(&self, target_classes: &[String], ancestors: &[Vec<String>]) -> bool {
        if let Some(class_name) = &self.options.no_drag_class_name
            && has_selector(target_classes, class_name, ancestors)
        {
            return false;
        }
        if let Some(selector) = &self.options.handle_selector {
            return has_selector(target_classes, selector, ancestors);
        }
        true
    }

    pub fn on_pointer_down<S: Clone>(
        &mut self,
        pointer: XYPosition,
        node_id: Option<&str>,
        node_lookup: &NodeLookup<S>,
    ) -> bool {
        self.drag_items = crate::core::store::get_drag_items(
            node_lookup,
            self.options.nodes_draggable,
            pointer,
            node_id,
        );
        if self.drag_items.is_empty() {
            self.phase = DragPhase::Idle;
            self.press_pointer = None;
            self.started = false;
            return false;
        }
        self.phase = DragPhase::Pressed;
        self.press_pointer = Some(pointer);
        self.active_node_id = node_id.map(|id| id.to_string());
        self.started = false;
        true
    }

    pub fn on_pointer_move<S: Clone>(
        &mut self,
        pointer: XYPosition,
        node_lookup: &NodeLookup<S>,
        mut calculate: impl FnMut(&mut NodeDragItem, &NodeLookup<S>),
    ) -> DragUpdate {
        if self.phase == DragPhase::Idle {
            return DragUpdate::default();
        }

        if let Some(start) = self.press_pointer {
            let distance = ((pointer.x - start.x).powi(2) + (pointer.y - start.y).powi(2)).sqrt();
            if distance < self.options.node_drag_threshold {
                return DragUpdate::default();
            }
        }

        self.phase = DragPhase::Dragging;
        self.started = true;

        for item in self.drag_items.values_mut() {
            calculate(item, node_lookup);
        }

        let mut positions: BTreeMap<String, NodePositionUpdate> = BTreeMap::new();
        let mut parent_expansions: Vec<(String, String, crate::core::geometry::Rect)> = Vec::new();
        for item in self.drag_items.values() {
            positions.insert(
                item.id.clone(),
                NodePositionUpdate {
                    position: item.position,
                    position_absolute: item.position_absolute,
                },
            );
            if item.expand_parent
                && let Some(parent_id) = item.parent_id.clone()
            {
                parent_expansions.push((item.id.clone(), parent_id, parent_expansion_rect(item)));
            }
        }

        let changed = !positions.is_empty();
        let single_node_position = self
            .active_node_id
            .as_ref()
            .and_then(|id| self.drag_items.get(id))
            .map(|item| item.position);

        DragUpdate {
            changed,
            items: self.drag_items.clone(),
            positions,
            parent_expansions,
            single_node_position,
        }
    }

    pub fn on_pointer_up(&mut self) -> (bool, Vec<NodeDragItem>) {
        let dragged = self.started;
        let items: Vec<NodeDragItem> = self.drag_items.values().cloned().collect();
        self.phase = DragPhase::Idle;
        self.press_pointer = None;
        self.started = false;
        self.active_node_id = None;
        self.drag_items.clear();
        (dragged, items)
    }

    pub fn cancel(&mut self) {
        self.phase = DragPhase::Idle;
        self.press_pointer = None;
        self.started = false;
        self.active_node_id = None;
        self.drag_items.clear();
    }
}
