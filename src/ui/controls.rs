use crate::core::geometry::PanelPosition;
use crate::ui::theme::FlowTheme;

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

#[derive(Clone, Debug, PartialEq)]
pub struct ControlsProps {
    pub position: PanelPosition,
    pub show_zoom: bool,
    pub show_fit_view: bool,
    pub show_interactive: bool,
    pub orientation: ControlsOrientation,
    pub button_size: f32,
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
        match self.orientation {
            ControlsOrientation::Vertical => self.button_size * self.buttons().len() as f32,
            ControlsOrientation::Horizontal => self.button_size,
        }
    }

    pub fn width(&self) -> f32 {
        match self.orientation {
            ControlsOrientation::Vertical => self.button_size,
            ControlsOrientation::Horizontal => self.button_size * self.buttons().len() as f32,
        }
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

pub fn controls_button_at(props: &ControlsProps, local_x: f32, local_y: f32) -> Option<ControlButton> {
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
