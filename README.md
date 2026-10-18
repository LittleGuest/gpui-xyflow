# gpui-xyflow

xyflow reimplemented in Rust on top of [GPUI Kit](https://crates.io/crates/gpui-kit).

The crate mirrors the structure of [xyflow/xyflow](https://github.com/xyflow/xyflow): a
framework independent graph system (the equivalent of `@xyflow/system`) plus a UI layer
(the equivalent of `@xyflow/react`, React Flow 12).

## Layout

```
src/lib.rs        crate root, wires the two layers together
src/core/         framework independent system layer
src/ui/           flow UI layer built on gpui-kit
examples/         runnable demos
tests/            unit and integration tests
```

`core` and `ui` are declared through `#[path]` modules, so the crate stays a single
package without a Cargo workspace.

### Core

| Module | Mirrors |
| --- | --- |
| `geometry` | `types`, `utils/general` geometry helpers |
| `constants` | `constants.ts` |
| `node` | `types/nodes.ts` |
| `edge` | `types/edges.ts` |
| `edge_path` | `utils/edges/*` |
| `graph` | `utils/graph.ts` |
| `store` | `utils/store.ts` |
| `connect` | `xyhandle/*` |
| `connections` | `utils/connections.ts` |
| `drag` | `xydrag/*` |
| `panzoom` | `xypanzoom/*` |
| `xyminimap` | `xyminimap/*` |
| `xypanzoom` | `xypanzoom/*` |
| `xyresizer` | `xyresizer/*` |
| `change` | `types/changes.ts` |
| `utils` | `utils/general.ts` |

### UI

| Module | Mirrors |
| --- | --- |
| `flow` | `<ReactFlow />` |
| `store` | `store/index.ts`, `types/component-props.ts` |
| `pane` | `container/Pane` |
| `viewport` | `container/Viewport` |
| `node_renderer` | `components/NodeWrapper`, `nodes/*` |
| `edge_renderer` | `components/EdgeWrapper`, `edges/*` |
| `handle` | `components/Handle` |
| `connections` | `components/ConnectionLine` |
| `marker` | `components/Edges/MarkerDefinitions` |
| `background` | `additional-components/Background` |
| `minimap` | `additional-components/MiniMap` |
| `controls` | `additional-components/Controls` |
| `panel` | `components/Panel` |
| `resizer` | `additional-components/NodeResizer` |
| `theme` | React Flow's `style.css` design tokens |
| `paths` | SVG path parsing helpers |
| `paint` | SVG path to GPUI `Path` bridging |

## Running the demos

```
cargo run --example flow_gallery
cargo run --example excalidraw_example
```

Both open a real window with pan, zoom, node dragging, connection dragging and
Shift-drag rubber band selection.

## Usage

```rust
use gpui_kit::*;
use gpui_xyflow::core::edge::Edge;
use gpui_xyflow::core::geometry::XYPosition;
use gpui_xyflow::core::node::Node;
use gpui_xyflow::ui::flow::Flow;
use gpui_xyflow::ui::store::FlowProps;

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);

        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|cx| {
                let mut flow = Flow::with_props(FlowProps::default(), cx);
                flow.set_nodes(vec![
                    Node::new("a", XYPosition::new(100.0, 100.0), "A".to_string())
                        .with_size(150.0, 50.0),
                    Node::new("b", XYPosition::new(400.0, 300.0), "B".to_string())
                        .with_size(150.0, 50.0),
                ]);
                flow.set_edges(vec![Edge::default()]);
                flow
            })
        })
        .expect("failed to open window");
    });
}
```

The core layer can also be used on its own:

```rust
use gpui_xyflow::core::*;

let graphics = get_bezier_path(
    0.0,
    0.0,
    Position::Bottom,
    100.0,
    200.0,
    Position::Top,
    BezierPathOptions::default(),
);

assert_eq!(graphics.path, "M0,0 C0,100 100,100 100,200");
```

## Tests

```
cargo test
```

The suite covers edge path generation, coordinate transforms, viewport fitting, node
adoption, z-index handling, connection validity, drag computation, minimap layout,
selection semantics and headless UI rendering.

## License

MIT
