use std::cell::Cell;
use std::collections::BTreeMap;

use crate::core::change::{NodeChange, NodeDimensionChange, NodePositionChange, SetAttributes};
use crate::core::constants::{
    DEFAULT_NODE_ORIGIN, INFINITE_EXTENT, ROOT_PARENT_Z_INCREMENT, SELECTED_NODE_Z,
};
use crate::core::geometry::{
    CoordinateExtent, Dimensions, NodeOrigin, Rect, XYPosition, ZIndexMode,
};
use crate::core::node::{
    bounds_from_handles, Handle, HandleType, InternalNode, Node, NodeDragItem, NodeHandleBounds,
    NodeLookup, ParentLookup,
};
use crate::core::utils::{
    box_to_rect, clamp_position, clamp_position_to_parent, get_bounds_of_boxes,
    get_bounds_of_rects, get_node_dimensions, get_rects_overlapping_area, internal_node_to_box,
    is_numeric, node_to_rect,
};

#[derive(Clone, Debug, PartialEq)]
pub struct AdoptOptions {
    pub node_origin: NodeOrigin,
    pub node_extent: CoordinateExtent,
    pub elevate_nodes_on_select: bool,
    pub z_index_mode: ZIndexMode,
    pub check_equality: bool,
}

impl Default for AdoptOptions {
    fn default() -> Self {
        AdoptOptions {
            node_origin: DEFAULT_NODE_ORIGIN,
            node_extent: INFINITE_EXTENT,
            elevate_nodes_on_select: true,
            z_index_mode: ZIndexMode::Basic,
            check_equality: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AdoptResult {
    pub nodes_initialized: bool,
    pub has_selected_nodes: bool,
}

pub fn parse_handles<S: Clone>(
    user_node: &Node<S>,
    existing: Option<&InternalNode<S>>,
) -> Option<NodeHandleBounds> {
    if user_node.handles.is_empty() {
        return if user_node.measured.is_measured() {
            existing.and_then(|node| node.internals.handle_bounds.clone())
        } else {
            None
        };
    }

    let mut bounds = bounds_from_handles(&user_node.handles);
    for handle in bounds.source.iter_mut().chain(bounds.target.iter_mut()) {
        if handle.width <= 0.0 {
            handle.width = 1.0;
        }
        if handle.height <= 0.0 {
            handle.height = 1.0;
        }
    }
    Some(bounds)
}

pub fn calculate_z<S: Clone>(node: &Node<S>, selected_node_z: f64, z_index_mode: ZIndexMode) -> f64 {
    let z_index = node
        .z_index
        .filter(|value| is_numeric(*value))
        .unwrap_or(0.0);
    if z_index_mode.is_manual() {
        return z_index;
    }
    z_index + if node.selected { selected_node_z } else { 0.0 }
}

pub fn adopt_user_nodes<S: Clone + PartialEq>(
    nodes: &[Node<S>],
    node_lookup: &mut NodeLookup<S>,
    parent_lookup: &mut ParentLookup<S>,
    options: &AdoptOptions,
) -> AdoptResult {
    let root_parent_index = Cell::new(0usize);
    let previous = node_lookup.clone();
    let selected_node_z = if options.elevate_nodes_on_select && !options.z_index_mode.is_manual() {
        SELECTED_NODE_Z
    } else {
        0.0
    };

    let mut nodes_initialized = !nodes.is_empty();
    let mut has_selected_nodes = false;

    node_lookup.clear();
    parent_lookup.clear();

    for user_node in nodes {
        let existing = previous.get(&user_node.id);
        let previous_is_same = options.check_equality
            && matches!(existing, Some(node) if node.internals.user_node.id == user_node.id
                && node.internals.user_node.position == user_node.position
                && node.internals.user_node.measured == user_node.measured);

        if previous_is_same {
            if let Some(node) = existing {
                node_lookup.insert(user_node.id.clone(), node.clone());
            }
        } else {
            let position_with_origin = user_node.position_with_origin(options.node_origin);
            let extent = user_node
                .extent
                .and_then(|extent| extent.coordinates())
                .unwrap_or(options.node_extent);
            let clamped = clamp_position(
                position_with_origin,
                extent,
                get_node_dimensions(user_node),
            );
            let handle_bounds = parse_handles(user_node, existing);
            let z = calculate_z(user_node, selected_node_z, options.z_index_mode);
            let internal =
                InternalNode::from_user_node(user_node.clone(), clamped, z, handle_bounds);
            node_lookup.insert(user_node.id.clone(), internal);
        }

        if let Some(internal) = node_lookup.get(&user_node.id) {
            if !internal.measured.is_measured() && !internal.hidden {
                nodes_initialized = false;
            }
        }

        if user_node.parent_id.is_some() {
            update_child_node(
                user_node.id.clone(),
                node_lookup,
                parent_lookup,
                options,
                Some(&root_parent_index),
            );
        }

        has_selected_nodes |= user_node.selected;
    }

    AdoptResult {
        nodes_initialized,
        has_selected_nodes,
    }
}

pub fn update_absolute_positions<S: Clone>(
    node_lookup: &mut NodeLookup<S>,
    parent_lookup: &mut ParentLookup<S>,
    options: &AdoptOptions,
) {
    let ids: Vec<String> = node_lookup.keys().cloned().collect();
    for id in ids {
        let Some(node) = node_lookup.get(&id).cloned() else {
            continue;
        };
        if node.parent_id.is_some() {
            update_child_node(id, node_lookup, parent_lookup, options, None);
        } else {
            let position_with_origin = node.position_with_origin(options.node_origin);
            let extent = node
                .extent
                .and_then(|extent| extent.coordinates())
                .unwrap_or(options.node_extent);
            let clamped = clamp_position(
                position_with_origin,
                extent,
                get_node_dimensions(&node.internals.user_node),
            );
            if let Some(node) = node_lookup.get_mut(&id) {
                node.internals.position_absolute = clamped;
            }
        }
    }
}

fn update_parent_lookup<S: Clone>(node: &InternalNode<S>, parent_lookup: &mut ParentLookup<S>) {
    let Some(parent_id) = &node.parent_id else {
        return;
    };
    parent_lookup
        .entry(parent_id.clone())
        .or_default()
        .insert(node.id.clone(), node.clone());
}

fn update_child_node<S: Clone>(
    id: String,
    node_lookup: &mut NodeLookup<S>,
    parent_lookup: &mut ParentLookup<S>,
    options: &AdoptOptions,
    root_parent_index: Option<&Cell<usize>>,
) {
    let Some(node) = node_lookup.get(&id).cloned() else {
        return;
    };
    let Some(parent_id) = node.parent_id.clone() else {
        return;
    };
    let Some(parent_node) = node_lookup.get(&parent_id).cloned() else {
        return;
    };

    update_parent_lookup(&node, parent_lookup);

    if let Some(index) = root_parent_index {
        if parent_node.parent_id.is_none()
            && parent_node.internals.root_parent_index.is_none()
            && options.z_index_mode == ZIndexMode::Auto
        {
            let next = index.get() + 1;
            index.set(next);
            if let Some(parent) = node_lookup.get_mut(&parent_id) {
                parent.internals.root_parent_index = Some(next);
                parent.internals.z += next as f64 * ROOT_PARENT_Z_INCREMENT;
            }
        }
        if let Some(root_index) = node_lookup
            .get(&parent_id)
            .and_then(|parent| parent.internals.root_parent_index)
        {
            index.set(root_index);
        }
    }

    let selected_node_z = if options.elevate_nodes_on_select && !options.z_index_mode.is_manual() {
        SELECTED_NODE_Z
    } else {
        0.0
    };

    let parent_node = node_lookup
        .get(&parent_id)
        .cloned()
        .unwrap_or(parent_node);
    let (x, y, z) = calculate_child_xyz(
        &node,
        &parent_node,
        options.node_origin,
        options.node_extent,
        selected_node_z,
        options.z_index_mode,
    );

    let changed = x != node.internals.position_absolute.x
        || y != node.internals.position_absolute.y
        || z != node.internals.z;

    if changed {
        if let Some(node) = node_lookup.get_mut(&id) {
            node.internals.position_absolute = XYPosition::new(x, y);
            node.internals.z = z;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn calculate_child_xyz<S: Clone>(
    child: &InternalNode<S>,
    parent: &InternalNode<S>,
    node_origin: NodeOrigin,
    node_extent: CoordinateExtent,
    selected_node_z: f64,
    z_index_mode: ZIndexMode,
) -> (f64, f64, f64) {
    let parent_position = parent.internals.position_absolute;
    let child_dimensions = child.size();
    let position_with_origin = child.position_with_origin(node_origin);
    let clamped = match child.extent.and_then(|extent| extent.coordinates()) {
        Some(extent) => clamp_position(position_with_origin, extent, child_dimensions),
        None => position_with_origin,
    };

    let mut absolute = clamp_position(
        XYPosition::new(parent_position.x + clamped.x, parent_position.y + clamped.y),
        node_extent,
        child_dimensions,
    );

    if matches!(child.extent, Some(crate::core::node::NodeExtent::Parent)) {
        absolute = clamp_position_to_parent(absolute, child_dimensions, parent);
    }

    let z_index = child
        .z_index
        .filter(|value| is_numeric(*value))
        .unwrap_or(0.0);
    let child_z = if z_index_mode.is_manual() {
        z_index
    } else {
        z_index + if child.selected { selected_node_z } else { 0.0 }
    };
    let parent_z = parent.internals.z;

    (
        absolute.x,
        absolute.y,
        if parent_z >= child_z {
            parent_z + 1.0
        } else {
            child_z
        },
    )
}

pub fn handle_expand_parent<S: Clone>(
    children: &[(String, String, Rect)],
    node_lookup: &NodeLookup<S>,
    parent_lookup: &ParentLookup<S>,
    node_origin: NodeOrigin,
) -> Vec<NodeChange<S>> {
    let mut changes: Vec<NodeChange<S>> = Vec::new();
    let mut expansions: BTreeMap<String, (Rect, InternalNode<S>)> = BTreeMap::new();

    for (_, parent_id, rect) in children {
        let Some(parent) = node_lookup.get(parent_id) else {
            continue;
        };
        let parent_rect = expansions
            .get(parent_id)
            .map(|(expanded, _)| *expanded)
            .unwrap_or_else(|| node_to_rect(&parent.internals.user_node, node_origin));
        let expanded = get_bounds_of_rects(parent_rect, *rect);
        expansions.insert(parent_id.clone(), (expanded, parent.clone()));
    }

    for (parent_id, (expanded_rect, parent)) in expansions {
        let position_absolute = parent.internals.position_absolute;
        let dimensions = parent.size();
        let origin = parent.origin.unwrap_or(node_origin);

        let x_change = if expanded_rect.x < position_absolute.x {
            (position_absolute.x - expanded_rect.x).abs().round()
        } else {
            0.0
        };
        let y_change = if expanded_rect.y < position_absolute.y {
            (position_absolute.y - expanded_rect.y).abs().round()
        } else {
            0.0
        };

        let new_width = dimensions.width.max(expanded_rect.width.round());
        let new_height = dimensions.height.max(expanded_rect.height.round());
        let width_change = (new_width - dimensions.width) * origin[0];
        let height_change = (new_height - dimensions.height) * origin[1];

        if x_change > 0.0 || y_change > 0.0 || width_change != 0.0 || height_change != 0.0 {
            changes.push(NodeChange::Position(NodePositionChange {
                id: parent_id.clone(),
                position: Some(XYPosition::new(
                    parent.position.x - x_change + width_change,
                    parent.position.y - y_change + height_change,
                )),
                position_absolute: None,
                dragging: None,
            }));

            if let Some(child_nodes) = parent_lookup.get(&parent_id) {
                for child in child_nodes.values() {
                    if children.iter().any(|(id, _, _)| id == &child.id) {
                        continue;
                    }
                    changes.push(NodeChange::Position(NodePositionChange {
                        id: child.id.clone(),
                        position: Some(XYPosition::new(
                            child.position.x + x_change,
                            child.position.y + y_change,
                        )),
                        position_absolute: None,
                        dragging: None,
                    }));
                }
            }
        }

        if dimensions.width < expanded_rect.width
            || dimensions.height < expanded_rect.height
            || x_change != 0.0
            || y_change != 0.0
        {
            changes.push(NodeChange::Dimensions(NodeDimensionChange {
                id: parent_id,
                dimensions: Some(Dimensions::new(
                    new_width
                        + if x_change != 0.0 {
                            origin[0] * x_change - width_change
                        } else {
                            0.0
                        },
                    new_height
                        + if y_change != 0.0 {
                            origin[1] * y_change - height_change
                        } else {
                            0.0
                        },
                )),
                resizing: None,
                set_attributes: Some(SetAttributes::Both),
            }));
        }
    }

    changes
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeInternalsUpdate {
    pub id: String,
    pub dimensions: Dimensions,
    pub handle_bounds: Option<NodeHandleBounds>,
    pub force: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeInternalsResult<S> {
    pub changes: Vec<NodeChange<S>>,
    pub updated_internals: bool,
}

pub fn update_node_internals<S: Clone>(
    updates: &[NodeInternalsUpdate],
    node_lookup: &mut NodeLookup<S>,
    parent_lookup: &mut ParentLookup<S>,
    options: &AdoptOptions,
) -> NodeInternalsResult<S> {
    let mut changes: Vec<NodeChange<S>> = Vec::new();
    let mut updated_internals = false;
    let mut parent_expand_children: Vec<(String, String, Rect)> = Vec::new();

    for update in updates {
        let Some(node) = node_lookup.get(&update.id).cloned() else {
            continue;
        };

        if node.hidden {
            if let Some(node) = node_lookup.get_mut(&update.id) {
                node.internals.handle_bounds = None;
            }
            updated_internals = true;
            continue;
        }

        let dimensions = update.dimensions;
        let dimension_changed = node.measured.width != Some(dimensions.width)
            || node.measured.height != Some(dimensions.height);
        let do_update = dimensions.width > 0.0
            && dimensions.height > 0.0
            && (dimension_changed || node.internals.handle_bounds.is_none() || update.force);

        if !do_update {
            continue;
        }

        let extent = node
            .extent
            .and_then(|extent| extent.coordinates())
            .unwrap_or(options.node_extent);
        let mut position_absolute = node.internals.position_absolute;

        if node.parent_id.is_some()
            && matches!(node.extent, Some(crate::core::node::NodeExtent::Parent))
        {
            if let Some(parent) = node
                .parent_id
                .as_ref()
                .and_then(|id| node_lookup.get(id))
                .cloned()
            {
                position_absolute = clamp_position_to_parent(position_absolute, dimensions, &parent);
            }
        } else {
            position_absolute = clamp_position(position_absolute, extent, dimensions);
        }

        let handle_bounds = update
            .handle_bounds
            .clone()
            .or_else(|| node.internals.handle_bounds.clone());

        let mut next = node.clone();
        next.measured = crate::core::node::MeasuredDimensions::new(dimensions.width, dimensions.height);
        next.internals.position_absolute = position_absolute;
        next.internals.handle_bounds = handle_bounds;
        node_lookup.insert(update.id.clone(), next.clone());

        if next.parent_id.is_some() {
            update_child_node(update.id.clone(), node_lookup, parent_lookup, options, None);
        }

        updated_internals = true;

        if dimension_changed {
            changes.push(NodeChange::Dimensions(NodeDimensionChange {
                id: next.id.clone(),
                dimensions: Some(dimensions),
                resizing: None,
                set_attributes: None,
            }));

            if next.expand_parent {
                if let Some(parent_id) = next.parent_id.clone() {
                    let rect = node_to_rect(&next.internals.user_node, options.node_origin);
                    parent_expand_children.push((next.id.clone(), parent_id, rect));
                }
            }
        }
    }

    if !parent_expand_children.is_empty() {
        changes.extend(handle_expand_parent(
            &parent_expand_children,
            node_lookup,
            parent_lookup,
            options.node_origin,
        ));
    }

    NodeInternalsResult {
        changes,
        updated_internals,
    }
}

pub fn get_drag_items<S: Clone>(
    node_lookup: &NodeLookup<S>,
    nodes_draggable: bool,
    pointer: XYPosition,
    node_id: Option<&str>,
) -> BTreeMap<String, NodeDragItem> {
    let mut items = BTreeMap::new();

    for (id, node) in node_lookup {
        if !(node.selected || Some(id.as_str()) == node_id) {
            continue;
        }
        if node.parent_id.is_some() && is_parent_selected(node, node_lookup) {
            continue;
        }
        if !node.draggable.unwrap_or(nodes_draggable) {
            continue;
        }
        items.insert(id.clone(), NodeDragItem::from_internal(node, pointer));
    }

    items
}

pub fn is_parent_selected<S: Clone>(node: &InternalNode<S>, node_lookup: &NodeLookup<S>) -> bool {
    let Some(parent_id) = &node.parent_id else {
        return false;
    };
    let Some(parent) = node_lookup.get(parent_id) else {
        return false;
    };
    if parent.selected {
        return true;
    }
    is_parent_selected(parent, node_lookup)
}

pub fn collect_handles(handles: &[Handle]) -> (Vec<Handle>, Vec<Handle>) {
    let mut source = Vec::new();
    let mut target = Vec::new();
    for handle in handles {
        match handle.handle_type {
            HandleType::Source => source.push(handle.clone()),
            HandleType::Target => target.push(handle.clone()),
        }
    }
    (source, target)
}

pub fn bounds_of_nodes<S: Clone>(nodes: &[InternalNode<S>]) -> Rect {
    let mut bounds = crate::core::geometry::Box2::EMPTY;
    let mut has_node = false;
    for node in nodes {
        has_node = true;
        bounds = get_bounds_of_boxes(bounds, internal_node_to_box(node));
    }
    if has_node {
        box_to_rect(bounds)
    } else {
        Rect::ZERO
    }
}

pub fn overlap_of(a: Rect, b: Rect) -> f64 {
    get_rects_overlapping_area(a, b)
}
