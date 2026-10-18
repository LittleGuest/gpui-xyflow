use crate::core::geometry::{PanelPosition, Rect};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelProps {
    pub position: PanelPosition,
    pub offset: f64,
    pub margin: f64,
}

impl Default for PanelProps {
    fn default() -> Self {
        PanelProps {
            position: PanelPosition::TopLeft,
            offset: 0.0,
            margin: 15.0,
        }
    }
}

impl PanelProps {
    pub fn new(position: PanelPosition) -> Self {
        PanelProps {
            position,
            ..PanelProps::default()
        }
    }

    pub fn with_offset(mut self, offset: f64) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_margin(mut self, margin: f64) -> Self {
        self.margin = margin;
        self
    }

    pub fn origin(
        self,
        container_width: f64,
        container_height: f64,
        width: f64,
        height: f64,
    ) -> (f64, f64) {
        let margin = self.margin;
        let offset = self.offset;
        match self.position {
            PanelPosition::TopLeft => (margin, margin + offset),
            PanelPosition::TopCenter => ((container_width - width) / 2.0, margin + offset),
            PanelPosition::TopRight => (container_width - width - margin, margin + offset),
            PanelPosition::CenterLeft => (margin, (container_height - height) / 2.0 + offset),
            PanelPosition::CenterRight => (
                container_width - width - margin,
                (container_height - height) / 2.0 + offset,
            ),
            PanelPosition::BottomLeft => (margin, container_height - height - margin - offset),
            PanelPosition::BottomCenter => (
                (container_width - width) / 2.0,
                container_height - height - margin - offset,
            ),
            PanelPosition::BottomRight => (
                container_width - width - margin,
                container_height - height - margin - offset,
            ),
        }
    }

    pub fn bounds(
        self,
        container_width: f64,
        container_height: f64,
        width: f64,
        height: f64,
    ) -> Rect {
        let (x, y) = self.origin(container_width, container_height, width, height);
        Rect::new(x, y, width, height)
    }
}

pub fn panel_anchor(position: PanelPosition) -> (f64, f64) {
    match position {
        PanelPosition::TopLeft => (0.0, 0.0),
        PanelPosition::TopCenter => (0.5, 0.0),
        PanelPosition::TopRight => (1.0, 0.0),
        PanelPosition::CenterLeft => (0.0, 0.5),
        PanelPosition::CenterRight => (1.0, 0.5),
        PanelPosition::BottomLeft => (0.0, 1.0),
        PanelPosition::BottomCenter => (0.5, 1.0),
        PanelPosition::BottomRight => (1.0, 1.0),
    }
}
