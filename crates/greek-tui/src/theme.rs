// Theme management for the TUI

use greek_common::Theme;
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;

pub struct TuiTheme {
    pub theme: Theme,
    pub colors: HashMap<String, Color>,
    pub styles: HashMap<String, Style>,
}

impl TuiTheme {
    pub fn from_theme(theme: Theme) -> Self {
        let mut colors = HashMap::new();
        let mut styles = HashMap::new();

        let background = Self::parse_color(&theme.background);
        let foreground = Self::parse_color(&theme.foreground);
        let accent = Self::parse_color(&theme.accent);
        let success = Self::parse_color(&theme.success);
        let warning = Self::parse_color(&theme.warning);
        let danger = Self::parse_color(&theme.danger);
        let muted = Self::parse_color(&theme.muted);
        let selection_bg = Self::parse_color(&theme.selection_bg);
        let selection_fg = Self::parse_color(&theme.selection_fg);
        let border = Self::parse_color(&theme.border);

        colors.insert("background".to_string(), background);
        colors.insert("foreground".to_string(), foreground);
        colors.insert("accent".to_string(), accent);
        colors.insert("success".to_string(), success);
        colors.insert("warning".to_string(), warning);
        colors.insert("danger".to_string(), danger);
        colors.insert("muted".to_string(), muted);
        colors.insert("selection_bg".to_string(), selection_bg);
        colors.insert("selection_fg".to_string(), selection_fg);
        colors.insert("border".to_string(), border);

        styles.insert(
            "default".to_string(),
            Style::default().fg(foreground).bg(background),
        );
        styles.insert(
            "accent".to_string(),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        );
        styles.insert("success".to_string(), Style::default().fg(success));
        styles.insert("warning".to_string(), Style::default().fg(warning));
        styles.insert("danger".to_string(), Style::default().fg(danger));
        styles.insert("muted".to_string(), Style::default().fg(muted));
        styles.insert(
            "selected".to_string(),
            Style::default()
                .fg(selection_fg)
                .bg(selection_bg)
                .add_modifier(Modifier::BOLD),
        );
        styles.insert("border".to_string(), Style::default().fg(border));
        styles.insert(
            "title".to_string(),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        );
        styles.insert(
            "highlight".to_string(),
            Style::default()
                .fg(Color::White)
                .bg(selection_bg)
                .add_modifier(Modifier::BOLD),
        );
        styles.insert(
            "row_even".to_string(),
            Style::default()
                .fg(foreground)
                .bg(Color::Rgb(245, 247, 250)),
        );
        styles.insert(
            "row_odd".to_string(),
            Style::default().fg(foreground).bg(background),
        );
        styles.insert(
            "header_label".to_string(),
            Style::default()
                .fg(Color::Rgb(100, 100, 100))
                .bg(Color::Rgb(235, 238, 242)),
        );
        styles.insert(
            "header_value".to_string(),
            Style::default()
                .fg(foreground)
                .bg(Color::Rgb(235, 238, 242))
                .add_modifier(Modifier::BOLD),
        );
        styles.insert(
            "status_bar".to_string(),
            Style::default()
                .fg(Color::Rgb(80, 80, 80))
                .bg(Color::Rgb(230, 233, 240)),
        );
        styles.insert(
            "popup_bg".to_string(),
            Style::default()
                .fg(foreground)
                .bg(Color::Rgb(255, 255, 255)),
        );
        styles.insert(
            "popup_border".to_string(),
            Style::default().fg(Color::Rgb(180, 190, 205)),
        );
        styles.insert(
            "popup_item".to_string(),
            Style::default()
                .fg(Color::Rgb(50, 50, 50))
                .bg(Color::Rgb(255, 255, 255)),
        );
        styles.insert(
            "popup_item_hover".to_string(),
            Style::default().fg(Color::White).bg(accent),
        );
        styles.insert(
            "gradient_top".to_string(),
            Style::default()
                .fg(foreground)
                .bg(Color::Rgb(232, 240, 254)),
        );
        styles.insert(
            "gradient_mid".to_string(),
            Style::default()
                .fg(foreground)
                .bg(Color::Rgb(240, 244, 252)),
        );
        styles.insert(
            "gradient_bot".to_string(),
            Style::default()
                .fg(foreground)
                .bg(Color::Rgb(248, 250, 253)),
        );
        styles.insert(
            "table_header".to_string(),
            Style::default()
                .fg(Color::Rgb(60, 70, 90))
                .bg(Color::Rgb(225, 230, 240))
                .add_modifier(Modifier::BOLD),
        );

        Self {
            theme,
            colors,
            styles,
        }
    }

    pub fn get_style(&self, name: &str) -> Style {
        self.styles
            .get(name)
            .copied()
            .unwrap_or_else(Style::default)
    }

    pub fn get_color(&self, name: &str) -> Color {
        self.colors.get(name).copied().unwrap_or(Color::Reset)
    }

    fn parse_color(hex: &str) -> Color {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            Color::Rgb(r, g, b)
        } else {
            match hex.to_lowercase().as_str() {
                "white" => Color::White,
                "black" => Color::Black,
                "gray" | "grey" => Color::Gray,
                _ => Color::Reset,
            }
        }
    }

    pub fn light() -> Self {
        Self::from_theme(Theme {
            name: "reek-light".to_string(),
            background: "#FFFFFF".to_string(),
            foreground: "#2C3E50".to_string(),
            accent: "#3B82F6".to_string(),
            success: "#10B981".to_string(),
            warning: "#F59E0B".to_string(),
            danger: "#EF4444".to_string(),
            muted: "#9CA3AF".to_string(),
            selection_bg: "#3B82F6".to_string(),
            selection_fg: "#FFFFFF".to_string(),
            border: "#D1D5DB".to_string(),
        })
    }
}

impl Default for TuiTheme {
    fn default() -> Self {
        Self::light()
    }
}
