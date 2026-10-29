use std::sync::Arc;

use gpui_kit::{AnyElement, App, Window};

use crate::{core::geometry::PanelPosition, ui::theme::FlowTheme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlButton {
    ZoomIn,
    ZoomOut,
    FitView,
    Interactive,
}

impl ControlButton {
    pub fn label(self) -> &'static str {
        match self {
            ControlButton::ZoomIn => "+",
            ControlButton::ZoomOut => "-",
            ControlButton::FitView => "F",
            ControlButton::Interactive => "L",
        }
    }

    pub fn aria_label(self) -> &'static str {
        match self {
            ControlButton::ZoomIn => "zoom in",
            ControlButton::ZoomOut => "zoom out",
            ControlButton::FitView => "fit view",
            ControlButton::Interactive => "toggle interactivity",
        }
    }

    pub fn all() -> [ControlButton; 4] {
        [
            ControlButton::ZoomIn,
            ControlButton::ZoomOut,
            ControlButton::FitView,
            ControlButton::Interactive,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitViewOptions {
    pub padding: f64,
    pub min_zoom: Option<f64>,
    pub max_zoom: Option<f64>,
    pub include_hidden_nodes: bool,
}

impl Default for FitViewOptions {
    fn default() -> Self {
        FitViewOptions {
            padding: 0.1,
            min_zoom: None,
            max_zoom: None,
            include_hidden_nodes: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlsProps {
    pub position: PanelPosition,
    pub show_zoom: bool,
    pub show_fit_view: bool,
    pub show_interactive: bool,
    pub orientation: ControlsOrientation,
    pub button_size: f32,
    pub aria_label: Option<String>,
    pub custom_items: Vec<CustomControlItem>,
    pub button_contents: Vec<ButtonContentItem>,
    pub callbacks: ControlsCallbacks,
    pub fit_view_options: FitViewOptions,
}

/// Builds extra content appended after the built-in control buttons.
pub type ControlContent = Arc<dyn Fn(&mut Window, &mut App) -> AnyElement + Send + Sync>;

#[derive(Clone)]
pub struct CustomControlItem {
    pub content: ControlContent,
}

impl std::fmt::Debug for CustomControlItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomControlItem").finish_non_exhaustive()
    }
}

impl PartialEq for CustomControlItem {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.content, &other.content)
    }
}

/// Replaces the visible content of a built-in control button.
#[derive(Clone)]
pub struct ButtonContentItem {
    pub button: ControlButton,
    pub content: ControlContent,
}

impl std::fmt::Debug for ButtonContentItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ButtonContentItem")
            .field("button", &self.button)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ButtonContentItem {
    fn eq(&self, other: &Self) -> bool {
        self.button == other.button && Arc::ptr_eq(&self.content, &other.content)
    }
}

pub type UnitControlHandler = Arc<dyn Fn() + Send + Sync>;
pub type InteractiveControlHandler = Arc<dyn Fn(bool) + Send + Sync>;

#[derive(Clone, Default)]
pub struct ControlsCallbacks {
    pub on_zoom_in: Option<UnitControlHandler>,
    pub on_zoom_out: Option<UnitControlHandler>,
    pub on_fit_view: Option<UnitControlHandler>,
    pub on_interactive_change: Option<InteractiveControlHandler>,
}

impl std::fmt::Debug for ControlsCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlsCallbacks").finish_non_exhaustive()
    }
}

impl PartialEq for ControlsCallbacks {
    fn eq(&self, other: &Self) -> bool {
        option_arc_eq(&self.on_zoom_in, &other.on_zoom_in)
            && option_arc_eq(&self.on_zoom_out, &other.on_zoom_out)
            && option_arc_eq(&self.on_fit_view, &other.on_fit_view)
            && match (&self.on_interactive_change, &other.on_interactive_change) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
    }
}

fn option_arc_eq<T: ?Sized>(left: &Option<Arc<T>>, right: &Option<Arc<T>>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => Arc::ptr_eq(left, right),
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlsOrientation {
    Vertical,
    Horizontal,
}

impl Default for ControlsProps {
    fn default() -> Self {
        ControlsProps {
            position: PanelPosition::BottomLeft,
            show_zoom: true,
            show_fit_view: true,
            show_interactive: true,
            orientation: ControlsOrientation::Vertical,
            button_size: 26.0,
            aria_label: None,
            custom_items: Vec::new(),
            button_contents: Vec::new(),
            callbacks: ControlsCallbacks::default(),
            fit_view_options: FitViewOptions::default(),
        }
    }
}

impl ControlsProps {
    pub fn buttons(&self) -> Vec<ControlButton> {
        let mut buttons = Vec::new();
        if self.show_zoom {
            buttons.push(ControlButton::ZoomIn);
            buttons.push(ControlButton::ZoomOut);
        }
        if self.show_fit_view {
            buttons.push(ControlButton::FitView);
        }
        if self.show_interactive {
            buttons.push(ControlButton::Interactive);
        }
        buttons
    }

    pub fn height(&self) -> f32 {
        let count = self.item_count();
        match self.orientation {
            ControlsOrientation::Vertical => self.button_size * count as f32,
            ControlsOrientation::Horizontal => self.button_size,
        }
    }

    pub fn width(&self) -> f32 {
        let count = self.item_count();
        match self.orientation {
            ControlsOrientation::Vertical => self.button_size,
            ControlsOrientation::Horizontal => self.button_size * count as f32,
        }
    }

    pub fn item_count(&self) -> usize {
        self.buttons().len() + self.custom_items.len()
    }

    pub fn with_aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    pub fn with_custom_item(mut self, content: ControlContent) -> Self {
        self.custom_items.push(CustomControlItem { content });
        self
    }

    pub fn with_fit_view_options(mut self, options: FitViewOptions) -> Self {
        self.fit_view_options = options;
        self
    }

    pub fn with_button_content(mut self, button: ControlButton, content: ControlContent) -> Self {
        self.button_contents
            .push(ButtonContentItem { button, content });
        self
    }

    pub fn button_content(&self, button: ControlButton) -> Option<&ControlContent> {
        self.button_contents
            .iter()
            .find(|item| item.button == button)
            .map(|item| &item.content)
    }
}

pub fn controls_background(theme: &FlowTheme) -> gpui_kit::Rgba {
    theme.controls_background
}

pub fn controls_foreground(theme: &FlowTheme) -> gpui_kit::Rgba {
    theme.controls_foreground
}

pub fn controls_border(theme: &FlowTheme) -> gpui_kit::Rgba {
    theme.controls_border
}

pub fn controls_button_at(
    props: &ControlsProps,
    local_x: f32,
    local_y: f32,
) -> Option<ControlButton> {
    let buttons = props.buttons();
    if buttons.is_empty() {
        return None;
    }
    let size = props.button_size;
    let index = match props.orientation {
        ControlsOrientation::Vertical => {
            if local_x < 0.0 || local_x > size {
                return None;
            }
            (local_y / size).floor() as usize
        }
        ControlsOrientation::Horizontal => {
            if local_y < 0.0 || local_y > size {
                return None;
            }
            (local_x / size).floor() as usize
        }
    };
    buttons.get(index).copied()
}

pub fn zoom_label(zoom: f64) -> String {
    format!("{}%", (zoom * 100.0).round() as i64)
}
