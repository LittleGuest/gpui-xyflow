use crate::{
    core::geometry::{Transform, XYPosition},
    ui::theme::FlowTheme,
};

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
    pub gap: [f64; 2],
    pub size: f64,
    pub offset: [f64; 2],
    pub line_width: f64,
    pub dot_radius: f64,
    pub color: Option<gpui_kit::Rgba>,
    pub bg_color: Option<gpui_kit::Rgba>,
}

impl Default for BackgroundPattern {
    fn default() -> Self {
        BackgroundPattern {
            variant: BackgroundVariant::Dots,
            gap: [20.0, 20.0],
            size: 1.0,
            offset: [0.0, 0.0],
            line_width: 1.0,
            dot_radius: 1.0,
            color: None,
            bg_color: None,
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
        self.gap = [gap, gap];
        self
    }

    pub fn with_gap_xy(mut self, gap: [f64; 2]) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    pub fn with_offset(mut self, offset: f64) -> Self {
        self.offset = [offset, offset];
        self
    }

    pub fn with_offset_xy(mut self, offset: [f64; 2]) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_color(mut self, color: gpui_kit::Rgba) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_bg_color(mut self, color: gpui_kit::Rgba) -> Self {
        self.bg_color = Some(color);
        self
    }

    pub fn color(self, theme: &FlowTheme) -> gpui_kit::Rgba {
        self.color.unwrap_or(match self.variant {
            BackgroundVariant::Dots => theme.pattern_dots_color,
            BackgroundVariant::Lines => theme.pattern_lines_color,
            BackgroundVariant::Cross => theme.pattern_cross_color,
        })
    }

    pub fn background_color(self, theme: &FlowTheme) -> gpui_kit::Rgba {
        self.bg_color.unwrap_or(theme.background_color)
    }

    pub fn scaled_gap(self, transform: Transform) -> [f64; 2] {
        let zoom = transform.zoom();
        [(self.gap[0] * zoom).max(1.0), (self.gap[1] * zoom).max(1.0)]
    }

    pub fn dot_list(self, transform: Transform, width: f64, height: f64) -> Vec<(XYPosition, f64)> {
        let gap = self.scaled_gap(transform);
        if gap[0] <= 0.0 || gap[1] <= 0.0 {
            return Vec::new();
        }
        let zoom = transform.zoom();
        let offset_x = self.offset[0] * zoom;
        let offset_y = self.offset[1] * zoom;
        let start_x = (transform.x() + offset_x) % gap[0] - gap[0];
        let start_y = (transform.y() + offset_y) % gap[1] - gap[1];
        let radius = (self.dot_radius * zoom).max(0.6);
        let mut dots = Vec::new();
        let mut y = start_y;
        while y < height + gap[1] {
            let mut x = start_x;
            while x < width + gap[0] {
                dots.push((XYPosition::new(x, y), radius));
                x += gap[0];
            }
            y += gap[1];
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
        if gap[0] <= 0.0 || gap[1] <= 0.0 {
            return Vec::new();
        }
        let zoom = transform.zoom();
        let mut segments = Vec::new();
        let mut x = (transform.x() + self.offset[0] * zoom) % gap[0];
        while x < width + gap[0] {
            segments.push((XYPosition::new(x, 0.0), XYPosition::new(x, height)));
            x += gap[0];
        }
        let mut y = (transform.y() + self.offset[1] * zoom) % gap[1];
        while y < height + gap[1] {
            segments.push((XYPosition::new(0.0, y), XYPosition::new(width, y)));
            y += gap[1];
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
        if gap[0] <= 0.0 || gap[1] <= 0.0 {
            return Vec::new();
        }
        let zoom = transform.zoom();
        let arm = (self.size * 2.0 * zoom).max(1.5);
        let mut segments = Vec::new();
        let mut y = (transform.y() + self.offset[1] * zoom) % gap[1] - gap[1];
        while y < height + gap[1] {
            let mut x = (transform.x() + self.offset[0] * zoom) % gap[0] - gap[0];
            while x < width + gap[0] {
                segments.push((XYPosition::new(x - arm, y), XYPosition::new(x + arm, y)));
                segments.push((XYPosition::new(x, y - arm), XYPosition::new(x, y + arm)));
                x += gap[0];
            }
            y += gap[1];
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
