use crate::core::geometry::{CoordinateExtent, NodeOrigin, XYPosition};

pub const RESIZER_HANDLE_POSITIONS: [&str; 4] =
    ["top-left", "top-right", "bottom-left", "bottom-right"];
pub const RESIZER_LINE_POSITIONS: [&str; 4] = ["top", "right", "bottom", "left"];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResizeParams {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ResizeParams {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        ResizeParams {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResizeParamsWithDirection {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub direction: [f64; 2],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Boundaries {
    pub min_width: f64,
    pub min_height: f64,
    pub max_width: f64,
    pub max_height: f64,
}

impl Boundaries {
    pub fn new(min_width: f64, min_height: f64, max_width: f64, max_height: f64) -> Self {
        Boundaries {
            min_width,
            min_height,
            max_width,
            max_height,
        }
    }

    pub fn with_defaults(min_width: f64, min_height: f64) -> Self {
        Boundaries {
            min_width,
            min_height,
            max_width: f64::MAX,
            max_height: f64::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControlDirection {
    pub is_horizontal: bool,
    pub is_vertical: bool,
    pub affects_x: bool,
    pub affects_y: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResizeStartValues {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    pub pointer_x: f64,
    pub pointer_y: f64,
    pub aspect_ratio: f64,
}

impl ResizeStartValues {
    pub fn from_params(params: ResizeParams, pointer: XYPosition) -> Self {
        ResizeStartValues {
            width: params.width,
            height: params.height,
            x: params.x,
            y: params.y,
            pointer_x: pointer.x,
            pointer_y: pointer.y,
            aspect_ratio: if params.height == 0.0 {
                1.0
            } else {
                params.width / params.height
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResizeDimensions {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
}

pub fn get_resize_direction(
    width: f64,
    prev_width: f64,
    height: f64,
    prev_height: f64,
    affects_x: bool,
    affects_y: bool,
) -> [f64; 2] {
    let delta_width = width - prev_width;
    let delta_height = height - prev_height;

    let mut direction = [
        if delta_width > 0.0 {
            1.0
        } else if delta_width < 0.0 {
            -1.0
        } else {
            0.0
        },
        if delta_height > 0.0 {
            1.0
        } else if delta_height < 0.0 {
            -1.0
        } else {
            0.0
        },
    ];

    if delta_width != 0.0 && affects_x {
        direction[0] = -direction[0];
    }

    if delta_height != 0.0 && affects_y {
        direction[1] = -direction[1];
    }

    direction
}

pub fn get_control_direction(control_position: &str) -> ControlDirection {
    let parts: Vec<&str> = control_position.split('-').collect();

    ControlDirection {
        is_horizontal: parts.contains(&"left") || parts.contains(&"right"),
        is_vertical: parts.contains(&"top") || parts.contains(&"bottom"),
        affects_x: parts.contains(&"left"),
        affects_y: parts.contains(&"top"),
    }
}

fn get_lower_extent_clamp(lower_extent: f64, lower_bound: f64) -> f64 {
    (lower_bound - lower_extent).max(0.0)
}

fn get_upper_extent_clamp(upper_extent: f64, upper_bound: f64) -> f64 {
    (upper_extent - upper_bound).max(0.0)
}

fn get_size_clamp(size: f64, min_size: f64, max_size: f64) -> f64 {
    (min_size - size).max(size - max_size).max(0.0)
}

fn xor(a: bool, b: bool) -> bool {
    if a { !b } else { b }
}

#[allow(clippy::too_many_arguments)]
pub fn get_dimensions_after_resize(
    start: ResizeStartValues,
    control: ControlDirection,
    pointer: XYPosition,
    boundaries: Boundaries,
    keep_aspect_ratio: bool,
    node_origin: NodeOrigin,
    extent: Option<CoordinateExtent>,
    child_extent: Option<CoordinateExtent>,
) -> ResizeDimensions {
    let mut affects_x = control.affects_x;
    let mut affects_y = control.affects_y;
    let is_horizontal = control.is_horizontal;
    let is_vertical = control.is_vertical;
    let is_diagonal = is_horizontal && is_vertical;

    let start_x = start.x;
    let start_y = start.y;
    let start_width = start.width;
    let start_height = start.height;
    let aspect_ratio = start.aspect_ratio;

    let mut dist_x = if is_horizontal {
        (pointer.x - start.pointer_x).floor()
    } else {
        0.0
    };
    let mut dist_y = if is_vertical {
        (pointer.y - start.pointer_y).floor()
    } else {
        0.0
    };

    let new_width = start_width + if affects_x { -dist_x } else { dist_x };
    let new_height = start_height + if affects_y { -dist_y } else { dist_y };

    let origin_offset_x = -node_origin[0] * start_width;
    let origin_offset_y = -node_origin[1] * start_height;

    let mut clamp_x = get_size_clamp(new_width, boundaries.min_width, boundaries.max_width);
    let mut clamp_y = get_size_clamp(new_height, boundaries.min_height, boundaries.max_height);

    if let Some(extent) = extent {
        let mut x_extent_clamp = 0.0;
        let mut y_extent_clamp = 0.0;

        if affects_x && dist_x < 0.0 {
            x_extent_clamp =
                get_lower_extent_clamp(start_x + dist_x + origin_offset_x, extent[0][0]);
        } else if !affects_x && dist_x > 0.0 {
            x_extent_clamp =
                get_upper_extent_clamp(start_x + new_width + origin_offset_x, extent[1][0]);
        }

        if affects_y && dist_y < 0.0 {
            y_extent_clamp =
                get_lower_extent_clamp(start_y + dist_y + origin_offset_y, extent[0][1]);
        } else if !affects_y && dist_y > 0.0 {
            y_extent_clamp =
                get_upper_extent_clamp(start_y + new_height + origin_offset_y, extent[1][1]);
        }

        clamp_x = clamp_x.max(x_extent_clamp);
        clamp_y = clamp_y.max(y_extent_clamp);
    }

    if let Some(child_extent) = child_extent {
        let mut x_extent_clamp = 0.0;
        let mut y_extent_clamp = 0.0;

        if affects_x && dist_x > 0.0 {
            x_extent_clamp = get_upper_extent_clamp(start_x + dist_x, child_extent[0][0]);
        } else if !affects_x && dist_x < 0.0 {
            x_extent_clamp = get_lower_extent_clamp(start_x + new_width, child_extent[1][0]);
        }

        if affects_y && dist_y > 0.0 {
            y_extent_clamp = get_upper_extent_clamp(start_y + dist_y, child_extent[0][1]);
        } else if !affects_y && dist_y < 0.0 {
            y_extent_clamp = get_lower_extent_clamp(start_y + new_height, child_extent[1][1]);
        }

        clamp_x = clamp_x.max(x_extent_clamp);
        clamp_y = clamp_y.max(y_extent_clamp);
    }

    if keep_aspect_ratio {
        if is_horizontal {
            let aspect_height_clamp = get_size_clamp(
                new_width / aspect_ratio,
                boundaries.min_height,
                boundaries.max_height,
            ) * aspect_ratio;
            clamp_x = clamp_x.max(aspect_height_clamp);

            if let Some(extent) = extent {
                let aspect_extent_clamp =
                    if (!affects_x && !affects_y) || (affects_x && !affects_y && is_diagonal) {
                        get_upper_extent_clamp(
                            start_y + origin_offset_y + new_width / aspect_ratio,
                            extent[1][1],
                        ) * aspect_ratio
                    } else {
                        get_lower_extent_clamp(
                            start_y
                                + origin_offset_y
                                + (if affects_x { dist_x } else { -dist_x }) / aspect_ratio,
                            extent[0][1],
                        ) * aspect_ratio
                    };
                clamp_x = clamp_x.max(aspect_extent_clamp);
            }

            if let Some(child_extent) = child_extent {
                let aspect_extent_clamp = if (!affects_x && !affects_y)
                    || (affects_x && !affects_y && is_diagonal)
                {
                    get_lower_extent_clamp(start_y + new_width / aspect_ratio, child_extent[1][1])
                        * aspect_ratio
                } else {
                    get_upper_extent_clamp(
                        start_y + (if affects_x { dist_x } else { -dist_x }) / aspect_ratio,
                        child_extent[0][1],
                    ) * aspect_ratio
                };
                clamp_x = clamp_x.max(aspect_extent_clamp);
            }
        }

        if is_vertical {
            let aspect_width_clamp = get_size_clamp(
                new_height * aspect_ratio,
                boundaries.min_width,
                boundaries.max_width,
            ) / aspect_ratio;
            clamp_y = clamp_y.max(aspect_width_clamp);

            if let Some(extent) = extent {
                let aspect_extent_clamp =
                    if (!affects_x && !affects_y) || (affects_y && !affects_x && is_diagonal) {
                        get_upper_extent_clamp(
                            start_x + new_height * aspect_ratio + origin_offset_x,
                            extent[1][0],
                        ) / aspect_ratio
                    } else {
                        get_lower_extent_clamp(
                            start_x
                                + (if affects_y { dist_y } else { -dist_y }) * aspect_ratio
                                + origin_offset_x,
                            extent[0][0],
                        ) / aspect_ratio
                    };
                clamp_y = clamp_y.max(aspect_extent_clamp);
            }

            if let Some(child_extent) = child_extent {
                let aspect_extent_clamp = if (!affects_x && !affects_y)
                    || (affects_y && !affects_x && is_diagonal)
                {
                    get_lower_extent_clamp(start_x + new_height * aspect_ratio, child_extent[1][0])
                        / aspect_ratio
                } else {
                    get_upper_extent_clamp(
                        start_x + (if affects_y { dist_y } else { -dist_y }) * aspect_ratio,
                        child_extent[0][0],
                    ) / aspect_ratio
                };
                clamp_y = clamp_y.max(aspect_extent_clamp);
            }
        }
    }

    dist_y += if dist_y < 0.0 { clamp_y } else { -clamp_y };
    dist_x += if dist_x < 0.0 { clamp_x } else { -clamp_x };

    if keep_aspect_ratio {
        if is_diagonal {
            if new_width > new_height * aspect_ratio {
                dist_y = (if xor(affects_x, affects_y) {
                    -dist_x
                } else {
                    dist_x
                }) / aspect_ratio;
            } else {
                dist_x = (if xor(affects_x, affects_y) {
                    -dist_y
                } else {
                    dist_y
                }) * aspect_ratio;
            }
        } else if is_horizontal {
            dist_y = dist_x / aspect_ratio;
            affects_y = affects_x;
        } else {
            dist_x = dist_y * aspect_ratio;
            affects_x = affects_y;
        }
    }

    let x = if affects_x { start_x + dist_x } else { start_x };
    let y = if affects_y { start_y + dist_y } else { start_y };

    ResizeDimensions {
        width: start_width + if affects_x { -dist_x } else { dist_x },
        height: start_height + if affects_y { -dist_y } else { dist_y },
        x: node_origin[0] * dist_x * (if !affects_x { 1.0 } else { -1.0 }) + x,
        y: node_origin[1] * dist_y * (if !affects_y { 1.0 } else { -1.0 }) + y,
    }
}

pub fn node_to_child_extent(
    child_position: XYPosition,
    child_size: (f64, f64),
    parent_position: XYPosition,
    node_origin: NodeOrigin,
) -> CoordinateExtent {
    let x = parent_position.x + child_position.x;
    let y = parent_position.y + child_position.y;
    let (width, height) = child_size;
    let origin_offset_x = node_origin[0] * width;
    let origin_offset_y = node_origin[1] * height;

    [
        [x - origin_offset_x, y - origin_offset_y],
        [x + width - origin_offset_x, y + height - origin_offset_y],
    ]
}

pub fn is_handle_position(position: &str) -> bool {
    RESIZER_HANDLE_POSITIONS.contains(&position)
}

pub fn is_line_position(position: &str) -> bool {
    RESIZER_LINE_POSITIONS.contains(&position)
}

#[derive(Clone, Debug, PartialEq)]
pub struct XYResizerOptions {
    pub keep_aspect_ratio: bool,
    pub node_origin: NodeOrigin,
    pub extent: Option<CoordinateExtent>,
    pub child_extent: Option<CoordinateExtent>,
    pub boundaries: Boundaries,
    pub snap_to_grid: bool,
    pub snap_grid: crate::core::geometry::SnapGrid,
}

impl Default for XYResizerOptions {
    fn default() -> Self {
        XYResizerOptions {
            keep_aspect_ratio: false,
            node_origin: crate::core::constants::DEFAULT_NODE_ORIGIN,
            extent: None,
            child_extent: None,
            boundaries: Boundaries::with_defaults(0.0, 0.0),
            snap_to_grid: false,
            snap_grid: [1.0, 1.0],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizePhase {
    Idle,
    Resizing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct XYResizerChange {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub direction: [f64; 2],
}

#[derive(Clone, Debug)]
pub struct XYResizer {
    options: XYResizerOptions,
    phase: ResizePhase,
    start: Option<ResizeStartValues>,
    control: Option<ControlDirection>,
    control_position: Option<String>,
    last: Option<XYResizerChange>,
    child_changes: Vec<(String, XYResizerChange)>,
}

impl Default for XYResizer {
    fn default() -> Self {
        Self::new(XYResizerOptions::default())
    }
}

impl XYResizer {
    pub fn new(options: XYResizerOptions) -> Self {
        XYResizer {
            options,
            phase: ResizePhase::Idle,
            start: None,
            control: None,
            control_position: None,
            last: None,
            child_changes: Vec::new(),
        }
    }

    pub fn options(&self) -> &XYResizerOptions {
        &self.options
    }

    pub fn set_options(&mut self, options: XYResizerOptions) {
        self.options = options;
    }

    pub fn phase(&self) -> ResizePhase {
        self.phase
    }

    pub fn is_resizing(&self) -> bool {
        self.phase == ResizePhase::Resizing
    }

    pub fn last(&self) -> Option<&XYResizerChange> {
        self.last.as_ref()
    }

    pub fn child_changes(&self) -> &[(String, XYResizerChange)] {
        &self.child_changes
    }

    pub fn start(&mut self, control_position: &str, params: ResizeParams, pointer: XYPosition) {
        self.phase = ResizePhase::Resizing;
        self.start = Some(ResizeStartValues::from_params(params, pointer));
        self.control = Some(get_control_direction(control_position));
        self.control_position = Some(control_position.to_string());
        self.last = None;
        self.child_changes.clear();
    }

    pub fn resize(&mut self, pointer: XYPosition) -> Option<XYResizerChange> {
        let start = self.start?;
        let control = self.control?;
        if self.phase != ResizePhase::Resizing {
            return None;
        }

        let pointer = if self.options.snap_to_grid {
            XYPosition::new(
                (pointer.x / self.options.snap_grid[0]).round() * self.options.snap_grid[0],
                (pointer.y / self.options.snap_grid[1]).round() * self.options.snap_grid[1],
            )
        } else {
            pointer
        };

        let dimensions = get_dimensions_after_resize(
            start,
            control,
            pointer,
            self.options.boundaries,
            self.options.keep_aspect_ratio,
            self.options.node_origin,
            self.options.extent,
            self.options.child_extent,
        );

        let direction = get_resize_direction(
            dimensions.width,
            start.width,
            dimensions.height,
            start.height,
            control.affects_x,
            control.affects_y,
        );

        let change = XYResizerChange {
            x: dimensions.x,
            y: dimensions.y,
            width: dimensions.width,
            height: dimensions.height,
            direction,
        };
        self.last = Some(change.clone());
        Some(change)
    }

    pub fn end(&mut self) -> Option<XYResizerChange> {
        let last = self.last.take();
        self.phase = ResizePhase::Idle;
        self.start = None;
        self.control = None;
        self.control_position = None;
        self.child_changes.clear();
        last
    }

    pub fn cancel(&mut self) {
        self.phase = ResizePhase::Idle;
        self.start = None;
        self.control = None;
        self.control_position = None;
        self.last = None;
        self.child_changes.clear();
    }

    pub fn set_child_extents(&mut self, children: Vec<(String, ResizeParams)>) {
        self.child_changes = children
            .into_iter()
            .map(|(id, params)| {
                (
                    id,
                    XYResizerChange {
                        x: params.x,
                        y: params.y,
                        width: params.width,
                        height: params.height,
                        direction: [0.0, 0.0],
                    },
                )
            })
            .collect();
    }

    pub fn control_position(&self) -> Option<&str> {
        self.control_position.as_deref()
    }

    pub fn is_handle(control_position: &str) -> bool {
        is_handle_position(control_position)
    }

    pub fn is_line(control_position: &str) -> bool {
        is_line_position(control_position)
    }
}
