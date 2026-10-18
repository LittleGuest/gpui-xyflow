use gpui_kit::{Hsla, Pixels, Rgba, px, rgb, rgba};

#[derive(Clone, Debug, PartialEq)]
pub struct FlowTheme {
    pub edge_stroke: Rgba,
    pub edge_stroke_width: f32,
    pub edge_stroke_selected: Rgba,
    pub connection_line_stroke: Rgba,
    pub connection_line_stroke_width: f32,
    pub background_color: Rgba,
    pub pattern_dots_color: Rgba,
    pub pattern_lines_color: Rgba,
    pub pattern_cross_color: Rgba,
    pub node_color: Rgba,
    pub node_border: Rgba,
    pub node_border_selected: Rgba,
    pub node_background: Rgba,
    pub node_group_background: Rgba,
    pub node_border_radius: f32,
    pub node_padding: f32,
    pub node_width: f32,
    pub node_font_size: f32,
    pub handle_background: Rgba,
    pub handle_border: Rgba,
    pub handle_size: f32,
    pub selection_background: Rgba,
    pub selection_border: Rgba,
    pub controls_background: Rgba,
    pub controls_background_hover: Rgba,
    pub controls_foreground: Rgba,
    pub controls_border: Rgba,
    pub controls_button_size: f32,
    pub minimap_background: Rgba,
    pub minimap_mask_background: Rgba,
    pub minimap_mask_stroke: Rgba,
    pub minimap_mask_stroke_width: f32,
    pub minimap_node_background: Rgba,
    pub minimap_node_stroke: Rgba,
    pub minimap_node_stroke_width: f32,
    pub edge_label_background: Rgba,
    pub edge_label_color: Rgba,
    pub edge_label_font_size: f32,
    pub attribution_background: Rgba,
    pub attribution_color: Rgba,
    pub resize_background: Rgba,
    pub is_dark: bool,
}

impl FlowTheme {
    pub fn light() -> Self {
        FlowTheme {
            edge_stroke: rgb(0xb1b1b7),
            edge_stroke_width: 1.0,
            edge_stroke_selected: rgb(0x555555),
            connection_line_stroke: rgb(0xb1b1b7),
            connection_line_stroke_width: 1.0,
            background_color: rgba(0x00000000),
            pattern_dots_color: rgb(0x91919a),
            pattern_lines_color: rgb(0xeeeeee),
            pattern_cross_color: rgb(0xe2e2e2),
            node_color: rgb(0x1a192b),
            node_border: rgb(0xbbbbbb),
            node_border_selected: rgb(0x555555),
            node_background: rgb(0xffffff),
            node_group_background: rgba(0xf0f0f040),
            node_border_radius: 0.0,
            node_padding: 10.0,
            node_width: 150.0,
            node_font_size: 12.0,
            handle_background: rgb(0x333333),
            handle_border: rgb(0xffffff),
            handle_size: 6.0,
            selection_background: rgba(0x9696b41a),
            selection_border: rgba(0x9b9b9bcc),
            controls_background: rgb(0xfefefe),
            controls_background_hover: rgb(0xf4f4f4),
            controls_foreground: rgb(0x333333),
            controls_border: rgb(0xeeeeee),
            controls_button_size: 26.0,
            minimap_background: rgb(0xffffff),
            minimap_mask_background: rgba(0xf0f0f099),
            minimap_mask_stroke: rgba(0x00000000),
            minimap_mask_stroke_width: 1.0,
            minimap_node_background: rgb(0xe2e2e2),
            minimap_node_stroke: rgba(0x00000000),
            minimap_node_stroke_width: 2.0,
            edge_label_background: rgb(0xffffff),
            edge_label_color: rgb(0x333333),
            edge_label_font_size: 10.0,
            attribution_background: rgba(0xffffff80),
            attribution_color: rgb(0x999999),
            resize_background: rgb(0x3367d9),
            is_dark: false,
        }
    }

    pub fn dark() -> Self {
        FlowTheme {
            edge_stroke: rgb(0x3e3e3e),
            edge_stroke_width: 1.0,
            edge_stroke_selected: rgb(0x727272),
            connection_line_stroke: rgb(0xb1b1b7),
            connection_line_stroke_width: 1.0,
            background_color: rgb(0x141414),
            pattern_dots_color: rgb(0x555555),
            pattern_lines_color: rgb(0x333333),
            pattern_cross_color: rgb(0x333333),
            node_color: rgb(0xf8f8f8),
            node_border: rgb(0xbbbbbb),
            node_border_selected: rgb(0x555555),
            node_background: rgb(0x1e1e1e),
            node_group_background: rgba(0xf0f0f040),
            node_border_radius: 0.0,
            node_padding: 10.0,
            node_width: 150.0,
            node_font_size: 12.0,
            handle_background: rgb(0x333333),
            handle_border: rgb(0x1e1e1e),
            handle_size: 6.0,
            selection_background: rgba(0x9696b41a),
            selection_border: rgba(0x9b9b9bcc),
            controls_background: rgb(0x2b2b2b),
            controls_background_hover: rgb(0x3e3e3e),
            controls_foreground: rgb(0xf8f8f8),
            controls_border: rgb(0x5b5b5b),
            controls_button_size: 26.0,
            minimap_background: rgb(0x141414),
            minimap_mask_background: rgba(0x3c3c3c99),
            minimap_mask_stroke: rgba(0x00000000),
            minimap_mask_stroke_width: 1.0,
            minimap_node_background: rgb(0x2b2b2b),
            minimap_node_stroke: rgba(0x00000000),
            minimap_node_stroke_width: 2.0,
            edge_label_background: rgb(0x141414),
            edge_label_color: rgb(0xf8f8f8),
            edge_label_font_size: 10.0,
            attribution_background: rgba(0x96969640),
            attribution_color: rgb(0x999999),
            resize_background: rgb(0x3367d9),
            is_dark: true,
        }
    }

    pub fn of(is_dark: bool) -> Self {
        if is_dark {
            FlowTheme::dark()
        } else {
            FlowTheme::light()
        }
    }

    pub fn edge_stroke_for(&self, selected: bool) -> Rgba {
        if selected {
            self.edge_stroke_selected
        } else {
            self.edge_stroke
        }
    }

    pub fn node_box_shadow_selected(&self) -> Hsla {
        if self.is_dark {
            rgb(0x999999).into()
        } else {
            rgb(0x1a192b).into()
        }
    }

    pub fn handle_radius(&self) -> Pixels {
        px(self.handle_size / 2.0)
    }
}

impl Default for FlowTheme {
    fn default() -> Self {
        FlowTheme::light()
    }
}
