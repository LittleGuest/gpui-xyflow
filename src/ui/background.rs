use crate::core::geometry::{Transform, XYPosition};
use crate::ui::theme::FlowTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundVariant {
    Dots,
    Lines,
    Cross,
}

impl BackgroundVariant {
    pub fn from_str(value: &str) -> BackgroundVariant {
        match value {
            "lines" => BackgroundVariant::Lines,
            "cross" => BackgroundVariant::Cross,
            _ => BackgroundVariant::Dots,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BackgroundVariant::Dots => "dots",
            BackgroundVariant::Lines => "lines",
            BackgroundVariant::Cross => "cross",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackgroundPattern {
    pub variant: BackgroundVariant,
    pub gap: f64,
    pub size: f64,
    pub offset: f64,
    pub line_width: f64,
    pub dot_radius: f64,
}

impl Default for BackgroundPattern {
    fn default() -> Self {
        BackgroundPattern {
            variant: BackgroundVariant::Dots,
            gap: 20.0,
            size: 1.0,
            offset: 0.0,
            line_width: 1.0,
            dot_radius: 1.0,
        }
    }
}

impl BackgroundPattern {
    pub fn new(variant: BackgroundVariant) -> Self {
        BackgroundPattern {
            variant,
            ..BackgroundPattern::default()
        }
    }

    pub fn with_gap(mut self, gap: f64) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    pub fn with_offset(mut self, offset: f64) -> Self {
        self.offset = offset;
        self
    }

    pub fn color(self, theme: &FlowTheme) -> gpui_kit::Rgba {
        match self.variant {
            BackgroundVariant::Dots => theme.pattern_dots_color,
            BackgroundVariant::Lines => theme.pattern_lines_color,
            BackgroundVariant::Cross => theme.pattern_cross_color,
        }
    }

    pub fn scaled_gap(self, transform: Transform) -> f64 {
        let zoom = transform.zoom();
        let gap = self.gap * zoom;
        if gap < 1.0 {
            1.0
        } else {
            gap
        }
    }

    pub fn dot_list(
        self,
        transform: Transform,
        width: f64,
        height: f64,
    ) -> Vec<(XYPosition, f64)> {
        let gap = self.scaled_gap(transform);
        if gap <= 0.0 {
            return Vec::new();
        }
        let zoom = transform.zoom();
        let start_x = transform.x() % gap - gap;
        let start_y = transform.y() % gap - gap;
        let radius = (self.dot_radius * zoom).max(0.6);
        let mut dots = Vec::new();
        let mut y = start_y;
        while y < height + gap {
            let mut x = start_x;
            while x < width + gap {
                dots.push((XYPosition::new(x, y), radius));
                x += gap;
            }
            y += gap;
        }
        dots
    }

    pub fn line_segments(
        self,
        transform: Transform,
        width: f64,
        height: f64,
    ) -> Vec<(XYPosition, XYPosition)> {
        let gap = self.scaled_gap(transform);
        if gap <= 0.0 {
            return Vec::new();
        }
        let mut segments = Vec::new();
        let mut x = transform.x() % gap;
        while x < width + gap {
            segments.push((
                XYPosition::new(x, 0.0),
                XYPosition::new(x, height),
            ));
            x += gap;
        }
        let mut y = transform.y() % gap;
        while y < height + gap {
            segments.push((
                XYPosition::new(0.0, y),
                XYPosition::new(width, y),
            ));
            y += gap;
        }
        segments
    }

    pub fn cross_segments(
        self,
        transform: Transform,
        width: f64,
        height: f64,
    ) -> Vec<(XYPosition, XYPosition)> {
        let gap = self.scaled_gap(transform);
        if gap <= 0.0 {
            return Vec::new();
        }
        let zoom = transform.zoom();
        let arm = (self.size * 2.0 * zoom).max(1.5);
        let mut segments = Vec::new();
        let mut y = transform.y() % gap - gap;
        while y < height + gap {
            let mut x = transform.x() % gap - gap;
            while x < width + gap {
                segments.push((XYPosition::new(x - arm, y), XYPosition::new(x + arm, y)));
                segments.push((XYPosition::new(x, y - arm), XYPosition::new(x, y + arm)));
                x += gap;
            }
            y += gap;
        }
        segments
    }

    pub fn primitive_count(self, transform: Transform, width: f64, height: f64) -> usize {
        match self.variant {
            BackgroundVariant::Dots => self.dot_list(transform, width, height).len(),
            BackgroundVariant::Lines => self.line_segments(transform, width, height).len(),
            BackgroundVariant::Cross => self.cross_segments(transform, width, height).len(),
        }
    }
}

pub fn background_color(theme: &FlowTheme) -> gpui_kit::Rgba {
    theme.background_color
}
