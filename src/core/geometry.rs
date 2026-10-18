#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Position {
    Left,
    Top,
    Right,
    Bottom,
}

impl Position {
    pub const ALL: [Position; 4] = [
        Position::Left,
        Position::Top,
        Position::Right,
        Position::Bottom,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Position::Left => "left",
            Position::Top => "top",
            Position::Right => "right",
            Position::Bottom => "bottom",
        }
    }

    pub fn from_str(value: &str) -> Option<Position> {
        match value {
            "left" => Some(Position::Left),
            "top" => Some(Position::Top),
            "right" => Some(Position::Right),
            "bottom" => Some(Position::Bottom),
            _ => None,
        }
    }

    pub fn is_horizontal(self) -> bool {
        matches!(self, Position::Left | Position::Right)
    }

    pub fn is_vertical(self) -> bool {
        matches!(self, Position::Top | Position::Bottom)
    }

    pub fn opposite(self) -> Position {
        match self {
            Position::Left => Position::Right,
            Position::Right => Position::Left,
            Position::Top => Position::Bottom,
            Position::Bottom => Position::Top,
        }
    }

    pub fn axis(self) -> Axis {
        match self {
            Position::Left | Position::Right => Axis::Horizontal,
            Position::Top | Position::Bottom => Axis::Vertical,
        }
    }

    pub fn direction(self) -> XYPosition {
        match self {
            Position::Left => XYPosition::new(-1.0, 0.0),
            Position::Right => XYPosition::new(1.0, 0.0),
            Position::Top => XYPosition::new(0.0, -1.0),
            Position::Bottom => XYPosition::new(0.0, 1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub fn other(self) -> Axis {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct XYPosition {
    pub x: f64,
    pub y: f64,
}

impl XYPosition {
    pub const ZERO: XYPosition = XYPosition { x: 0.0, y: 0.0 };

    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn add(self, other: XYPosition) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    pub fn sub(self, other: XYPosition) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    pub fn scale(self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }

    pub fn distance_to(self, other: XYPosition) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn is_zero(self) -> bool {
        self.x == 0.0 && self.y == 0.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct XYZPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Dimensions {
    pub width: f64,
    pub height: f64,
}

impl Dimensions {
    pub const ZERO: Dimensions = Dimensions {
        width: 0.0,
        height: 0.0,
    };

    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn area(self) -> f64 {
        self.width * self.height
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const ZERO: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn origin(self) -> XYPosition {
        XYPosition::new(self.x, self.y)
    }

    pub fn dimensions(self) -> Dimensions {
        Dimensions::new(self.width, self.height)
    }

    pub fn max_x(self) -> f64 {
        self.x + self.width
    }

    pub fn max_y(self) -> f64 {
        self.y + self.height
    }

    pub fn center(self) -> XYPosition {
        XYPosition::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn contains_point(self, point: XYPosition) -> bool {
        point.x >= self.x
            && point.x <= self.max_x()
            && point.y >= self.y
            && point.y <= self.max_y()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box2 {
    pub x: f64,
    pub y: f64,
    pub x2: f64,
    pub y2: f64,
}

impl Box2 {
    pub const EMPTY: Box2 = Box2 {
        x: f64::INFINITY,
        y: f64::INFINITY,
        x2: f64::NEG_INFINITY,
        y2: f64::NEG_INFINITY,
    };

    pub fn new(x: f64, y: f64, x2: f64, y2: f64) -> Self {
        Self { x, y, x2, y2 }
    }

    pub fn is_empty(self) -> bool {
        self.x > self.x2 || self.y > self.y2
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform(pub f64, pub f64, pub f64);

impl Default for Transform {
    fn default() -> Self {
        Transform::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Transform = Transform(0.0, 0.0, 1.0);

    pub fn new(x: f64, y: f64, zoom: f64) -> Self {
        Transform(x, y, zoom)
    }

    pub fn x(self) -> f64 {
        self.0
    }

    pub fn y(self) -> f64 {
        self.1
    }

    pub fn zoom(self) -> f64 {
        self.2
    }

    pub fn viewport(self) -> Viewport {
        Viewport {
            x: self.0,
            y: self.1,
            zoom: self.2,
        }
    }

    pub fn apply(self, point: XYPosition) -> XYPosition {
        XYPosition::new(point.x * self.2 + self.0, point.y * self.2 + self.1)
    }

    pub fn invert(self, point: XYPosition) -> XYPosition {
        let zoom = if self.2 == 0.0 { 1.0 } else { self.2 };
        XYPosition::new((point.x - self.0) / zoom, (point.y - self.1) / zoom)
    }

    pub fn css(self) -> String {
        format!("translate({}px,{}px) scale({})", self.0, self.1, self.2)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport::IDENTITY
    }
}

impl Viewport {
    pub const IDENTITY: Viewport = Viewport {
        x: 0.0,
        y: 0.0,
        zoom: 1.0,
    };

    pub fn new(x: f64, y: f64, zoom: f64) -> Self {
        Self { x, y, zoom }
    }

    pub fn transform(self) -> Transform {
        Transform(self.x, self.y, self.zoom)
    }
}

pub type CoordinateExtent = [[f64; 2]; 2];

pub type SnapGrid = [f64; 2];

pub type NodeOrigin = [f64; 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaddingUnit {
    Px,
    Percent,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaddingWithUnit {
    Number(f64),
    Px(f64),
    Percent(f64),
}

impl PaddingWithUnit {
    pub fn parse(value: &str) -> Option<PaddingWithUnit> {
        let trimmed = value.trim();
        if let Some(number) = trimmed.strip_suffix("px") {
            return number.trim().parse::<f64>().ok().map(PaddingWithUnit::Px);
        }
        if let Some(number) = trimmed.strip_suffix('%') {
            return number
                .trim()
                .parse::<f64>()
                .ok()
                .map(PaddingWithUnit::Percent);
        }
        trimmed.parse::<f64>().ok().map(PaddingWithUnit::Number)
    }

    pub fn unit(self) -> Option<PaddingUnit> {
        match self {
            PaddingWithUnit::Number(_) => None,
            PaddingWithUnit::Px(_) => Some(PaddingUnit::Px),
            PaddingWithUnit::Percent(_) => Some(PaddingUnit::Percent),
        }
    }
}

impl From<f64> for PaddingWithUnit {
    fn from(value: f64) -> Self {
        PaddingWithUnit::Number(value)
    }
}

impl From<i32> for PaddingWithUnit {
    fn from(value: i32) -> Self {
        PaddingWithUnit::Number(value as f64)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Padding {
    pub top: Option<PaddingWithUnit>,
    pub right: Option<PaddingWithUnit>,
    pub bottom: Option<PaddingWithUnit>,
    pub left: Option<PaddingWithUnit>,
    pub x: Option<PaddingWithUnit>,
    pub y: Option<PaddingWithUnit>,
}

impl Padding {
    pub fn all(value: impl Into<PaddingWithUnit>) -> Self {
        let value = value.into();
        Padding {
            top: Some(value),
            right: Some(value),
            bottom: Some(value),
            left: Some(value),
            x: None,
            y: None,
        }
    }
}

impl From<f64> for Padding {
    fn from(value: f64) -> Self {
        Padding::all(value)
    }
}

impl From<i32> for Padding {
    fn from(value: i32) -> Self {
        Padding::all(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelectionRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub start_x: f64,
    pub start_y: f64,
}

impl SelectionRect {
    pub fn as_rect(self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    Partial,
    #[default]
    Full,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanOnScrollMode {
    #[default]
    Free,
    Vertical,
    Horizontal,
}

impl PanOnScrollMode {
    pub fn as_str(self) -> &'static str {
        match self {
            PanOnScrollMode::Free => "free",
            PanOnScrollMode::Vertical => "vertical",
            PanOnScrollMode::Horizontal => "horizontal",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    CenterLeft,
    CenterRight,
}

impl Default for PanelPosition {
    fn default() -> Self {
        PanelPosition::BottomLeft
    }
}

impl PanelPosition {
    pub const ALL: [PanelPosition; 8] = [
        PanelPosition::TopLeft,
        PanelPosition::TopCenter,
        PanelPosition::TopRight,
        PanelPosition::BottomLeft,
        PanelPosition::BottomCenter,
        PanelPosition::BottomRight,
        PanelPosition::CenterLeft,
        PanelPosition::CenterRight,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            PanelPosition::TopLeft => "top-left",
            PanelPosition::TopCenter => "top-center",
            PanelPosition::TopRight => "top-right",
            PanelPosition::BottomLeft => "bottom-left",
            PanelPosition::BottomCenter => "bottom-center",
            PanelPosition::BottomRight => "bottom-right",
            PanelPosition::CenterLeft => "center-left",
            PanelPosition::CenterRight => "center-right",
        }
    }

    pub fn from_str(value: &str) -> Option<PanelPosition> {
        PanelPosition::ALL
            .into_iter()
            .find(|position| position.as_str() == value)
    }

    pub fn horizontal(self) -> Alignment {
        match self {
            PanelPosition::TopLeft | PanelPosition::BottomLeft | PanelPosition::CenterLeft => {
                Alignment::Start
            }
            PanelPosition::TopCenter | PanelPosition::BottomCenter => Alignment::Center,
            PanelPosition::TopRight | PanelPosition::BottomRight | PanelPosition::CenterRight => {
                Alignment::End
            }
        }
    }

    pub fn vertical(self) -> VerticalAlignment {
        match self {
            PanelPosition::TopLeft | PanelPosition::TopCenter | PanelPosition::TopRight => {
                VerticalAlignment::Start
            }
            PanelPosition::BottomLeft
            | PanelPosition::BottomCenter
            | PanelPosition::BottomRight => VerticalAlignment::End,
            PanelPosition::CenterLeft | PanelPosition::CenterRight => VerticalAlignment::Center,
        }
    }

    pub fn is_top(self) -> bool {
        matches!(self.vertical(), VerticalAlignment::Start)
    }

    pub fn is_bottom(self) -> bool {
        matches!(self.vertical(), VerticalAlignment::End)
    }

    pub fn is_left(self) -> bool {
        matches!(self.horizontal(), Alignment::Start)
    }

    pub fn is_right(self) -> bool {
        matches!(self.horizontal(), Alignment::End)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
}

impl Alignment {
    pub fn fraction(self) -> f64 {
        match self {
            Alignment::Start => 0.0,
            Alignment::Center => 0.5,
            Alignment::End => 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VerticalAlignment {
    Start,
    #[default]
    Center,
    End,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorMode {
    #[default]
    Light,
    Dark,
    System,
}

impl ColorMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ColorMode::Light => "light",
            ColorMode::Dark => "dark",
            ColorMode::System => "system",
        }
    }

    pub fn resolve(self, system_is_dark: bool) -> ResolvedColorMode {
        match self {
            ColorMode::Light => ResolvedColorMode::Light,
            ColorMode::Dark => ResolvedColorMode::Dark,
            ColorMode::System => {
                if system_is_dark {
                    ResolvedColorMode::Dark
                } else {
                    ResolvedColorMode::Light
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolvedColorMode {
    Light,
    Dark,
}

impl ResolvedColorMode {
    pub fn is_dark(self) -> bool {
        matches!(self, ResolvedColorMode::Dark)
    }

    pub fn class(self) -> &'static str {
        match self {
            ResolvedColorMode::Light => "light",
            ResolvedColorMode::Dark => "dark",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ZIndexMode {
    Auto,
    #[default]
    Basic,
    Manual,
}

impl ZIndexMode {
    pub fn is_manual(self) -> bool {
        matches!(self, ZIndexMode::Manual)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ZIndexMode::Auto => "auto",
            ZIndexMode::Basic => "basic",
            ZIndexMode::Manual => "manual",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum KeyCode {
    Single(String),
    Multiple(Vec<String>),
    #[default]
    None,
}

impl KeyCode {
    pub fn one(key: impl Into<String>) -> Self {
        KeyCode::Single(key.into())
    }

    pub fn many(keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        KeyCode::Multiple(keys.into_iter().map(Into::into).collect())
    }

    pub fn keys(&self) -> Vec<&str> {
        match self {
            KeyCode::Single(key) => vec![key.as_str()],
            KeyCode::Multiple(keys) => keys.iter().map(String::as_str).collect(),
            KeyCode::None => Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            KeyCode::None => true,
            KeyCode::Single(_) => false,
            KeyCode::Multiple(keys) => keys.is_empty(),
        }
    }

    pub fn matches(&self, pressed: &str) -> bool {
        self.keys().iter().any(|key| *key == pressed)
    }
}

impl From<&str> for KeyCode {
    fn from(value: &str) -> Self {
        KeyCode::Single(value.to_string())
    }
}

impl From<String> for KeyCode {
    fn from(value: String) -> Self {
        KeyCode::Single(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgePosition {
    pub source_x: f64,
    pub source_y: f64,
    pub target_x: f64,
    pub target_y: f64,
    pub source_position: Position,
    pub target_position: Position,
}

impl EdgePosition {
    pub fn new(
        source_x: f64,
        source_y: f64,
        target_x: f64,
        target_y: f64,
        source_position: Position,
        target_position: Position,
    ) -> Self {
        Self {
            source_x,
            source_y,
            target_x,
            target_y,
            source_position,
            target_position,
        }
    }
}
