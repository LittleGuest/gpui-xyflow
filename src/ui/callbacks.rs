use std::fmt;
use std::sync::Arc;

use crate::core::change::{EdgeChange, NodeChange};
use crate::core::connect::ConnectionInProgress;
use crate::core::edge::Edge;
use crate::core::node::Node;

pub type NodeChangeHandler<S> = Arc<dyn Fn(&[NodeChange<S>]) + Send + Sync>;
pub type EdgeChangeHandler<S> = Arc<dyn Fn(&[EdgeChange<S>]) + Send + Sync>;
pub type NodesHandler<S> = Arc<dyn Fn(&[Node<S>]) + Send + Sync>;
pub type EdgeHandler<S> = Arc<dyn Fn(&Edge<S>) + Send + Sync>;
pub type EdgesHandler<S> = Arc<dyn Fn(&[Edge<S>]) + Send + Sync>;
pub type ConnectHandler<S> = Arc<dyn Fn(&ConnectionInProgress<S>) + Send + Sync>;
pub type DeleteHandler<S> = Arc<dyn Fn(&[Node<S>], &[Edge<S>]) + Send + Sync>;
pub type ViewportHandler = Arc<dyn Fn(crate::core::geometry::Viewport) + Send + Sync>;
pub type TransformHandler = Arc<dyn Fn(crate::core::geometry::Transform) + Send + Sync>;
pub type RectHandler = Arc<dyn Fn(&crate::core::geometry::Rect) + Send + Sync>;
pub type UnitHandler = Arc<dyn Fn() + Send + Sync>;
pub type ErrorHandler = Arc<dyn Fn(&str) + Send + Sync>;

macro_rules! callback_table {
    ($name:ident, $($field:ident : $ty:ty),* $(,)?) => {
        #[derive(Clone)]
        pub struct $name<S = String> {
            $(pub $field: Option<$ty>,)*
            marker: std::marker::PhantomData<S>,
        }

        impl<S> Default for $name<S> {
            fn default() -> Self {
                Self {
                    $($field: None,)*
                    marker: std::marker::PhantomData,
                }
            }
        }

        impl<S> fmt::Debug for $name<S> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }

        impl<S> $name<S> {
            pub fn empty() -> Self {
                Self::default()
            }

            pub fn is_empty(&self) -> bool {
                $(self.$field.is_none() &&)* true
            }
        }
    };
}

callback_table!(
    FlowCallbacks,
    on_nodes_change: NodeChangeHandler<S>,
    on_edges_change: EdgeChangeHandler<S>,
    on_node_drag_start: NodesHandler<S>,
    on_node_drag: NodesHandler<S>,
    on_node_drag_stop: NodesHandler<S>,
    on_selection_drag_start: NodesHandler<S>,
    on_selection_drag: NodesHandler<S>,
    on_selection_drag_stop: NodesHandler<S>,
    on_move_start: ViewportHandler,
    on_move: ViewportHandler,
    on_move_end: ViewportHandler,
    on_viewport_change_start: TransformHandler,
    on_viewport_change: TransformHandler,
    on_viewport_change_end: TransformHandler,
    on_connect: ConnectHandler<S>,
    on_connect_start: ConnectHandler<S>,
    on_connect_end: ConnectHandler<S>,
    on_click_connect_start: ConnectHandler<S>,
    on_click_connect_end: ConnectHandler<S>,
    on_nodes_delete: NodesHandler<S>,
    on_edges_delete: EdgesHandler<S>,
    on_delete: DeleteHandler<S>,
    on_edge_click: EdgeHandler<S>,
    on_edge_double_click: EdgeHandler<S>,
    on_edge_context_menu: EdgeHandler<S>,
    on_edge_mouse_enter: EdgeHandler<S>,
    on_edge_mouse_move: EdgeHandler<S>,
    on_edge_mouse_leave: EdgeHandler<S>,
    on_pane_click: UnitHandler,
    on_pane_context_menu: UnitHandler,
    on_pane_mouse_enter: UnitHandler,
    on_pane_mouse_move: UnitHandler,
    on_pane_mouse_leave: UnitHandler,
    on_selection_change: UnitHandler,
    on_selection_start: RectHandler,
    on_selection_end: RectHandler,
    on_init: UnitHandler,
    on_error: ErrorHandler,
);
